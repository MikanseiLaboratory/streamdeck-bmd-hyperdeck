use serde::{Deserialize, Serialize};

/// Connection state of one deck.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum ConnectionStatus {
    Disabled,
    Connecting,
    Connected,
    Disconnected { message: String },
}

impl ConnectionStatus {
    /// Short label for the property inspector.
    pub fn label(&self) -> &str {
        match self {
            Self::Disabled => "Disabled",
            Self::Connecting => "Connecting",
            Self::Connected => "Connected",
            Self::Disconnected { .. } => "Disconnected",
        }
    }
}

/// Status row for one configured deck.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstanceStatus {
    pub id: String,
    pub name: String,
    pub status: ConnectionStatus,
    pub model: String,
}
