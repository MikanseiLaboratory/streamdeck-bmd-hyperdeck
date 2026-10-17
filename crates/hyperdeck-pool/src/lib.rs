//! Connection pool for an arbitrary number of HyperDeck Ethernet clients.
//!
//! Each configured deck is owned by a supervisor task. That task keeps one TCP
//! session, serializes commands, applies `5xx` notifications, and reconnects
//! with exponential backoff.

mod config;
mod message;
mod pool;
mod snapshot;
mod status;

pub use config::{
    DEFAULT_UPDATE_INTERVAL_MS, HyperDeckInstanceConfig, MAX_UPDATE_INTERVAL_MS,
    MIN_UPDATE_INTERVAL_MS, TargetGroup, TargetSelector, clamp_update_interval_ms, resolve_targets,
};
pub use message::DeckMessage;
pub use pool::{CallError, HyperDeckPool, PoolEvent, PoolOptions};
pub use snapshot::{ClipRow, DeckSnapshot};
pub use status::{ConnectionStatus, InstanceStatus};

#[cfg(any(test, feature = "mock"))]
pub mod testing;
