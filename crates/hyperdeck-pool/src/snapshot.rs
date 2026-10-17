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
            apply_transport(&mut self.transport, message);
        }
        if text.contains("display timecode") || text.contains("timeline position") {
            merge_map(&mut self.transport, message);
        }
        if message.code == 202 || message.code == 502 || text.contains("slot info") {
            apply_slot(&mut self.slot, message);
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

/// `208 transport info` replaces the transport. A `508` notice replaces the
/// fields it carries, but an omitted playback timecode stays. That timecode
/// arrives on its own notice, and clearing it here blanks the key until then.
fn apply_transport(map: &mut BTreeMap<String, String>, message: &DeckMessage) {
    if message.params.is_empty() {
        return;
    }
    if message.code == 208 {
        replace_map(map, message);
        return;
    }
    let kept: Vec<(String, String)> = [
        "display timecode",
        "timecode",
        "timeline position",
        "timeline",
    ]
    .into_iter()
    .filter_map(|key| {
        if message.get(key).is_some() {
            None
        } else {
            map.get(key).map(|value| (key.to_string(), value.clone()))
        }
    })
    .collect();
    replace_map(map, message);
    for (key, value) in kept {
        map.insert(key, value);
    }
}

/// `202 slot info` replaces the slot. A `502` notice only carries the fields
/// the deck included, so an omitted recording time stays at the last value.
fn apply_slot(map: &mut BTreeMap<String, String>, message: &DeckMessage) {
    if message.params.is_empty() {
        return;
    }
    if message.code == 202 {
        replace_map(map, message);
        return;
    }
    merge_map(map, message);
    let status = message.get("status").unwrap_or("");
    if matches!(status, "empty" | "mounting" | "error") {
        for key in [
            "recording time",
            "remaining size",
            "total size",
            "volume name",
        ] {
            if message.get(key).is_none() {
                map.remove(key);
            }
        }
    }
}

fn merge_map(map: &mut BTreeMap<String, String>, message: &DeckMessage) {
    for (key, value) in &message.params {
        map.insert(key.clone(), value.clone());
    }
}

fn clip_name_from_row(rest: &str) -> String {
    let tokens: Vec<&str> = rest.split_whitespace().collect();
    if tokens.is_empty() {
        return String::new();
    }
    // Version 1: `{name} {start} {duration}`.
    if tokens.len() >= 3
        && is_timecode(tokens[tokens.len() - 1])
        && is_timecode(tokens[tokens.len() - 2])
    {
        return tokens[..tokens.len() - 2].join(" ");
    }
    // Version 2 and 3: timecodes first, then the file name.
    let name_at = tokens.iter().position(|token| !is_timecode(token));
    if let Some(index) = name_at
        && index > 0
    {
        return tokens[index..].join(" ");
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
    fn transport_notice_keeps_the_playback_timecode_it_omits() {
        let mut snapshot = DeckSnapshot::default();
        snapshot.apply(&DeckMessage {
            code: 208,
            text: "transport info".into(),
            body: String::new(),
            params: vec![
                ("status".into(), "play".into()),
                ("speed".into(), "100".into()),
                ("display timecode".into(), "00:01:02:03".into()),
                ("timecode".into(), "00:01:02:03".into()),
                ("timeline".into(), "1500".into()),
                ("loop".into(), "true".into()),
            ],
        });
        snapshot.apply(&DeckMessage {
            code: 508,
            text: "transport info".into(),
            body: String::new(),
            params: vec![
                ("status".into(), "play".into()),
                ("speed".into(), "200".into()),
            ],
        });
        assert_eq!(snapshot.speed(), 200);
        assert_eq!(snapshot.transport_value("loop"), "");
        assert_eq!(snapshot.transport_value("display timecode"), "00:01:02:03");
        assert_eq!(snapshot.transport_value("timecode"), "00:01:02:03");
        assert_eq!(snapshot.transport_value("timeline"), "1500");
        snapshot.apply(&DeckMessage {
            code: 513,
            text: "display timecode".into(),
            body: String::new(),
            params: vec![("display timecode".into(), "00:01:02:04".into())],
        });
        assert_eq!(snapshot.transport_value("display timecode"), "00:01:02:04");
        assert_eq!(snapshot.transport_value("timecode"), "00:01:02:03");
        snapshot.apply(&DeckMessage {
            code: 208,
            text: "transport info".into(),
            body: String::new(),
            params: vec![
                ("status".into(), "stopped".into()),
                ("speed".into(), "0".into()),
            ],
        });
        assert_eq!(snapshot.transport_value("display timecode"), "");
        assert_eq!(snapshot.transport_value("timecode"), "");
        assert_eq!(snapshot.transport_value("timeline"), "");
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
        snapshot.apply(&DeckMessage {
            code: 205,
            text: "clips info".into(),
            body: "1: 00:00:00:00 00:00:10:00 00:00:01:00 00:00:08:00 folder/My Clip.mov\r\n"
                .into(),
            params: Vec::new(),
        });
        assert_eq!(snapshot.clips[0].name, "folder/My Clip.mov");
    }

    #[test]
    fn slot_notice_keeps_recording_time_the_deck_already_sent() {
        let mut snapshot = DeckSnapshot::default();
        snapshot.apply(&DeckMessage {
            code: 202,
            text: "slot info".into(),
            body: String::new(),
            params: vec![
                ("slot id".into(), "1".into()),
                ("status".into(), "mounted".into()),
                ("recording time".into(), "3600".into()),
                ("remaining size".into(), "1000".into()),
            ],
        });
        snapshot.apply(&DeckMessage {
            code: 502,
            text: "slot info".into(),
            body: String::new(),
            params: vec![
                ("slot id".into(), "1".into()),
                ("status".into(), "mounted".into()),
                ("volume name".into(), "Media".into()),
            ],
        });
        assert_eq!(snapshot.slot_value("recording time"), "3600");
        assert_eq!(snapshot.slot_value("remaining size"), "1000");
        assert_eq!(snapshot.slot_value("volume name"), "Media");
        snapshot.apply(&DeckMessage {
            code: 502,
            text: "slot info".into(),
            body: String::new(),
            params: vec![("recording time".into(), "3599".into())],
        });
        assert_eq!(snapshot.slot_value("recording time"), "3599");
        assert_eq!(snapshot.slot_value("remaining size"), "1000");
    }

    #[test]
    fn empty_slot_notice_drops_the_previous_recording_time() {
        let mut snapshot = DeckSnapshot::default();
        snapshot.apply(&DeckMessage {
            code: 202,
            text: "slot info".into(),
            body: String::new(),
            params: vec![
                ("status".into(), "mounted".into()),
                ("recording time".into(), "3600".into()),
                ("remaining size".into(), "1000".into()),
                ("volume name".into(), "Media".into()),
            ],
        });
        snapshot.apply(&DeckMessage {
            code: 502,
            text: "slot info".into(),
            body: String::new(),
            params: vec![
                ("slot id".into(), "1".into()),
                ("status".into(), "empty".into()),
            ],
        });
        assert_eq!(snapshot.slot_value("recording time"), "");
        assert_eq!(snapshot.slot_value("remaining size"), "");
        assert_eq!(snapshot.slot_value("volume name"), "");
        assert_eq!(snapshot.slot_value("status"), "empty");
    }

    #[test]
    fn slot_query_replaces_the_previous_slot() {
        let mut snapshot = DeckSnapshot::default();
        snapshot.apply(&DeckMessage {
            code: 202,
            text: "slot info".into(),
            body: String::new(),
            params: vec![
                ("recording time".into(), "3600".into()),
                ("remaining size".into(), "1000".into()),
            ],
        });
        snapshot.apply(&DeckMessage {
            code: 202,
            text: "slot info".into(),
            body: String::new(),
            params: vec![
                ("slot id".into(), "2".into()),
                ("status".into(), "mounted".into()),
                ("recording time".into(), "10".into()),
            ],
        });
        assert_eq!(snapshot.slot_value("recording time"), "10");
        assert_eq!(snapshot.slot_value("remaining size"), "");
        assert_eq!(snapshot.slot_value("slot id"), "2");
    }
}
