use std::collections::BTreeMap;

use hyperdeck_pool::{
    HyperDeckInstanceConfig, TargetGroup as PoolGroup, TargetSelector as PoolSelector,
};
use serde::{Deserialize, Serialize};
use streamdeck_plugin::{TS, export_ts};

/// One named HyperDeck, as stored in global settings.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export, rename_all = "camelCase")]
#[serde(rename_all = "camelCase")]
pub struct InstanceConfig {
    pub id: String,
    pub name: String,
    #[serde(default = "default_host")]
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
}

fn default_host() -> String {
    "127.0.0.1".into()
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

impl From<InstanceConfig> for HyperDeckInstanceConfig {
    fn from(value: InstanceConfig) -> Self {
        Self {
            id: value.id,
            name: value.name,
            host: value.host,
            port: value.port,
            color: value.color,
            enabled: value.enabled,
            username: value.username,
            password: value.password,
        }
    }
}

/// A named set of instance ids.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export, rename_all = "camelCase")]
#[serde(rename_all = "camelCase")]
pub struct TargetGroup {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub members: Vec<String>,
}

impl From<TargetGroup> for PoolGroup {
    fn from(value: TargetGroup) -> Self {
        Self {
            id: value.id,
            name: value.name,
            members: value.members,
        }
    }
}

/// Which configured decks an action talks to.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(tag = "kind", rename_all = "camelCase")]
#[ts(tag = "kind", rename_all = "camelCase", export)]
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

impl From<&TargetSelector> for PoolSelector {
    fn from(value: &TargetSelector) -> Self {
        match value {
            TargetSelector::All => Self::All,
            TargetSelector::Group { id } => Self::Group { id: id.clone() },
            TargetSelector::Instances { ids } => Self::Instances { ids: ids.clone() },
        }
    }
}

/// The deck inserted on the first launch.
pub fn localhost_instance() -> InstanceConfig {
    InstanceConfig {
        id: "localhost".into(),
        name: "Localhost".into(),
        host: default_host(),
        port: default_port(),
        color: default_color(),
        enabled: true,
        username: String::new(),
        password: String::new(),
    }
}

/// Plugin-wide settings.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[ts(export, rename_all = "camelCase")]
#[serde(rename_all = "camelCase")]
pub struct GlobalSettings {
    #[serde(default)]
    pub instances: Vec<InstanceConfig>,
    #[serde(default)]
    pub groups: Vec<TargetGroup>,
    #[serde(default = "default_long_press")]
    pub long_press_ms: u32,
    #[serde(default = "default_fg")]
    pub fg_color: String,
    #[serde(default)]
    pub seeded: bool,
}

fn default_long_press() -> u32 {
    500
}

fn default_fg() -> String {
    "#f4f7fb".into()
}

impl Default for GlobalSettings {
    fn default() -> Self {
        Self {
            instances: Vec::new(),
            groups: Vec::new(),
            long_press_ms: default_long_press(),
            fg_color: default_fg(),
            seeded: false,
        }
    }
}

/// Target and whether parameters are shared across decks.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[ts(export, rename_all = "camelCase")]
#[serde(rename_all = "camelCase")]
pub struct CommonSettings {
    #[serde(default)]
    pub target: TargetSelector,
    #[serde(default = "default_true")]
    pub shared_params: bool,
}

impl Default for CommonSettings {
    fn default() -> Self {
        Self {
            target: TargetSelector::All,
            shared_params: true,
        }
    }
}

/// Optional hold-to-act behavior.
#[derive(Clone, Debug, Default, Serialize, Deserialize, TS)]
#[ts(export, rename_all = "camelCase")]
#[serde(rename_all = "camelCase")]
pub struct AdvancedSettings {
    #[serde(default)]
    pub long_press: bool,
    #[serde(default)]
    pub long_press_ms: u32,
}

/// Fields used by every action. The inspector shows only the ones that apply.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[ts(export, rename_all = "camelCase")]
#[serde(rename_all = "camelCase")]
pub struct ActionParams {
    #[serde(default = "default_speed")]
    pub speed: i32,
    #[serde(default)]
    pub loop_playback: bool,
    #[serde(default)]
    pub single_clip: bool,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub slot_id: String,
    #[serde(default)]
    pub device: String,
    #[serde(default)]
    pub clip_id: String,
    #[serde(default)]
    pub command_mode: String,
    #[serde(default)]
    pub range_count: String,
    #[serde(default)]
    pub range_in: String,
    #[serde(default)]
    pub range_out: String,
    #[serde(default = "default_goto")]
    pub goto_mode: String,
    #[serde(default)]
    pub timecode: String,
    #[serde(default)]
    pub mode: String,
    #[serde(default)]
    pub value: String,
    #[serde(default = "default_threshold")]
    pub threshold_seconds: u32,
    #[serde(default = "default_status_field")]
    pub status_field: String,
    #[serde(default)]
    pub raw: String,
    #[serde(default)]
    pub raw_rotate_up: String,
    #[serde(default)]
    pub raw_rotate_down: String,
    #[serde(default)]
    pub raw_push: String,
    #[serde(default)]
    pub raw_touch: String,
    #[serde(default)]
    pub step: i32,
    #[serde(default = "default_direction")]
    pub direction: String,
    #[serde(default)]
    pub active_color: String,
    #[serde(default)]
    pub confirm: bool,
    #[serde(default)]
    pub slate_reel: String,
    #[serde(default)]
    pub slate_scene: String,
    #[serde(default)]
    pub slate_take: String,
    #[serde(default)]
    pub slate_project: String,
    #[serde(default)]
    pub slate_lens: String,
    #[serde(default)]
    pub nas_url: String,
    #[serde(default)]
    pub nas_user: String,
    #[serde(default)]
    pub nas_password: String,
}

fn default_speed() -> i32 {
    100
}

fn default_threshold() -> u32 {
    60
}

fn default_status_field() -> String {
    "status".into()
}

fn default_goto() -> String {
    "timeline-start".into()
}

fn default_direction() -> String {
    "next".into()
}

impl Default for ActionParams {
    fn default() -> Self {
        Self {
            speed: default_speed(),
            loop_playback: false,
            single_clip: false,
            name: String::new(),
            slot_id: String::new(),
            device: String::new(),
            clip_id: String::new(),
            command_mode: String::new(),
            range_count: String::new(),
            range_in: String::new(),
            range_out: String::new(),
            goto_mode: default_goto(),
            timecode: String::new(),
            mode: String::new(),
            value: String::new(),
            threshold_seconds: default_threshold(),
            status_field: default_status_field(),
            raw: String::new(),
            raw_rotate_up: String::new(),
            raw_rotate_down: String::new(),
            raw_push: String::new(),
            raw_touch: String::new(),
            step: 0,
            direction: default_direction(),
            active_color: String::new(),
            confirm: false,
            slate_reel: String::new(),
            slate_scene: String::new(),
            slate_take: String::new(),
            slate_project: String::new(),
            slate_lens: String::new(),
            nas_url: String::new(),
            nas_user: String::new(),
            nas_password: String::new(),
        }
    }
}

/// Settings stored on one Stream Deck action.
#[derive(Clone, Debug, Default, Serialize, Deserialize, TS)]
#[ts(export, rename_all = "camelCase")]
#[serde(rename_all = "camelCase")]
pub struct ActionSettings {
    #[serde(default)]
    pub common: CommonSettings,
    #[serde(default)]
    pub advanced: AdvancedSettings,
    #[serde(default)]
    pub shared: ActionParams,
    #[serde(default)]
    pub params: BTreeMap<String, ActionParams>,
}

impl ActionSettings {
    /// Parameters for one deck, honoring the shared-params switch.
    pub fn params_for(&self, instance_id: &str) -> &ActionParams {
        if self.common.shared_params {
            &self.shared
        } else {
            self.params.get(instance_id).unwrap_or(&self.shared)
        }
    }
}

export_ts!(InstanceConfig);
export_ts!(TargetGroup);
export_ts!(TargetSelector);
export_ts!(GlobalSettings);
export_ts!(CommonSettings);
export_ts!(AdvancedSettings);
export_ts!(ActionParams);
export_ts!(ActionSettings);

/// Keep the TypeScript exports linked into the typegen binary.
pub fn force_link() {
    let _ = std::any::type_name::<ActionSettings>();
}
