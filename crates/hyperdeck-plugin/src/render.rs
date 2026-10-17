use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use serde_json::{Value, json};

use crate::kind::ActionKind;

/// How one deck contributes to a key.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SegmentState {
    /// The action matches on this deck.
    Active,
    /// Connected, and the action does not match.
    Off,
    /// The deck rejected the last command.
    Error,
    /// The deck is not connected.
    Disconnected,
}

/// One colored band.
#[derive(Clone, Debug)]
pub struct Segment {
    /// Instance name, used for the initial on the band.
    pub label: String,
    /// Instance color, or the active color when the band is lit.
    pub color: String,
    /// Light, idle, error, or disconnected.
    pub state: SegmentState,
}

/// Touchscreen layout sized for a timecode or a clip name.
pub const LAYOUT_DIAL: &str = "layouts/dial.json";
/// Custom layout for two to four decks.
pub const LAYOUT_MULTI: &str = "layouts/dial-multi.json";

/// `0` when every band is active. Stream Deck state 0 is the lit state.
pub fn hardware_state(states: impl IntoIterator<Item = SegmentState>) -> i32 {
    let mut any = false;
    let mut all_active = true;
    for state in states {
        any = true;
        if state != SegmentState::Active {
            all_active = false;
        }
    }
    if any && all_active { 0 } else { 1 }
}

/// SVG data URL for a key, with one band per deck.
pub fn key_image(kind: ActionKind, segments: &[Segment], foreground: &str) -> String {
    let strip = if segments.is_empty() {
        0.0
    } else if segments.len() <= 6 {
        22.0
    } else {
        10.0
    };
    let width = if segments.is_empty() {
        0.0
    } else {
        144.0 / segments.len() as f32
    };
    let mut bars = String::new();
    for (index, segment) in segments.iter().enumerate() {
        let x = width * index as f32;
        let fill = fill_for(segment);
        let y = 144.0 - strip;
        bars.push_str(&format!(
            r#"<rect x="{x:.2}" y="{y:.2}" width="{width:.2}" height="{strip:.2}" fill="{fill}"/>"#
        ));
        if segments.len() <= 6 && strip > 0.0 {
            let label = escape(&initial(&segment.label));
            let tx = x + width / 2.0;
            bars.push_str(&format!(
                r#"<text x="{tx:.2}" y="138" text-anchor="middle" font-size="12" font-family="sans-serif" font-weight="700" fill="{foreground}">{label}</text>"#
            ));
        }
    }
    let icon = icon_path(kind);
    let background = "#1b1f27";
    let svg = format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="144" height="144" viewBox="0 0 144 144"><rect width="144" height="144" rx="18" fill="{background}"/><g fill="none" stroke="{foreground}" stroke-width="6" stroke-linecap="round" stroke-linejoin="round" transform="translate(40 28)">{icon}</g>{bars}</svg>"#
    );
    format!(
        "data:image/svg+xml;base64,{}",
        STANDARD.encode(svg.as_bytes())
    )
}

/// Layout id and `setFeedback` payload.
///
/// One deck uses `layouts/dial.json`. Two or more decks use `layouts/dial-multi.json`.
pub fn dial_feedback(
    kind: ActionKind,
    segments: &[Segment],
    captions: &[String],
    levels: &[u8],
    foreground: &str,
) -> (String, Value) {
    let icon = dial_icon(kind, foreground);
    if segments.len() <= 1 {
        let segment = segments.first();
        let caption = captions.first().map(String::as_str).unwrap_or("");
        let level = levels.first().copied().unwrap_or(0);
        let (title, value) = dial_text(kind, caption);
        (
            LAYOUT_DIAL.to_string(),
            json!({
                "title": title,
                "value": value,
                "icon": icon,
                "indicator": indicator(segment, level),
            }),
        )
    } else {
        let title = captions
            .iter()
            .take(4)
            .map(|caption| {
                if kind == ActionKind::ClipList {
                    let (name, timecode) = dial_text(kind, caption);
                    clip_line(&name, &timecode)
                } else {
                    caption.clone()
                }
            })
            .filter(|caption| !caption.is_empty() && caption != kind.dial_heading())
            .collect::<Vec<_>>()
            .join(" · ");
        let mut payload = json!({ "title": title });
        for index in 0..4 {
            payload[format!("indicator{}", index + 1)] = match segments.get(index) {
                Some(segment) => indicator(Some(segment), levels.get(index).copied().unwrap_or(0)),
                None => json!({ "value": 0, "enabled": false }),
            };
        }
        (LAYOUT_MULTI.to_string(), payload)
    }
}

fn dial_text(kind: ActionKind, caption: &str) -> (String, String) {
    if kind == ActionKind::ClipList {
        let mut parts = caption.split('\u{1e}');
        let id = parts.next().unwrap_or("");
        let name = parts.next().unwrap_or("");
        let timecode = parts.next().unwrap_or("");
        let title = if !name.is_empty() {
            name.to_string()
        } else if !id.is_empty() {
            id.to_string()
        } else {
            kind.dial_heading().to_string()
        };
        return (title, timecode.to_string());
    }
    (kind.dial_heading().to_string(), caption.to_string())
}

fn clip_line(name: &str, timecode: &str) -> String {
    match (name.is_empty(), timecode.is_empty()) {
        (false, false) => format!("{name} {timecode}"),
        (false, true) => name.to_string(),
        (true, false) => timecode.to_string(),
        (true, true) => String::new(),
    }
}

fn indicator(segment: Option<&Segment>, level: u8) -> Value {
    let color = segment
        .map(|item| item.color.clone())
        .unwrap_or_else(|| "#4c8dff".into());
    let enabled = segment.is_some_and(|item| item.state != SegmentState::Disconnected);
    json!({
        "value": level,
        "enabled": enabled,
        "bar_fill_c": color,
        "range": { "min": 0, "max": 100 },
    })
}

fn dial_icon(kind: ActionKind, foreground: &str) -> String {
    let path = icon_path(kind);
    let svg = format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="48" height="48" viewBox="0 0 64 64"><g fill="none" stroke="{foreground}" stroke-width="5" stroke-linecap="round" stroke-linejoin="round">{path}</g></svg>"#
    );
    format!(
        "data:image/svg+xml;base64,{}",
        STANDARD.encode(svg.as_bytes())
    )
}

fn fill_for(segment: &Segment) -> String {
    match segment.state {
        SegmentState::Active => segment.color.clone(),
        SegmentState::Off => "#2a3544".into(),
        SegmentState::Error => "#c43c3c".into(),
        SegmentState::Disconnected => "#3a3a3a".into(),
    }
}

fn initial(label: &str) -> String {
    label
        .chars()
        .find(|char| !char.is_whitespace())
        .map(|char| char.to_uppercase().to_string())
        .unwrap_or_else(|| "?".into())
}

fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn icon_path(kind: ActionKind) -> &'static str {
    match kind {
        ActionKind::Play | ActionKind::PlaySpeed => r#"<path d="M22 16v32l24-16z"/>"#,
        ActionKind::Record | ActionKind::RecordNamed | ActionKind::RecordCache => {
            r#"<circle cx="32" cy="32" r="14"/>"#
        }
        ActionKind::Stop => r#"<rect x="18" y="18" width="28" height="28"/>"#,
        ActionKind::Preview => r#"<rect x="12" y="18" width="40" height="28" rx="4"/>"#,
        ActionKind::Loop | ActionKind::SingleClip | ActionKind::PlayRange => {
            r#"<path d="M18 32a14 14 0 1 0 4-10M18 16v10h10"/>"#
        }
        ActionKind::Slot | ActionKind::SlotUnblock | ActionKind::ExternalDrive => {
            r#"<rect x="14" y="16" width="36" height="28" rx="4"/>"#
        }
        ActionKind::Clip
        | ActionKind::ClipStep
        | ActionKind::ClipList
        | ActionKind::ClipsAdd
        | ActionKind::ClipsRemove
        | ActionKind::ClipsRebuild
        | ActionKind::ClipsClear => r#"<path d="M16 18h32M16 32h32M16 46h20"/>"#,
        ActionKind::GoTo | ActionKind::Position | ActionKind::Jog => {
            r#"<path d="M16 32h32M40 22l8 10-8 10"/>"#
        }
        ActionKind::Remote | ActionKind::PlayOnStartup => {
            r#"<path d="M20 40V24h24v16M28 24V16h8v8"/>"#
        }
        ActionKind::StopMode
        | ActionKind::RecordTrigger
        | ActionKind::VideoInput
        | ActionKind::AudioInput
        | ActionKind::FileFormat
        | ActionKind::AudioCodec
        | ActionKind::TimecodeInput
        | ActionKind::TimecodeOutput
        | ActionKind::DynamicRange
        | ActionKind::ReferenceSource
        | ActionKind::Genlock
        | ActionKind::ReferenceLock => r#"<path d="M16 20h32M16 32h24M16 44h16"/>"#,
        ActionKind::AppendTimestamp
        | ActionKind::UsbSpill
        | ActionKind::TimecodePreset
        | ActionKind::RecordPrefix
        | ActionKind::Slate => r#"<path d="M18 16h20l8 8v24H18z"/>"#,
        ActionKind::Nas => r#"<path d="M14 22h36v8H14zM14 34h36v8H14z"/>"#,
        ActionKind::DroppedFrames | ActionKind::Remaining | ActionKind::Status => {
            r#"<path d="M16 44V28M32 44V18M48 44V26"/>"#
        }
        ActionKind::Identify => r#"<circle cx="32" cy="32" r="14"/><path d="M32 24v10"/>"#,
        ActionKind::Raw | ActionKind::RawDial => {
            r#"<path d="M20 16l-10 16 10 16M44 16l10 16-10 16"/>"#
        }
        ActionKind::Shuttle => r#"<path d="M14 32h36M18 22l-8 10 8 10M46 22l8 10-8 10"/>"#,
        ActionKind::AudioChannels => r#"<path d="M16 28h8l10-8v24l-10-8h-8zM42 24v16"/>"#,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hardware_state_is_on_only_when_every_segment_is_active() {
        assert_eq!(
            hardware_state([SegmentState::Active, SegmentState::Active]),
            0
        );
        assert_eq!(hardware_state([SegmentState::Active, SegmentState::Off]), 1);
        assert_eq!(hardware_state([SegmentState::Error]), 1);
        assert_eq!(hardware_state([]), 1);
    }

    #[test]
    fn dial_feedback_uses_b1_for_one_deck_and_multi_for_several() {
        let one = Segment {
            label: "A".into(),
            color: "#4c8dff".into(),
            state: SegmentState::Active,
        };
        let (layout, payload) = dial_feedback(
            ActionKind::Shuttle,
            std::slice::from_ref(&one),
            &["100%".into()],
            &[2],
            "#f4f7fb",
        );
        assert_eq!(layout, LAYOUT_DIAL);
        assert_eq!(payload["title"], "Shuttle");
        assert_eq!(payload["value"], "100%");
        assert_eq!(payload["indicator"]["value"], 2);
        assert!(payload.get("indicator1").is_none());

        let two = [
            one.clone(),
            Segment {
                label: "B".into(),
                color: "#ef5b5b".into(),
                state: SegmentState::Off,
            },
        ];
        let (layout, payload) = dial_feedback(
            ActionKind::Shuttle,
            &two,
            &["100%".into(), "0%".into()],
            &[2, 0],
            "#f4f7fb",
        );
        assert_eq!(layout, LAYOUT_MULTI);
        assert_eq!(payload["title"], "100% · 0%");
        assert_eq!(payload["indicator1"]["value"], 2);
        assert_eq!(payload["indicator2"]["value"], 0);
        assert_eq!(payload["indicator2"]["bar_fill_c"], "#ef5b5b");
        assert!(payload.get("indicator").is_none());

        let (layout, payload) = dial_feedback(
            ActionKind::ClipList,
            std::slice::from_ref(&one),
            &["3\u{1e}interview.mov\u{1e}01:02:03:04".into()],
            &[100],
            "#f4f7fb",
        );
        assert_eq!(layout, LAYOUT_DIAL);
        assert_eq!(payload["title"], "interview.mov");
        assert_eq!(payload["value"], "01:02:03:04");

        let (layout, payload) = dial_feedback(
            ActionKind::ClipList,
            &two,
            &[
                "3\u{1e}interview.mov\u{1e}01:02:03".into(),
                "1\u{1e}slate.mov\u{1e}00:10:00".into(),
            ],
            &[100, 50],
            "#f4f7fb",
        );
        assert_eq!(layout, LAYOUT_MULTI);
        assert_eq!(
            payload["title"],
            "interview.mov 01:02:03 · slate.mov 00:10:00"
        );
    }
}
