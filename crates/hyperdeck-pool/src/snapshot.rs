use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::message::DeckMessage;

/// One timeline or disk clip row.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClipRow {
    pub id: String,
    pub name: String,
}

/// Fields received from the deck. Queries and notifications both land here.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeckSnapshot {
    pub connected: bool,
    pub model: String,
    pub protocol_version: String,
    pub transport: BTreeMap<String, String>,
    pub slot: BTreeMap<String, String>,
    pub config: BTreeMap<String, String>,
    pub remote: BTreeMap<String, String>,
    pub playrange: BTreeMap<String, String>,
    pub clips: Vec<ClipRow>,
    pub disk_clips: Vec<ClipRow>,
    pub dropped_frames: u64,
    pub uptime_seconds: u64,
}

impl DeckSnapshot {
    /// Transport field, or an empty string.
    pub fn transport_value(&self, key: &str) -> &str {
        self.transport.get(key).map(String::as_str).unwrap_or("")
    }

    /// Configuration field, or an empty string.
    pub fn config_value(&self, key: &str) -> &str {
        self.config.get(key).map(String::as_str).unwrap_or("")
    }

    /// Slot field, or an empty string.
    pub fn slot_value(&self, key: &str) -> &str {
        self.slot.get(key).map(String::as_str).unwrap_or("")
    }

    /// Remote field, or an empty string.
    pub fn remote_value(&self, key: &str) -> &str {
        self.remote.get(key).map(String::as_str).unwrap_or("")
    }

    /// `status` token from the latest transport info.
    pub fn status(&self) -> &str {
        self.transport_value("status")
    }

    /// Playback speed in percent.
    pub fn speed(&self) -> i32 {
        self.transport_value("speed").parse().unwrap_or(0)
    }

    /// Timeline position in frames.
    pub fn timeline(&self) -> u32 {
        self.transport_value("timeline").parse().unwrap_or(0)
    }

    /// Name of the current clip id, when the clip list has it.
    pub fn clip_name(&self) -> String {
        let id = self.transport_value("clip id").trim();
        self.clips
            .iter()
            .find(|clip| clip.id == id)
            .map(|clip| clip.name.clone())
            .unwrap_or_default()
    }

    /// `true` when a play range is currently stored.
    pub fn playrange_active(&self) -> bool {
        self.playrange.values().any(|value| {
            let value = value.trim();
            !value.is_empty() && value != "none" && value != "0"
        })
    }

    /// Fold one server message into the cache. `true` when a field changed.
    pub fn apply(&mut self, message: &DeckMessage) -> bool {
        let previous = self.clone();
        self.fold(message);
        self != &previous
    }

    fn fold(&mut self, message: &DeckMessage) {
        let text = message.text.to_ascii_lowercase();
        if message.code == 500 || text.contains("connection info") {
            if let Some(model) = message.get("model") {
                self.model = model.to_string();
            }
            if let Some(version) = message.get("protocol version") {
                self.protocol_version = version.to_string();
            }
        }
        if message.code == 208 || message.code == 508 || text.contains("transport info") {
            replace_map(&mut self.transport, message);
        }
        if text.contains("display timecode") || text.contains("timeline position") {
            merge_map(&mut self.transport, message);
        }
        if message.code == 202 || message.code == 502 || text.contains("slot info") {
            replace_map(&mut self.slot, message);
        }
        if message.code == 211
            || message.code == 511
            || text.contains("configuration")
            || text.contains("play option")
            || text.contains("play on startup")
            || text.contains("dynamic range")
            || text.contains("nas")
            || text.contains("external drive")
            || text.contains("cache")
        {
            merge_map(&mut self.config, message);
        }
        if message.code == 210 || message.code == 510 || text.contains("remote") {
            merge_map(&mut self.remote, message);
        }
        if text.contains("playrange") {
            if message.params.is_empty() && message.body.trim().is_empty() {
                self.playrange.clear();
            } else {
                replace_map(&mut self.playrange, message);
            }
        }
        if message.code == 205 || text.contains("clips") {
            let rows = parse_rows(&message.body);
            if !rows.is_empty() {
                self.clips = rows;
            }
        }
        if message.code == 206 || text.contains("disk list") {
            let rows = parse_rows(&message.body);
            if !rows.is_empty() {
                self.disk_clips = rows;
            }
        }
        if text.contains("dropped")
            && let Some(value) = message
                .get("dropped frames")
                .or_else(|| message.get("count"))
                .or_else(|| message.params.first().map(|(_, value)| value.as_str()))
            && let Ok(count) = value.parse()
        {
            self.dropped_frames = count;
        }
        if let Some(uptime) = message.get("uptime")
            && let Ok(seconds) = uptime.parse()
        {
            self.uptime_seconds = seconds;
        }
    }
}

fn replace_map(map: &mut BTreeMap<String, String>, message: &DeckMessage) {
    if message.params.is_empty() {
        return;
    }
    map.clear();
    merge_map(map, message);
}

fn merge_map(map: &mut BTreeMap<String, String>, message: &DeckMessage) {
    for (key, value) in &message.params {
        map.insert(key.clone(), value.clone());
    }
}

fn clip_name_from_row(rest: &str) -> String {
    let tokens: Vec<&str> = rest.split_whitespace().collect();
    if tokens.len() >= 3
        && is_timecode(tokens[tokens.len() - 1])
        && is_timecode(tokens[tokens.len() - 2])
    {
        return tokens[..tokens.len() - 2].join(" ");
    }
    tokens.first().copied().unwrap_or("").to_string()
}

fn is_timecode(token: &str) -> bool {
    let mut parts = token.split([':', ';']);
    (0..4).all(|_| {
        parts
            .next()
            .is_some_and(|part| part.len() == 2 && part.chars().all(|ch| ch.is_ascii_digit()))
    }) && parts.next().is_none()
}

fn parse_rows(body: &str) -> Vec<ClipRow> {
    let mut rows = Vec::new();
    for line in body.lines() {
        let line = line.trim().trim_end_matches('\r');
        if line.is_empty() {
            continue;
        }
        let Some((id, rest)) = line.split_once(':') else {
            continue;
        };
        let id = id.trim();
        if id.is_empty() || !id.chars().all(|ch| ch.is_ascii_digit()) {
            continue;
        }
        rows.push(ClipRow {
            id: id.to_string(),
            name: clip_name_from_row(rest),
        });
    }
    rows
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transport_fields_replace_the_previous_status() {
        let mut snapshot = DeckSnapshot::default();
        snapshot.apply(&DeckMessage {
            code: 208,
            text: "transport info".into(),
            body: String::new(),
            params: vec![
                ("status".into(), "play".into()),
                ("speed".into(), "100".into()),
                ("loop".into(), "true".into()),
            ],
        });
        snapshot.apply(&DeckMessage {
            code: 508,
            text: "transport info".into(),
            body: String::new(),
            params: vec![
                ("status".into(), "stopped".into()),
                ("speed".into(), "0".into()),
            ],
        });
        assert_eq!(snapshot.status(), "stopped");
        assert_eq!(snapshot.transport_value("loop"), "");
        assert_eq!(snapshot.speed(), 0);
    }

    #[test]
    fn display_timecode_notification_updates_the_cached_timecode() {
        let mut snapshot = DeckSnapshot::default();
        snapshot.apply(&DeckMessage {
            code: 208,
            text: "transport info".into(),
            body: String::new(),
            params: vec![("display timecode".into(), "00:00:00:00".into())],
        });
        snapshot.apply(&DeckMessage {
            code: 513,
            text: "display timecode".into(),
            body: String::new(),
            params: vec![("display timecode".into(), "01:02:03:04".into())],
        });
        assert_eq!(snapshot.transport_value("display timecode"), "01:02:03:04");
    }

    #[test]
    fn clip_rows_keep_the_full_name_before_the_timecodes() {
        let mut snapshot = DeckSnapshot::default();
        snapshot.apply(&DeckMessage {
            code: 205,
            text: "clips info".into(),
            body: "clip count: 2\r\n1: My Clip 00:00:00:00 00:00:10:00\r\n2: other.mov 00:00:10:00 00:00:05:00\r\n".into(),
            params: Vec::new(),
        });
        assert_eq!(snapshot.clips[0].name, "My Clip");
        assert_eq!(snapshot.clips[1].name, "other.mov");
        snapshot.apply(&DeckMessage {
            code: 206,
            text: "disk list".into(),
            body: "1: clip.mov H.264 1920x1080 00:00:10:00\r\n".into(),
            params: Vec::new(),
        });
        assert_eq!(snapshot.disk_clips[0].name, "clip.mov");
    }
}
