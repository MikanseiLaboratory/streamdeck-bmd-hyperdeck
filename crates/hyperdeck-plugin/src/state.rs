use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use hyperdeck_pool::{ConnectionStatus, HyperDeckPool, PoolEvent, PoolOptions, resolve_targets};
use serde_json::{Value, json};
use streamdeck_plugin::{CommandSender, PluginLifecycle, Target, async_trait};
use tokio::sync::{Mutex, Notify};
use tokio::time::sleep;
use tokio_util::sync::CancellationToken;

use crate::contracts::{ActionSettings, GlobalSettings, localhost_instance};
use crate::kind::{ActionKind, PressKind};
use crate::ops::{self, Gesture};
use crate::render::{self, Segment, SegmentState};

struct Press {
    token: CancellationToken,
    fired_long: Arc<AtomicBool>,
}

struct LiveKey {
    kind: ActionKind,
    settings: ActionSettings,
    multi: bool,
    /// Layout last accepted by Stream Deck. Sent again only when it changes.
    layout: String,
    /// Picture last accepted by Stream Deck. Unchanged faces are not sent again.
    sent: SentFace,
}

/// Last `setImage` / `setTitle` / `setState` / `setFeedback` accepted for one key.
#[derive(Clone, Debug, Default)]
struct SentFace {
    image: Option<String>,
    title: Option<String>,
    state: Option<i32>,
    feedback: Option<Value>,
}

struct Runtime {
    sender: Mutex<Option<CommandSender>>,
    keys: Mutex<HashMap<String, LiveKey>>,
    presses: Mutex<HashMap<String, Press>>,
    global: Mutex<GlobalSettings>,
    dirty: Mutex<HashSet<String>>,
    dirty_notify: Notify,
    inspectors: Mutex<HashSet<String>>,
    catalog_dirty: Mutex<HashSet<String>>,
    catalog_notify: Notify,
}

/// Shared plugin state. Actions forward presses here.
#[derive(Clone)]
pub struct AppState {
    /// Connection pool.
    pub pool: HyperDeckPool,
    runtime: Arc<Runtime>,
}

impl Default for AppState {
    fn default() -> Self {
        Self::new(PoolOptions::default())
    }
}

impl AppState {
    /// Pool plus the repaint and catalog workers.
    pub fn new(options: PoolOptions) -> Self {
        let state = Self::detached(HyperDeckPool::new(options));
        state.spawn_workers();
        state
    }

    /// Pool without background workers. Tests call [`Self::refresh_key`] directly.
    pub fn detached(pool: HyperDeckPool) -> Self {
        Self {
            pool,
            runtime: Arc::new(Runtime {
                sender: Mutex::new(None),
                keys: Mutex::new(HashMap::new()),
                presses: Mutex::new(HashMap::new()),
                global: Mutex::new(GlobalSettings::default()),
                dirty: Mutex::new(HashSet::new()),
                dirty_notify: Notify::new(),
                inspectors: Mutex::new(HashSet::new()),
                catalog_dirty: Mutex::new(HashSet::new()),
                catalog_notify: Notify::new(),
            }),
        }
    }

    /// Remember the command channel and ask Stream Deck for global settings once.
    pub async fn note_sender(&self, sender: CommandSender) {
        let mut current = self.runtime.sender.lock().await;
        if current.is_none() {
            let _ = sender.get_global_settings(None);
            *current = Some(sender);
        }
    }

    /// Apply global settings, seeding localhost only on the first launch.
    pub async fn apply_global_value(&self, value: &Value) {
        let mut settings: GlobalSettings =
            serde_json::from_value(value.clone()).unwrap_or_default();
        if !settings.seeded {
            if settings.instances.is_empty() {
                settings.instances.push(localhost_instance());
            }
            settings.seeded = true;
            if let Some(sender) = self.runtime.sender.lock().await.clone()
                && let Ok(payload) = serde_json::to_value(&settings)
            {
                let _ = sender.set_global_settings(&payload);
            }
        }
        self.apply_global(settings).await;
    }

    async fn apply_global(&self, settings: GlobalSettings) {
        let instances = settings.instances.iter().cloned().map(Into::into).collect();
        let groups = settings.groups.iter().cloned().map(Into::into).collect();
        self.pool.reconcile(instances, groups).await;
        *self.runtime.global.lock().await = settings;
        let contexts: Vec<String> = self.runtime.keys.lock().await.keys().cloned().collect();
        for context in contexts {
            self.refresh_key(&context).await;
        }
        self.mark_catalog_open().await;
    }

    /// Register or replace the action on `context`.
    pub async fn upsert_key(
        &self,
        context: &str,
        kind: ActionKind,
        settings: ActionSettings,
        multi: bool,
    ) {
        self.runtime.keys.lock().await.insert(
            context.to_string(),
            LiveKey {
                kind,
                settings,
                multi,
                layout: String::new(),
                sent: SentFace::default(),
            },
        );
        self.refresh_key(context).await;
        self.mark_catalog_open().await;
    }

    /// Forget a key that left the profile.
    pub async fn remove_key(&self, context: &str) {
        self.runtime.keys.lock().await.remove(context);
        self.runtime.inspectors.lock().await.remove(context);
        self.runtime.catalog_dirty.lock().await.remove(context);
        if let Some(press) = self.runtime.presses.lock().await.remove(context) {
            press.token.cancel();
        }
    }

    /// The property inspector opened for this key.
    pub async fn inspector_appeared(&self, context: &str) {
        self.runtime
            .inspectors
            .lock()
            .await
            .insert(context.to_string());
        self.push_status(context).await;
        self.push_catalog(context).await;
    }

    /// The property inspector closed.
    pub async fn inspector_disappeared(&self, context: &str) {
        self.runtime.inspectors.lock().await.remove(context);
        self.runtime.catalog_dirty.lock().await.remove(context);
    }

    /// Start a press. A hold past the threshold runs the long-press command.
    pub fn begin_press(&self, context: &str) {
        let context = context.to_string();
        let state = self.clone();
        tokio::spawn(async move {
            let (long_press, delay) = {
                let keys = state.runtime.keys.lock().await;
                let Some(key) = keys.get(&context) else {
                    return;
                };
                let global = state.runtime.global.lock().await.long_press_ms.max(100);
                let delay = if key.settings.advanced.long_press_ms == 0 {
                    global
                } else {
                    key.settings.advanced.long_press_ms
                };
                (key.settings.advanced.long_press, delay)
            };
            if !long_press {
                return;
            }
            let token = CancellationToken::new();
            let fired = Arc::new(AtomicBool::new(false));
            state.runtime.presses.lock().await.insert(
                context.clone(),
                Press {
                    token: token.clone(),
                    fired_long: Arc::clone(&fired),
                },
            );
            tokio::select! {
                _ = token.cancelled() => {}
                _ = sleep(Duration::from_millis(u64::from(delay))) => {
                    fired.store(true, Ordering::SeqCst);
                    state.run_gesture(&context, Gesture::Press(PressKind::Long)).await;
                }
            }
        });
    }

    /// Finish a press. A hold that already fired does not also fire the short press.
    pub fn end_press(&self, context: &str) {
        let context = context.to_string();
        let state = self.clone();
        tokio::spawn(async move {
            let press = state.runtime.presses.lock().await.remove(&context);
            if let Some(press) = press {
                press.token.cancel();
                if press.fired_long.load(Ordering::SeqCst) {
                    return;
                }
            }
            state
                .run_gesture(&context, Gesture::Press(PressKind::Single))
                .await;
        });
    }

    /// Dial push.
    pub fn dial_down(&self, context: &str) {
        self.spawn_gesture(context, Gesture::Press(PressKind::Single));
    }

    /// Dial touch.
    pub fn dial_touch(&self, context: &str) {
        self.spawn_gesture(context, Gesture::Touch);
    }

    /// Dial rotation.
    pub fn rotate(&self, context: &str, ticks: i32) {
        self.spawn_gesture(context, Gesture::Rotate(ticks));
    }

    /// Property inspector message. `ready` and `reconnect` are handled.
    pub fn handle_inspector(&self, context: &str, payload: Value) {
        let context = context.to_string();
        let state = self.clone();
        tokio::spawn(async move {
            let message_type = payload.get("type").and_then(Value::as_str).unwrap_or("");
            match message_type {
                "ready" | "refresh" => {
                    state
                        .runtime
                        .inspectors
                        .lock()
                        .await
                        .insert(context.clone());
                    state.push_status(&context).await;
                    state.push_catalog(&context).await;
                }
                "reconnect" => {
                    if let Some(id) = payload.get("id").and_then(Value::as_str) {
                        state.pool.reconnect(id).await;
                    } else {
                        let ids: Vec<String> = state
                            .runtime
                            .global
                            .lock()
                            .await
                            .instances
                            .iter()
                            .map(|instance| instance.id.clone())
                            .collect();
                        for id in ids {
                            state.pool.reconnect(&id).await;
                        }
                    }
                }
                _ => {}
            }
        });
    }

    /// Schedule a repaint. The picture is read from the deck snapshots when it is drawn.
    pub async fn refresh_key(&self, context: &str) {
        if self.key_settings(context).await.is_none() {
            return;
        }
        self.mark_dirty(context).await;
    }

    async fn run_gesture(&self, context: &str, gesture: Gesture) {
        let Some((kind, settings)) = self.key_settings(context).await else {
            return;
        };
        let targets = self.targets_for(&settings).await;
        let mut any_ok = false;
        for id in &targets {
            let snapshot = self.pool.snapshot(id).await.unwrap_or_default();
            let params = settings.params_for(id).clone();
            let commands = ops::commands(kind, &params, &snapshot, gesture);
            if commands.is_empty() {
                continue;
            }
            let mut failed = false;
            for command in &commands {
                if let Err(error) = self.pool.call_raw(id, command.as_bytes()).await {
                    tracing::warn!(id, %error, "hyperdeck command failed");
                    failed = true;
                    break;
                }
            }
            if !failed {
                any_ok = true;
            }
        }
        self.refresh_key(context).await;
        if kind.confirms_press() && matches!(gesture, Gesture::Press(_)) {
            self.feedback(context, any_ok && !targets.is_empty()).await;
        }
    }

    fn spawn_gesture(&self, context: &str, gesture: Gesture) {
        let context = context.to_string();
        let state = self.clone();
        tokio::spawn(async move {
            state.run_gesture(&context, gesture).await;
        });
    }

    async fn on_pool_event(&self, event: &PoolEvent) {
        let id = match event {
            PoolEvent::Status { id } | PoolEvent::Updated { id } => id.clone(),
        };
        let contexts: Vec<String> = self.runtime.keys.lock().await.keys().cloned().collect();
        for context in contexts {
            let Some((_, settings)) = self.key_settings(&context).await else {
                continue;
            };
            let targets = self.targets_for(&settings).await;
            if targets.iter().any(|target| target == &id) {
                self.refresh_key(&context).await;
            }
        }
        self.mark_catalog_open().await;
        self.push_status_to_open_inspectors().await;
    }

    async fn paint(&self, context: &str) {
        let Some(sender) = self.runtime.sender.lock().await.clone() else {
            return;
        };
        let (kind, settings, multi, sent_layout, sent) = {
            let keys = self.runtime.keys.lock().await;
            let Some(key) = keys.get(context) else {
                return;
            };
            (
                key.kind,
                key.settings.clone(),
                key.multi,
                key.layout.clone(),
                key.sent.clone(),
            )
        };
        let global = self.runtime.global.lock().await.clone();
        let targets = self.targets_for(&settings).await;
        let mut visual = Vec::with_capacity(targets.len());
        let mut captions = Vec::with_capacity(targets.len());
        let mut levels = Vec::with_capacity(targets.len());
        let mut titles = HashMap::new();
        for id in &targets {
            let appearance = self.appearance(kind, &settings, id).await;
            let instance = global.instances.iter().find(|item| item.id == *id);
            let params = settings.params_for(id);
            let mut color = instance
                .map(|item| item.color.clone())
                .unwrap_or_else(|| "#4c8dff".into());
            if appearance.state == SegmentState::Active && !params.active_color.trim().is_empty() {
                color = params.active_color.trim().to_string();
            }
            visual.push(Segment {
                label: instance
                    .map(|item| item.name.clone())
                    .unwrap_or_else(|| id.clone()),
                color,
                state: appearance.state,
            });
            captions.push(appearance.title.clone());
            levels.push(appearance.indicator);
            if !appearance.title.trim().is_empty() {
                titles.insert(id.clone(), appearance.title);
            }
        }
        let title = join_titles(&targets, &titles);
        if kind.is_dial() {
            let (layout, payload) =
                render::dial_feedback(kind, &visual, &captions, &levels, &global.fg_color);
            let mut previous_feedback = sent.feedback;
            if sent_layout != layout
                && sender.set_feedback_layout(context, &layout).is_ok()
                && let Some(key) = self.runtime.keys.lock().await.get_mut(context)
            {
                key.layout = layout;
                key.sent.feedback = None;
                previous_feedback = None;
            }
            let delta = feedback_delta(previous_feedback.as_ref(), &payload);
            if !feedback_is_blank(&delta)
                && sender.set_feedback(context, &delta).is_ok()
                && let Some(key) = self.runtime.keys.lock().await.get_mut(context)
            {
                key.sent.feedback = Some(payload);
            }
            return;
        }
        if !multi {
            let image = render::key_image(kind, &visual, &global.fg_color);
            let both_states = kind.has_toggle_state();
            let image_changed = sent.image.as_deref() != Some(image.as_str());
            let title_changed = sent.title.as_deref() != Some(title.as_str());
            if (image_changed || title_changed)
                && publish_face(
                    &sender,
                    context,
                    both_states,
                    image_changed.then_some(image.as_str()),
                    title_changed.then_some(title.as_str()),
                )
                && let Some(key) = self.runtime.keys.lock().await.get_mut(context)
            {
                if image_changed {
                    key.sent.image = Some(image);
                }
                if title_changed {
                    key.sent.title = Some(title);
                }
            }
        }
        if kind.has_toggle_state() {
            let active = render::hardware_state(visual.iter().map(|segment| segment.state));
            if sent.state != Some(active)
                && sender.set_state(context, active).is_ok()
                && let Some(key) = self.runtime.keys.lock().await.get_mut(context)
            {
                key.sent.state = Some(active);
            }
        }
    }

    async fn feedback(&self, context: &str, ok: bool) {
        let Some(sender) = self.runtime.sender.lock().await.clone() else {
            return;
        };
        if ok {
            let _ = sender.show_ok(context);
        } else {
            let _ = sender.show_alert(context);
        }
    }

    async fn push_status_to_open_inspectors(&self) {
        let contexts: Vec<String> = self
            .runtime
            .inspectors
            .lock()
            .await
            .iter()
            .cloned()
            .collect();
        for context in contexts {
            self.push_status(&context).await;
        }
    }

    async fn push_status(&self, context: &str) {
        let Some(sender) = self.runtime.sender.lock().await.clone() else {
            return;
        };
        let instances = self.pool.statuses().await;
        let _ = sender.send_to_property_inspector(
            context,
            &json!({"type": "status", "instances": instances}),
        );
    }

    async fn push_catalog(&self, context: &str) {
        let Some(sender) = self.runtime.sender.lock().await.clone() else {
            return;
        };
        let Some((_, settings)) = self.key_settings(context).await else {
            return;
        };
        let targets = self.targets_for(&settings).await;
        let mut decks = Vec::new();
        for id in &targets {
            let snapshot = self.pool.snapshot(id).await.unwrap_or_default();
            let name = self
                .runtime
                .global
                .lock()
                .await
                .instances
                .iter()
                .find(|item| item.id == *id)
                .map(|item| item.name.clone())
                .unwrap_or_else(|| id.clone());
            decks.push(json!({
                "id": id,
                "name": name,
                "clips": snapshot.clips,
                "diskClips": snapshot.disk_clips,
                "slot": snapshot.slot,
                "config": snapshot.config,
                "nas": snapshot.config_value("url"),
                "externalDrive": snapshot.config_value("selected"),
            }));
        }
        let _ =
            sender.send_to_property_inspector(context, &json!({"type": "catalog", "decks": decks}));
    }

    async fn mark_catalog_open(&self) {
        let open: HashSet<String> = self.runtime.inspectors.lock().await.clone();
        if open.is_empty() {
            return;
        }
        self.runtime.catalog_dirty.lock().await.extend(open);
        self.runtime.catalog_notify.notify_waiters();
    }

    async fn targets_for(&self, settings: &ActionSettings) -> Vec<String> {
        let global = self.runtime.global.lock().await.clone();
        let configs: Vec<_> = global.instances.into_iter().map(Into::into).collect();
        let groups: Vec<_> = global.groups.into_iter().map(Into::into).collect();
        resolve_targets(&configs, &groups, &(&settings.common.target).into())
    }

    async fn key_settings(&self, context: &str) -> Option<(ActionKind, ActionSettings)> {
        let keys = self.runtime.keys.lock().await;
        let key = keys.get(context)?;
        Some((key.kind, key.settings.clone()))
    }

    async fn appearance(
        &self,
        kind: ActionKind,
        settings: &ActionSettings,
        id: &str,
    ) -> Appearance {
        let snapshot = self.pool.snapshot(id).await.unwrap_or_default();
        let connected = matches!(
            self.pool.status(id).await,
            Some(ConnectionStatus::Connected)
        ) && snapshot.connected;
        let face = ops::face(kind, settings.params_for(id), &snapshot);
        let state = if !connected {
            SegmentState::Disconnected
        } else if face.active {
            SegmentState::Active
        } else {
            SegmentState::Off
        };
        Appearance {
            state,
            title: face.title,
            indicator: face.indicator,
        }
    }

    async fn mark_dirty(&self, context: &str) {
        self.runtime.dirty.lock().await.insert(context.to_string());
        self.runtime.dirty_notify.notify_waiters();
    }

    fn spawn_workers(&self) {
        let painter = self.clone();
        tokio::spawn(async move {
            loop {
                let notified = painter.runtime.dirty_notify.notified();
                tokio::pin!(notified);
                if painter.runtime.dirty.lock().await.is_empty() {
                    notified.await;
                }
                sleep(Duration::from_millis(15)).await;
                let contexts: Vec<String> = painter.runtime.dirty.lock().await.drain().collect();
                for context in contexts {
                    painter.paint(&context).await;
                }
            }
        });

        let listener = self.clone();
        let mut events = self.pool.subscribe();
        tokio::spawn(async move {
            loop {
                match events.recv().await {
                    Ok(event) => listener.on_pool_event(&event).await,
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {}
                    Err(_) => break,
                }
            }
        });

        let catalogs = self.clone();
        tokio::spawn(async move {
            loop {
                let notified = catalogs.runtime.catalog_notify.notified();
                tokio::pin!(notified);
                if catalogs.runtime.catalog_dirty.lock().await.is_empty() {
                    notified.await;
                }
                sleep(Duration::from_millis(200)).await;
                let contexts: Vec<String> = catalogs
                    .runtime
                    .catalog_dirty
                    .lock()
                    .await
                    .drain()
                    .collect();
                for context in contexts {
                    catalogs.push_catalog(&context).await;
                }
            }
        });
    }

    #[cfg(test)]
    async fn segment_state(&self, context: &str, id: &str) -> Option<SegmentState> {
        let (kind, settings) = self.key_settings(context).await?;
        let targets = self.targets_for(&settings).await;
        if !targets.iter().any(|target| target == id) {
            return None;
        }
        Some(self.appearance(kind, &settings, id).await.state)
    }
}

struct Appearance {
    state: SegmentState,
    title: String,
    indicator: u8,
}

/// Keys that differ from the feedback Stream Deck already has.
///
/// A timecode tick must not resend the filename. Stream Deck redraws every
/// property in the payload, and repeating an unchanged title blanks it.
fn feedback_delta(previous: Option<&Value>, next: &Value) -> Value {
    let Some(previous) = previous else {
        return next.clone();
    };
    let (Some(prev), Some(next_map)) = (previous.as_object(), next.as_object()) else {
        return if previous == next {
            json!({})
        } else {
            next.clone()
        };
    };
    let mut delta = serde_json::Map::new();
    for (key, value) in next_map {
        if prev.get(key) != Some(value) {
            delta.insert(key.clone(), value.clone());
        }
    }
    Value::Object(delta)
}

fn feedback_is_blank(value: &Value) -> bool {
    value.as_object().is_some_and(|map| map.is_empty())
}

/// `setImage` / `setTitle` for the current face.
///
/// Toggle actions have two hardware states. A title sent without a state index
/// lands on the state that is visible now, so the other state stays blank and
/// the text vanishes on the next `setState`.
fn publish_face(
    sender: &CommandSender,
    context: &str,
    both_states: bool,
    image: Option<&str>,
    title: Option<&str>,
) -> bool {
    let states: &[Option<i32>] = if both_states {
        &[Some(0), Some(1)]
    } else {
        &[None]
    };
    for state in states {
        if let Some(image) = image
            && sender
                .set_image(context, Some(image), Target::HardwareAndSoftware, *state)
                .is_err()
        {
            return false;
        }
        if let Some(title) = title
            && sender
                .set_title(context, Some(title), Target::HardwareAndSoftware, *state)
                .is_err()
        {
            return false;
        }
    }
    true
}

fn join_titles(targets: &[String], titles: &HashMap<String, String>) -> String {
    let mut unique = Vec::new();
    for id in targets {
        let Some(title) = titles.get(id) else {
            continue;
        };
        if !title.is_empty() && !unique.iter().any(|have: &String| have == title) {
            unique.push(title.clone());
        }
    }
    unique.join(" / ")
}

/// Receives `didReceiveGlobalSettings`.
pub struct GlobalHook {
    /// Plugin state shared with the actions.
    pub state: AppState,
}

#[async_trait]
impl PluginLifecycle for GlobalHook {
    async fn on_did_receive_global_settings(
        &mut self,
        settings: &Value,
    ) -> streamdeck_plugin::Result<()> {
        self.state.apply_global_value(settings).await;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use hyperdeck_pool::{ConnectionStatus, PoolOptions, testing::listen};
    use tokio::time::Instant;

    use super::*;
    use crate::contracts::{ActionSettings, GlobalSettings, InstanceConfig};
    use crate::kind::ActionKind;

    fn instance(id: &str, addr: std::net::SocketAddr, color: &str) -> InstanceConfig {
        InstanceConfig {
            id: id.into(),
            name: id.into(),
            host: addr.ip().to_string(),
            port: addr.port(),
            color: color.into(),
            enabled: true,
            username: String::new(),
            password: String::new(),
            update_interval_ms: hyperdeck_pool::DEFAULT_UPDATE_INTERVAL_MS,
        }
    }

    async fn wait_connected(pool: &HyperDeckPool, id: &str) {
        let start = Instant::now();
        loop {
            let ready = matches!(pool.status(id).await, Some(ConnectionStatus::Connected))
                && pool
                    .snapshot(id)
                    .await
                    .is_some_and(|snapshot| snapshot.connected && !snapshot.status().is_empty());
            if ready {
                return;
            }
            if start.elapsed() > Duration::from_secs(5) {
                panic!("deck {id} did not connect: {:?}", pool.status(id).await);
            }
            sleep(Duration::from_millis(20)).await;
        }
    }

    #[test]
    fn feedback_delta_leaves_an_unchanged_filename_out() {
        let previous = json!({
            "filename": "clip.mov",
            "timecode": "01:02:03:04",
        });
        let next = json!({
            "filename": "clip.mov",
            "timecode": "01:02:03:05",
        });
        let delta = feedback_delta(Some(&previous), &next);
        assert_eq!(delta, json!({ "timecode": "01:02:03:05" }));
        assert!(feedback_is_blank(&feedback_delta(Some(&next), &next)));
        assert_eq!(feedback_delta(None, &next), next);
    }

    #[tokio::test]
    async fn only_the_recording_deck_lights_its_band() {
        let first = listen().await;
        let second = listen().await;
        let pool = HyperDeckPool::new(PoolOptions::for_tests());
        let state = AppState::detached(pool.clone());
        let settings = GlobalSettings {
            seeded: true,
            instances: vec![
                instance("a", first.addr, "#4c8dff"),
                instance("b", second.addr, "#ef5b5b"),
            ],
            ..GlobalSettings::default()
        };
        state.apply_global(settings).await;
        wait_connected(&pool, "a").await;
        wait_connected(&pool, "b").await;
        state
            .upsert_key("key", ActionKind::Record, ActionSettings::default(), false)
            .await;
        pool.call("a", "record", &[]).await.unwrap();
        state.refresh_key("key").await;
        assert_eq!(
            state.segment_state("key", "a").await,
            Some(SegmentState::Active)
        );
        assert_eq!(
            state.segment_state("key", "b").await,
            Some(SegmentState::Off)
        );
    }
}
