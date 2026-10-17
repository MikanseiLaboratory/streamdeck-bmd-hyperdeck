use serde::{Deserialize, Serialize};

/// One named HyperDeck.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HyperDeckInstanceConfig {
    pub id: String,
    pub name: String,
    pub host: String,
    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(default = "default_color")]
    pub color: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub username: String,
    #[serde(default)]
    pub password: String,
    /// How often this deck is polled for transport state, in milliseconds.
    #[serde(default = "default_update_interval_ms")]
    pub update_interval_ms: u32,
}

/// Default transport poll, faster than the previous fixed 500 ms.
pub const DEFAULT_UPDATE_INTERVAL_MS: u32 = 200;

/// Lowest poll a connection card can request.
pub const MIN_UPDATE_INTERVAL_MS: u32 = 50;

/// Highest poll a connection card can request.
pub const MAX_UPDATE_INTERVAL_MS: u32 = 5_000;

fn default_update_interval_ms() -> u32 {
    DEFAULT_UPDATE_INTERVAL_MS
}

/// Keep a requested interval inside the range the session will actually use.
pub fn clamp_update_interval_ms(value: u32) -> u32 {
    value.clamp(MIN_UPDATE_INTERVAL_MS, MAX_UPDATE_INTERVAL_MS)
}

fn default_port() -> u16 {
    9993
}

fn default_color() -> String {
    "#4c8dff".into()
}

fn default_true() -> bool {
    true
}

impl HyperDeckInstanceConfig {
    /// `host:port` passed to `TcpStream::connect`.
    pub fn socket_addr(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}

/// A named set of instance ids.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TargetGroup {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub members: Vec<String>,
}

/// Which configured decks an action talks to.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum TargetSelector {
    #[default]
    All,
    Group {
        id: String,
    },
    Instances {
        ids: Vec<String>,
    },
}

/// Enabled instance ids selected by `selector`.
pub fn resolve_targets(
    configs: &[HyperDeckInstanceConfig],
    groups: &[TargetGroup],
    selector: &TargetSelector,
) -> Vec<String> {
    let enabled: Vec<&HyperDeckInstanceConfig> =
        configs.iter().filter(|config| config.enabled).collect();
    match selector {
        TargetSelector::All => enabled
            .into_iter()
            .map(|config| config.id.clone())
            .collect(),
        TargetSelector::Group { id } => {
            let Some(group) = groups.iter().find(|group| group.id == *id) else {
                return Vec::new();
            };
            group
                .members
                .iter()
                .filter(|member| enabled.iter().any(|config| config.id == **member))
                .cloned()
                .collect()
        }
        TargetSelector::Instances { ids } => ids
            .iter()
            .filter(|id| enabled.iter().any(|config| config.id == **id))
            .cloned()
            .collect(),
    }
}
