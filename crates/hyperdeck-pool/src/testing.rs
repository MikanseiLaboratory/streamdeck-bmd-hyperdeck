use std::net::SocketAddr;

use hyperdeck_tokio::{MockDeck, serve};
use tokio::task::JoinHandle;

/// Loopback HyperDeck used by tests.
pub struct DemoDeck {
    /// Bound address.
    pub addr: SocketAddr,
    handle: JoinHandle<()>,
}

impl Drop for DemoDeck {
    fn drop(&mut self) {
        self.handle.abort();
    }
}

/// Bind a demo deck and serve it until the guard is dropped.
pub async fn listen() -> DemoDeck {
    let (deck, addr, listener) = hyperdeck_tokio::bind_mock()
        .await
        .expect("bind mock hyperdeck");
    let handle = tokio::spawn(async move {
        let MockDeck { sim, hub, clock } = deck;
        let _ = serve(
            listener,
            sim,
            hub,
            hyperdeck_tokio::hyperdeck::DEMO_IDENTITY,
            clock,
        )
        .await;
    });
    DemoDeck { addr, handle }
}
