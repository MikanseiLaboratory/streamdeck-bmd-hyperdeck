use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use hyperdeck_tokio::core::RawMessage;
use hyperdeck_tokio::core::ethernet::encode_command;
use hyperdeck_tokio::core::session::{ClientEvent, ClientSession};
use hyperdeck_tokio::core::types::CodeClass;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::{Mutex, broadcast, mpsc, oneshot};
use tokio::time::{Instant, sleep, timeout};
use tokio_util::sync::CancellationToken;

use crate::config::{HyperDeckInstanceConfig, TargetGroup};
use crate::message::DeckMessage;
use crate::snapshot::DeckSnapshot;
use crate::status::{ConnectionStatus, InstanceStatus};

const BUF: usize = 16384;

/// How a supervisor waits between attempts.
#[derive(Clone, Debug)]
pub struct PoolOptions {
    pub connect_timeout: Duration,
    pub command_timeout: Duration,
    pub initial_backoff: Duration,
    pub max_backoff: Duration,
    pub timecode_interval: Duration,
}

impl Default for PoolOptions {
    fn default() -> Self {
        Self {
            connect_timeout: Duration::from_secs(3),
            command_timeout: Duration::from_secs(3),
            initial_backoff: Duration::from_secs(1),
            max_backoff: Duration::from_secs(30),
            timecode_interval: Duration::from_millis(100),
        }
    }
}

impl PoolOptions {
    /// Short timeouts for loopback tests.
    pub fn for_tests() -> Self {
        Self {
            connect_timeout: Duration::from_secs(2),
            command_timeout: Duration::from_secs(2),
            initial_backoff: Duration::from_millis(20),
            max_backoff: Duration::from_millis(50),
            timecode_interval: Duration::from_millis(20),
        }
    }
}

/// Something the pool wants the plugin to repaint.
#[derive(Clone, Debug)]
pub enum PoolEvent {
    /// Connection label changed.
    Status { id: String },
    /// Cached deck state changed.
    Updated { id: String },
}

/// Failure while sending one command.
#[derive(Debug, thiserror::Error)]
pub enum CallError {
    #[error("hyperdeck {id} is not connected")]
    Disconnected { id: String },
    #[error("hyperdeck command timed out")]
    Timeout,
    #[error("hyperdeck closed the connection")]
    Closed,
    #[error("hyperdeck returned {code} {text}")]
    Device { code: u16, text: String },
    #[error("{0}")]
    Protocol(String),
}

struct Call {
    bytes: Vec<u8>,
    reply: oneshot::Sender<Result<DeckMessage, CallError>>,
}

struct Slot {
    config: HyperDeckInstanceConfig,
    snapshot: Arc<Mutex<DeckSnapshot>>,
    commands: mpsc::Sender<Call>,
    cancel: CancellationToken,
    status: ConnectionStatus,
}

struct Inner {
    slots: HashMap<String, Slot>,
    groups: Vec<TargetGroup>,
    configs: Vec<HyperDeckInstanceConfig>,
}

/// Shared pool of HyperDeck sessions.
#[derive(Clone)]
pub struct HyperDeckPool {
    inner: Arc<Mutex<Inner>>,
    events: broadcast::Sender<PoolEvent>,
    options: PoolOptions,
}

impl HyperDeckPool {
    /// Empty pool. Supervisors start from [`Self::reconcile`].
    pub fn new(options: PoolOptions) -> Self {
        let (events, _) = broadcast::channel(256);
        Self {
            inner: Arc::new(Mutex::new(Inner {
                slots: HashMap::new(),
                groups: Vec::new(),
                configs: Vec::new(),
            })),
            events,
            options,
        }
    }

    /// Stream of status and snapshot updates.
    pub fn subscribe(&self) -> broadcast::Receiver<PoolEvent> {
        self.events.subscribe()
    }

    /// Start, stop, and restart supervisors so they match `configs`.
    pub async fn reconcile(&self, configs: Vec<HyperDeckInstanceConfig>, groups: Vec<TargetGroup>) {
        let mut inner = self.inner.lock().await;
        let wanted: HashMap<String, HyperDeckInstanceConfig> = configs
            .iter()
            .cloned()
            .map(|config| (config.id.clone(), config))
            .collect();
        let stale: Vec<String> = inner
            .slots
            .keys()
            .filter(|id| !wanted.contains_key(*id))
            .cloned()
            .collect();
        for id in stale {
            if let Some(slot) = inner.slots.remove(&id) {
                slot.cancel.cancel();
            }
        }
        for config in &configs {
            let restart = match inner.slots.get(&config.id) {
                None => true,
                Some(slot) => slot.config != *config,
            };
            if !restart {
                continue;
            }
            if let Some(slot) = inner.slots.remove(&config.id) {
                slot.cancel.cancel();
            }
            if !config.enabled {
                inner
                    .slots
                    .insert(config.id.clone(), disabled_slot(config.clone()));
                let _ = self.events.send(PoolEvent::Status {
                    id: config.id.clone(),
                });
                continue;
            }
            inner
                .slots
                .insert(config.id.clone(), self.spawn(config.clone()));
        }
        inner.groups = groups;
        inner.configs = configs;
    }

    /// Drop the current session and connect again.
    pub async fn reconnect(&self, id: &str) {
        let config = {
            let inner = self.inner.lock().await;
            inner.slots.get(id).map(|slot| slot.config.clone())
        };
        let Some(config) = config else {
            return;
        };
        if !config.enabled {
            return;
        }
        let mut inner = self.inner.lock().await;
        if let Some(slot) = inner.slots.remove(id) {
            slot.cancel.cancel();
        }
        inner.slots.insert(config.id.clone(), self.spawn(config));
    }

    /// Latest cached state.
    pub async fn snapshot(&self, id: &str) -> Option<DeckSnapshot> {
        let inner = self.inner.lock().await;
        let slot = inner.slots.get(id)?;
        Some(slot.snapshot.lock().await.clone())
    }

    /// Connection label for one deck.
    pub async fn status(&self, id: &str) -> Option<ConnectionStatus> {
        let inner = self.inner.lock().await;
        inner.slots.get(id).map(|slot| slot.status.clone())
    }

    /// Status of every configured deck.
    pub async fn statuses(&self) -> Vec<InstanceStatus> {
        let inner = self.inner.lock().await;
        inner
            .configs
            .iter()
            .map(|config| {
                let slot = inner.slots.get(&config.id);
                InstanceStatus {
                    id: config.id.clone(),
                    name: config.name.clone(),
                    status: slot.map(|slot| slot.status.clone()).unwrap_or(
                        ConnectionStatus::Disconnected {
                            message: "missing".into(),
                        },
                    ),
                    model: String::new(),
                }
            })
            .collect()
    }

    /// Enabled ids for the current groups. The plugin resolves selectors itself
    /// when it has the settings; this exposes the stored groups.
    pub async fn groups(&self) -> Vec<TargetGroup> {
        self.inner.lock().await.groups.clone()
    }

    /// Configured instances, including disabled ones.
    pub async fn configs(&self) -> Vec<HyperDeckInstanceConfig> {
        self.inner.lock().await.configs.clone()
    }

    /// Send one already encoded command, including its trailing newline.
    pub async fn call_raw(&self, id: &str, bytes: &[u8]) -> Result<DeckMessage, CallError> {
        let sender = {
            let inner = self.inner.lock().await;
            inner.slots.get(id).map(|slot| slot.commands.clone())
        };
        let Some(sender) = sender else {
            return Err(CallError::Disconnected { id: id.to_string() });
        };
        let (reply, response) = oneshot::channel();
        sender
            .send(Call {
                bytes: bytes.to_vec(),
                reply,
            })
            .await
            .map_err(|_| CallError::Disconnected { id: id.to_string() })?;
        response.await.map_err(|_| CallError::Closed)?
    }

    /// Encode `name` plus parameters and send them.
    pub async fn call(
        &self,
        id: &str,
        name: &str,
        params: &[(&str, &str)],
    ) -> Result<DeckMessage, CallError> {
        let mut line = String::new();
        encode_command(&mut line, name, params)
            .map_err(|_| CallError::Protocol("command too large".into()))?;
        self.call_raw(id, line.as_bytes()).await
    }

    fn spawn(&self, config: HyperDeckInstanceConfig) -> Slot {
        let (commands, rx) = mpsc::channel(32);
        let snapshot = Arc::new(Mutex::new(DeckSnapshot::default()));
        let cancel = CancellationToken::new();
        let events = self.events.clone();
        let options = self.options.clone();
        let task_snapshot = Arc::clone(&snapshot);
        let task_cancel = cancel.clone();
        let task_config = config.clone();
        let pool = self.clone();
        tokio::spawn(async move {
            supervise(
                task_config,
                rx,
                task_snapshot,
                task_cancel,
                events,
                options,
                pool,
            )
            .await;
        });
        Slot {
            config,
            snapshot,
            commands,
            cancel,
            status: ConnectionStatus::Connecting,
        }
    }
}

fn disabled_slot(config: HyperDeckInstanceConfig) -> Slot {
    let (commands, _rx) = mpsc::channel(1);
    Slot {
        config,
        snapshot: Arc::new(Mutex::new(DeckSnapshot::default())),
        commands,
        cancel: CancellationToken::new(),
        status: ConnectionStatus::Disabled,
    }
}

async fn set_status(pool: &HyperDeckPool, id: &str, status: ConnectionStatus) {
    let mut inner = pool.inner.lock().await;
    if let Some(slot) = inner.slots.get_mut(id) {
        slot.status = status;
    }
    drop(inner);
    let _ = pool.events.send(PoolEvent::Status { id: id.to_string() });
}

async fn supervise(
    config: HyperDeckInstanceConfig,
    mut commands: mpsc::Receiver<Call>,
    snapshot: Arc<Mutex<DeckSnapshot>>,
    cancel: CancellationToken,
    events: broadcast::Sender<PoolEvent>,
    options: PoolOptions,
    pool: HyperDeckPool,
) {
    let mut backoff = options.initial_backoff;
    loop {
        if cancel.is_cancelled() {
            break;
        }
        set_status(&pool, &config.id, ConnectionStatus::Connecting).await;
        match run_session(
            &config,
            &mut commands,
            &snapshot,
            &cancel,
            &events,
            &options,
            &pool,
        )
        .await
        {
            Ok(()) => break,
            Err(error) => {
                tracing::debug!(id = %config.id, %error, "hyperdeck session ended");
                {
                    let mut guard = snapshot.lock().await;
                    guard.connected = false;
                }
                set_status(
                    &pool,
                    &config.id,
                    ConnectionStatus::Disconnected {
                        message: error.to_string(),
                    },
                )
                .await;
            }
        }
        tokio::select! {
            _ = cancel.cancelled() => break,
            _ = sleep(backoff) => {}
        }
        backoff = (backoff * 2).min(options.max_backoff);
    }
}

async fn run_session(
    config: &HyperDeckInstanceConfig,
    commands: &mut mpsc::Receiver<Call>,
    snapshot: &Arc<Mutex<DeckSnapshot>>,
    cancel: &CancellationToken,
    events: &broadcast::Sender<PoolEvent>,
    options: &PoolOptions,
    pool: &HyperDeckPool,
) -> Result<(), CallError> {
    let stream = tokio::select! {
        _ = cancel.cancelled() => return Ok(()),
        result = timeout(options.connect_timeout, TcpStream::connect(config.socket_addr())) => {
            result.map_err(|_| CallError::Timeout)?.map_err(|error| CallError::Protocol(error.to_string()))?
        }
    };
    stream
        .set_nodelay(true)
        .map_err(|error| CallError::Protocol(error.to_string()))?;
    let mut link = Link {
        stream,
        session: ClientSession::<BUF>::new(),
        snapshot: Arc::clone(snapshot),
        events: events.clone(),
        id: config.id.clone(),
        options: options.clone(),
        waiting: false,
        coalesce: false,
    };
    link.read_preamble().await?;
    link.handshake(config).await?;
    {
        let mut guard = snapshot.lock().await;
        guard.connected = true;
    }
    set_status(pool, &config.id, ConnectionStatus::Connected).await;
    let _ = events.send(PoolEvent::Updated {
        id: config.id.clone(),
    });
    link.pump(commands, cancel).await
}

struct Link {
    stream: TcpStream,
    session: ClientSession<BUF>,
    snapshot: Arc<Mutex<DeckSnapshot>>,
    events: broadcast::Sender<PoolEvent>,
    id: String,
    options: PoolOptions,
    waiting: bool,
    coalesce: bool,
}

impl Link {
    async fn read_preamble(&mut self) -> Result<(), CallError> {
        let deadline = Instant::now() + self.options.connect_timeout;
        loop {
            if let Some(message) = self.drain().await? {
                let _ = message;
            }
            let guard = self.snapshot.lock().await;
            if !guard.model.is_empty() || !guard.protocol_version.is_empty() {
                return Ok(());
            }
            drop(guard);
            self.read_some(deadline).await?;
        }
    }

    async fn handshake(&mut self, config: &HyperDeckInstanceConfig) -> Result<(), CallError> {
        if !config.username.is_empty() {
            let block = format!(
                "authenticate:\r\nusername: {}\r\npassword: {}\r\n\r\n",
                config.username, config.password
            );
            self.roundtrip(block.into_bytes()).await?;
        }
        self.roundtrip(b"remote: override: true\r\n".to_vec())
            .await?;
        self.roundtrip(notify_command()).await?;
        match self.roundtrip(b"clips get: version: 2\r\n".to_vec()).await {
            Ok(_) => {}
            Err(error @ (CallError::Closed | CallError::Timeout)) => return Err(error),
            Err(error) => {
                tracing::debug!(id = %self.id, %error, "clips get version 2 skipped");
                match self.roundtrip(b"clips get\r\n".to_vec()).await {
                    Ok(_) => {}
                    Err(error @ (CallError::Closed | CallError::Timeout)) => return Err(error),
                    Err(error) => {
                        tracing::debug!(id = %self.id, %error, "clips get skipped");
                    }
                }
            }
        }
        for query in [
            "device info\r\n",
            "transport info\r\n",
            "slot info\r\n",
            "disk list\r\n",
            "configuration\r\n",
            "playrange\r\n",
            "remote\r\n",
            "uptime\r\n",
            "play option\r\n",
            "play on startup\r\n",
            "dynamic range\r\n",
            "nas selected\r\n",
            "external drive selected\r\n",
        ] {
            match self.roundtrip(query.as_bytes().to_vec()).await {
                Ok(_) => {}
                Err(error @ (CallError::Closed | CallError::Timeout)) => return Err(error),
                Err(error) => {
                    tracing::debug!(id = %self.id, %error, query, "hyperdeck query skipped");
                }
            }
        }
        Ok(())
    }

    async fn pump(
        &mut self,
        commands: &mut mpsc::Receiver<Call>,
        cancel: &CancellationToken,
    ) -> Result<(), CallError> {
        let mut pending: Option<oneshot::Sender<Result<DeckMessage, CallError>>> = None;
        let mut deadline: Option<Instant> = None;
        let mut buf = [0u8; 2048];
        loop {
            let idle = timeout(self.options.timecode_interval, std::future::pending::<()>());
            tokio::select! {
                _ = cancel.cancelled() => {
                    if let Some(reply) = pending.take() {
                        let _ = reply.send(Err(CallError::Closed));
                    }
                    return Ok(());
                }
                read = self.stream.read(&mut buf) => {
                    let count = read.map_err(|error| CallError::Protocol(error.to_string()))?;
                    if count == 0 {
                        if let Some(reply) = pending.take() {
                            let _ = reply.send(Err(CallError::Closed));
                        }
                        return Err(CallError::Closed);
                    }
                    self.session
                        .push(&buf[..count])
                        .map_err(|error| CallError::Protocol(error.to_string()))?;
                    if let Some(message) = self.drain().await?
                        && let Some(reply) = pending.take()
                    {
                        deadline = None;
                        let _ = reply.send(accept(message));
                    }
                }
                incoming = commands.recv(), if pending.is_none() => {
                    let Some(call) = incoming else {
                        return Ok(());
                    };
                    if let Err(error) = self.session.begin_command() {
                        let _ = call.reply.send(Err(CallError::Protocol(error.to_string())));
                        continue;
                    }
                    self.waiting = true;
                    if let Err(error) = self.stream.write_all(&call.bytes).await {
                        self.session.abandon();
                        self.waiting = false;
                        let _ = call.reply.send(Err(CallError::Protocol(error.to_string())));
                        return Err(CallError::Protocol(error.to_string()));
                    }
                    pending = Some(call.reply);
                    deadline = Some(Instant::now() + self.options.command_timeout);
                }
                _ = sleep_until(deadline), if deadline.is_some() => {
                    self.session.abandon();
                    self.waiting = false;
                    deadline = None;
                    if let Some(reply) = pending.take() {
                        let _ = reply.send(Err(CallError::Timeout));
                    }
                }
                _ = idle, if self.coalesce => {
                    self.coalesce = false;
                    let _ = self.events.send(PoolEvent::Updated { id: self.id.clone() });
                }
            }
        }
    }

    async fn roundtrip(&mut self, bytes: Vec<u8>) -> Result<DeckMessage, CallError> {
        self.session
            .begin_command()
            .map_err(|error| CallError::Protocol(error.to_string()))?;
        self.waiting = true;
        if let Err(error) = self.stream.write_all(&bytes).await {
            self.session.abandon();
            self.waiting = false;
            return Err(CallError::Protocol(error.to_string()));
        }
        let deadline = Instant::now() + self.options.command_timeout;
        loop {
            if let Some(message) = self.drain().await? {
                return accept(message);
            }
            self.read_some(deadline).await?;
        }
    }

    async fn read_some(&mut self, deadline: Instant) -> Result<(), CallError> {
        let mut buf = [0u8; 2048];
        let remaining = deadline.saturating_duration_since(Instant::now());
        let read = timeout(remaining, self.stream.read(&mut buf)).await;
        let count = match read {
            Ok(Ok(0)) => return Err(CallError::Closed),
            Ok(Ok(count)) => count,
            Ok(Err(error)) => return Err(CallError::Protocol(error.to_string())),
            Err(_) => {
                self.session.abandon();
                self.waiting = false;
                return Err(CallError::Timeout);
            }
        };
        self.session
            .push(&buf[..count])
            .map_err(|error| CallError::Protocol(error.to_string()))?;
        Ok(())
    }

    async fn drain(&mut self) -> Result<Option<DeckMessage>, CallError> {
        let mut response = None;
        loop {
            let event = self
                .session
                .poll::<BUF>()
                .map_err(|error| CallError::Protocol(error.to_string()))?;
            let Some(event) = event else {
                break;
            };
            match event {
                ClientEvent::Response(raw) => {
                    self.waiting = false;
                    let message = decode(&raw)?;
                    if !message.is_failure() {
                        self.apply_message(message.clone()).await;
                    }
                    response = Some(message);
                }
                ClientEvent::Notification(raw) | ClientEvent::ConnectionInfo(raw) => {
                    self.apply_notice(&raw).await?;
                }
            }
        }
        Ok(response)
    }

    async fn apply_notice(&mut self, raw: &RawMessage<BUF>) -> Result<(), CallError> {
        let message = decode(raw)?;
        self.apply_message(message).await;
        Ok(())
    }

    async fn apply_message(&mut self, message: DeckMessage) {
        let coalesced = is_timecode(&message);
        let changed = {
            let mut snapshot = self.snapshot.lock().await;
            snapshot.apply(&message)
        };
        if !changed {
            return;
        }
        if coalesced {
            self.coalesce = true;
        } else {
            let _ = self.events.send(PoolEvent::Updated {
                id: self.id.clone(),
            });
        }
    }
}

fn accept(message: DeckMessage) -> Result<DeckMessage, CallError> {
    if message.is_failure() {
        Err(CallError::Device {
            code: message.code,
            text: message.text,
        })
    } else {
        Ok(message)
    }
}

fn decode(raw: &RawMessage<BUF>) -> Result<DeckMessage, CallError> {
    let response = raw
        .response()
        .map_err(|error| CallError::Protocol(error.to_string()))?;
    let _ = response.code.class();
    let params = response
        .params()
        .map(|(key, value)| (key.to_string(), value.to_string()))
        .collect();
    let message = DeckMessage {
        code: response.code.as_u16(),
        text: response.text.to_string(),
        body: response.body.to_string(),
        params,
    };
    if message.is_failure() || response.code.class() == CodeClass::Failure {
        return Ok(message);
    }
    Ok(message)
}

fn is_timecode(message: &DeckMessage) -> bool {
    let text = message.text.to_ascii_lowercase();
    text.contains("display timecode")
        || text.contains("timeline position")
        || text.contains("dropped")
}

fn notify_command() -> Vec<u8> {
    let flags = [
        "transport",
        "slot",
        "configuration",
        "display timecode",
        "timeline position",
        "playrange",
        "clips",
        "disk",
        "dropped frames",
        "remote",
        "device info",
        "cache",
        "dynamic range",
    ];
    let params: Vec<(&str, &str)> = flags.iter().map(|name| (*name, "true")).collect();
    let mut line = String::new();
    encode_command(&mut line, "notify", &params).expect("notify command fits");
    line.into_bytes()
}

async fn sleep_until(deadline: Option<Instant>) {
    let Some(deadline) = deadline else {
        std::future::pending::<()>().await;
        return;
    };
    sleep(deadline.saturating_duration_since(Instant::now())).await;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::listen;

    fn config(addr: std::net::SocketAddr) -> HyperDeckInstanceConfig {
        HyperDeckInstanceConfig {
            id: "deck".into(),
            name: "Deck".into(),
            host: addr.ip().to_string(),
            port: addr.port(),
            color: "#4c8dff".into(),
            enabled: true,
            username: String::new(),
            password: String::new(),
        }
    }

    async fn wait_connected(pool: &HyperDeckPool) {
        let start = Instant::now();
        loop {
            if matches!(pool.status("deck").await, Some(ConnectionStatus::Connected)) {
                let snapshot = pool.snapshot("deck").await.unwrap();
                if snapshot.connected && !snapshot.status().is_empty() {
                    return;
                }
            }
            if start.elapsed() > Duration::from_secs(5) {
                panic!("deck did not connect: {:?}", pool.status("deck").await);
            }
            sleep(Duration::from_millis(20)).await;
        }
    }

    #[tokio::test]
    async fn play_updates_the_cached_transport() {
        let demo = listen().await;
        let pool = HyperDeckPool::new(PoolOptions::for_tests());
        pool.reconcile(vec![config(demo.addr)], Vec::new()).await;
        wait_connected(&pool).await;
        pool.call("deck", "play", &[]).await.unwrap();
        let snapshot = pool.snapshot("deck").await.unwrap();
        assert_eq!(snapshot.status(), "play");
        assert_eq!(snapshot.speed(), 100);
        assert!(
            !snapshot.transport_value("display timecode").is_empty(),
            "play notification should cache a timecode"
        );
    }

    #[tokio::test]
    async fn unsupported_command_returns_the_device_code() {
        let demo = listen().await;
        let pool = HyperDeckPool::new(PoolOptions::for_tests());
        pool.reconcile(vec![config(demo.addr)], Vec::new()).await;
        wait_connected(&pool).await;
        let error = pool.call("deck", "reboot", &[]).await.unwrap_err();
        assert!(
            matches!(error, CallError::Device { code: 103, .. }),
            "{error}"
        );
    }

    #[tokio::test]
    async fn reconnect_applies_a_fresh_session() {
        let demo = listen().await;
        let pool = HyperDeckPool::new(PoolOptions::for_tests());
        pool.reconcile(vec![config(demo.addr)], Vec::new()).await;
        wait_connected(&pool).await;
        pool.reconnect("deck").await;
        wait_connected(&pool).await;
        pool.call("deck", "record", &[]).await.unwrap();
        assert_eq!(pool.snapshot("deck").await.unwrap().status(), "record");
    }

    #[tokio::test]
    async fn handshake_query_response_fills_the_snapshot() {
        let demo = listen().await;
        let pool = HyperDeckPool::new(PoolOptions::for_tests());
        pool.reconcile(vec![config(demo.addr)], Vec::new()).await;
        wait_connected(&pool).await;
        let snapshot = pool.snapshot("deck").await.unwrap();
        assert_eq!(snapshot.transport_value("video format"), "1080p2997");
        assert_eq!(snapshot.model, "HyperDeck Rust");
        assert!(!snapshot.config_value("audio input").is_empty());
    }
}
