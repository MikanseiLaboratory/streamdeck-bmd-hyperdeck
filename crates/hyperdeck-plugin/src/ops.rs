use hyperdeck_pool::DeckSnapshot;
use hyperdeck_tokio::core::types::{Timecode, VideoFormat};

use crate::contracts::ActionParams;
use crate::kind::{ActionKind, PressKind};

/// What the user did on a key or a dial.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Gesture {
    /// Key down/up, or a dial push.
    Press(PressKind),
    /// Encoder rotation. Positive ticks are clockwise.
    Rotate(i32),
    /// Encoder touch screen tap.
    Touch,
}

/// Text and light derived from one deck snapshot.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Face {
    /// The segment should light.
    pub active: bool,
    /// Title fragment for this deck.
    pub title: String,
    /// Bar position from 0 to 100.
    pub indicator: u8,
}

/// Ethernet commands for one gesture. Each string already ends in CRLF.
pub fn commands(
    kind: ActionKind,
    params: &ActionParams,
    snap: &DeckSnapshot,
    gesture: Gesture,
) -> Vec<String> {
    match gesture {
        Gesture::Rotate(ticks) => rotate_commands(kind, params, snap, ticks),
        Gesture::Touch => touch_commands(kind, params, snap),
        Gesture::Press(press) => press_commands(kind, params, snap, press),
    }
}

/// Title, light, and bar for the current snapshot.
pub fn face(kind: ActionKind, params: &ActionParams, snap: &DeckSnapshot) -> Face {
    let status = snap.status();
    let timecode = display_timecode(snap);
    match kind {
        ActionKind::Play => Face {
            active: playing(status),
            title: format!("{}% {timecode}", snap.speed()),
            indicator: percent_of(snap.speed().unsigned_abs(), 5000),
        },
        ActionKind::Record => Face {
            active: status == "record",
            title: recording_time(snap),
            indicator: 0,
        },
        ActionKind::Stop => Face {
            active: status == "stopped" || status == "stop",
            title: status.to_string(),
            indicator: 0,
        },
        ActionKind::Preview => Face {
            active: status == "preview",
            title: status.to_string(),
            indicator: 0,
        },
        ActionKind::Loop => flag_face(snap.transport_value("loop"), "Loop"),
        ActionKind::SingleClip => flag_face(snap.transport_value("single clip"), "Single"),
        ActionKind::Slot => {
            let current = snap.transport_value("slot id");
            let wanted = if params.slot_id.trim().is_empty() {
                current
            } else {
                params.slot_id.trim()
            };
            Face {
                active: !wanted.is_empty() && current == wanted,
                title: format!(
                    "{} {}",
                    snap.slot_value("slot name"),
                    snap.slot_value("recording time")
                )
                .trim()
                .to_string(),
                indicator: 0,
            }
        }
        ActionKind::Clip | ActionKind::ClipStep => Face {
            active: !params.clip_id.trim().is_empty()
                && snap.transport_value("clip id") == params.clip_id.trim(),
            title: clip_title(snap),
            indicator: 0,
        },
        ActionKind::ClipList => Face {
            active: !current_clip_id(snap).is_empty(),
            title: clip_list_caption(snap),
            indicator: clip_indicator(snap),
        },
        ActionKind::PlayRange => Face {
            active: snap.playrange_active(),
            title: playrange_title(snap),
            indicator: 0,
        },
        ActionKind::Remote => flag_face(snap.remote_value("enable"), "Remote"),
        ActionKind::PlayOnStartup => flag_face(snap.config_value("enable"), "Startup"),
        ActionKind::StopMode => value_face(snap.config_value("stop mode"), &params.value),
        ActionKind::RecordTrigger => value_face(snap.config_value("record trigger"), &params.value),
        ActionKind::VideoInput => value_face(snap.config_value("video input"), &params.value),
        ActionKind::AudioInput => value_face(snap.config_value("audio input"), &params.value),
        ActionKind::FileFormat => value_face(snap.config_value("file format"), &params.value),
        ActionKind::AudioCodec => value_face(snap.config_value("audio codec"), &params.value),
        ActionKind::TimecodeInput => value_face(snap.config_value("timecode input"), &params.value),
        ActionKind::TimecodeOutput => {
            value_face(snap.config_value("timecode output"), &params.value)
        }
        ActionKind::DynamicRange => {
            let key = if params.mode == "record" {
                "record override"
            } else {
                "playback override"
            };
            value_face(snap.config_value(key), &params.value)
        }
        ActionKind::RecordCache => flag_face(snap.config_value("record cache"), "Cache"),
        ActionKind::AppendTimestamp => {
            flag_face(snap.config_value("append timestamp"), "Timestamp")
        }
        ActionKind::UsbSpill => flag_face(snap.config_value("usb spill"), "USB"),
        ActionKind::ReferenceSource => {
            value_face(snap.config_value("reference source"), &params.value)
        }
        ActionKind::Genlock => flag_face(snap.config_value("genlock resync"), "Genlock"),
        ActionKind::ExternalDrive => {
            let current = snap.config_value("selected");
            let wanted = params.value.trim();
            Face {
                active: !wanted.is_empty() && current == wanted,
                title: current.to_string(),
                indicator: 0,
            }
        }
        ActionKind::Nas => {
            let current =
                first_nonempty(&[snap.config_value("url"), snap.config_value("selected")]);
            Face {
                active: !params.nas_url.trim().is_empty() && current == params.nas_url.trim(),
                title: current.to_string(),
                indicator: 0,
            }
        }
        ActionKind::DroppedFrames => Face {
            active: snap.dropped_frames > 0,
            title: snap.dropped_frames.to_string(),
            indicator: 0,
        },
        ActionKind::Remaining => {
            let seconds = recording_seconds(snap);
            let threshold = params.threshold_seconds;
            Face {
                active: seconds > 0 && seconds <= u64::from(threshold),
                title: format!("{seconds}s {}", snap.slot_value("remaining size")),
                indicator: 0,
            }
        }
        ActionKind::ReferenceLock => flag_face(snap.transport_value("reference locked"), "Ref"),
        ActionKind::GoTo
        | ActionKind::Identify
        | ActionKind::RecordNamed
        | ActionKind::ClipsAdd
        | ActionKind::ClipsRemove
        | ActionKind::ClipsRebuild
        | ActionKind::ClipsClear
        | ActionKind::SlotUnblock
        | ActionKind::TimecodePreset
        | ActionKind::RecordPrefix
        | ActionKind::Raw => Face {
            active: false,
            title: timecode,
            indicator: 0,
        },
        ActionKind::Slate => Face {
            active: false,
            title: format!(
                "{} {} {}",
                params.slate_reel, params.slate_scene, params.slate_take
            )
            .trim()
            .to_string(),
            indicator: 0,
        },
        ActionKind::Status => Face {
            active: false,
            title: status_title(params, snap),
            indicator: 0,
        },
        ActionKind::Shuttle => {
            let speed = snap.speed();
            Face {
                active: status == "shuttle",
                title: format!("{speed}%"),
                indicator: percent_of(speed.unsigned_abs(), 5000),
            }
        }
        ActionKind::Jog => Face {
            active: status == "jog",
            title: timecode,
            indicator: 0,
        },
        ActionKind::PlaySpeed => {
            let speed = snap.speed();
            Face {
                active: playing(status),
                title: format_play_speed(speed),
                indicator: percent_of(speed.unsigned_abs().min(5000), 5000),
            }
        }
        ActionKind::Position => Face {
            active: false,
            title: timecode,
            indicator: position_indicator(snap),
        },
        ActionKind::AudioChannels => {
            let count = snap.config_value("audio input channels").trim();
            let indicator = count
                .parse::<u32>()
                .map(|value| percent_of(value.min(64), 64))
                .unwrap_or(0);
            Face {
                active: false,
                title: count.to_string(),
                indicator,
            }
        }
        ActionKind::RawDial => Face {
            active: false,
            title: "Raw".into(),
            indicator: 0,
        },
    }
}

fn press_commands(
    kind: ActionKind,
    params: &ActionParams,
    snap: &DeckSnapshot,
    press: PressKind,
) -> Vec<String> {
    if matches!(
        kind,
        ActionKind::DroppedFrames
            | ActionKind::Remaining
            | ActionKind::ReferenceLock
            | ActionKind::Status
    ) {
        return Vec::new();
    }
    match kind {
        ActionKind::Play => {
            if playing(snap.status()) {
                vec![line("stop", &[])]
            } else {
                vec![play_line(params)]
            }
        }
        ActionKind::Record => {
            if press == PressKind::Long {
                vec![line("record spill", &[])]
            } else if snap.status() == "record" {
                vec![line("stop", &[])]
            } else {
                vec![record_line(params)]
            }
        }
        ActionKind::Stop | ActionKind::Shuttle | ActionKind::Jog | ActionKind::Position => {
            vec![line("stop", &[])]
        }
        ActionKind::ClipList => {
            let id = snap.transport_value("clip id").trim();
            if id.is_empty() {
                vec![line("play", &[])]
            } else {
                vec![line("play", &[("clip id", id)])]
            }
        }
        ActionKind::Preview => vec![line(
            "preview",
            &[(
                "enable",
                if snap.status() == "preview" {
                    "false"
                } else {
                    "true"
                },
            )],
        )],
        ActionKind::Loop => vec![line(
            "play",
            &[("loop", flip(snap.transport_value("loop")))],
        )],
        ActionKind::SingleClip => vec![line(
            "play",
            &[("single clip", flip(snap.transport_value("single clip")))],
        )],
        ActionKind::Slot => slot_select(params),
        ActionKind::Clip => {
            let id = params.clip_id.trim();
            if id.is_empty() {
                return Vec::new();
            }
            if press == PressKind::Long {
                vec![line("play", &[("clip id", id)])]
            } else {
                vec![line("goto", &[("clip id", id)])]
            }
        }
        ActionKind::ClipStep => {
            let step = if params.direction == "previous" {
                "-1"
            } else {
                "+1"
            };
            vec![line("goto", &[("clip id", step)])]
        }
        ActionKind::PlayRange => {
            if press == PressKind::Long {
                vec![line("playrange clear", &[])]
            } else {
                playrange_set(params)
            }
        }
        ActionKind::GoTo => goto_command(params),
        ActionKind::Remote => vec![line(
            "remote",
            &[("enable", flip(snap.remote_value("enable")))],
        )],
        ActionKind::PlayOnStartup => vec![line(
            "play on startup",
            &[("enable", flip(snap.config_value("enable")))],
        )],
        ActionKind::StopMode => config_value("play option", "stop mode", &params.value),
        ActionKind::RecordTrigger => config_value("configuration", "record trigger", &params.value),
        ActionKind::VideoInput => config_value("configuration", "video input", &params.value),
        ActionKind::AudioInput => config_value("configuration", "audio input", &params.value),
        ActionKind::FileFormat => config_value("configuration", "file format", &params.value),
        ActionKind::AudioCodec => config_value("configuration", "audio codec", &params.value),
        ActionKind::TimecodeInput => config_value("configuration", "timecode input", &params.value),
        ActionKind::TimecodeOutput => {
            config_value("configuration", "timecode output", &params.value)
        }
        ActionKind::DynamicRange => {
            let key = if params.mode == "record" {
                "record override"
            } else {
                "playback override"
            };
            one_if_value("dynamic range", &[(key, params.value.trim())])
        }
        ActionKind::RecordCache => bool_config("record cache", snap),
        ActionKind::AppendTimestamp => bool_config("append timestamp", snap),
        ActionKind::UsbSpill => bool_config("usb spill", snap),
        ActionKind::ReferenceSource => {
            config_value("configuration", "reference source", &params.value)
        }
        ActionKind::Genlock => bool_config("genlock resync", snap),
        ActionKind::ExternalDrive => {
            one_if_value("external drive", &[("select", params.value.trim())])
        }
        ActionKind::Nas => nas_commands(params),
        ActionKind::Identify => vec![line("identify", &[("enable", "true")])],
        ActionKind::RecordNamed => {
            let name = params.name.trim();
            if name.is_empty() {
                Vec::new()
            } else {
                vec![line("record", &[("name", name)])]
            }
        }
        ActionKind::ClipsAdd => {
            let name = params.name.trim();
            if name.is_empty() {
                Vec::new()
            } else if params.clip_id.trim().is_empty() {
                vec![line("clips add", &[("name", name)])]
            } else {
                vec![line(
                    "clips add",
                    &[("name", name), ("clip id", params.clip_id.trim())],
                )]
            }
        }
        ActionKind::ClipsRemove => {
            let id = params.clip_id.trim();
            if id.is_empty() {
                Vec::new()
            } else {
                vec![line("clips remove", &[("clip id", id)])]
            }
        }
        ActionKind::ClipsRebuild => vec![line("clips rebuild", &[])],
        ActionKind::ClipsClear => {
            if params.confirm {
                vec![line("clips clear", &[])]
            } else {
                Vec::new()
            }
        }
        ActionKind::SlotUnblock => {
            let id = params.slot_id.trim();
            if id.is_empty() {
                vec![line("slot unblock", &[])]
            } else {
                vec![line("slot unblock", &[("slot id", id)])]
            }
        }
        ActionKind::TimecodePreset => {
            config_value("configuration", "timecode preset", &params.timecode)
        }
        ActionKind::RecordPrefix => config_value("configuration", "record prefix", &params.name),
        ActionKind::Slate => slate_block(params),
        ActionKind::Raw => raw_line(&params.raw).into_iter().collect(),
        ActionKind::RawDial => raw_line(&params.raw_push).into_iter().collect(),
        ActionKind::PlaySpeed => {
            if playing(snap.status()) {
                vec![line("stop", &[])]
            } else {
                vec![play_line(params)]
            }
        }
        ActionKind::AudioChannels
        | ActionKind::DroppedFrames
        | ActionKind::Remaining
        | ActionKind::ReferenceLock
        | ActionKind::Status => Vec::new(),
    }
}

fn rotate_commands(
    kind: ActionKind,
    params: &ActionParams,
    snap: &DeckSnapshot,
    ticks: i32,
) -> Vec<String> {
    if ticks == 0 {
        return Vec::new();
    }
    let step = step_of(kind, params);
    let delta = ticks.saturating_mul(step);
    match kind {
        ActionKind::Shuttle => {
            let speed = (i64::from(snap.speed()) + i64::from(delta)).clamp(-5000, 5000);
            vec![line("shuttle", &[("speed", &speed.to_string())])]
        }
        ActionKind::Jog => {
            let frames = delta.unsigned_abs().max(1);
            let Some(tc) = jog_timecode(frames, reported_fps(snap)) else {
                return Vec::new();
            };
            let signed = if delta < 0 {
                format!("-{tc}")
            } else {
                format!("+{tc}")
            };
            vec![line("jog", &[("timecode", &signed)])]
        }
        ActionKind::PlaySpeed => {
            let speed = (i64::from(snap.speed()) + i64::from(delta)).clamp(-5000, 5000);
            vec![line("play", &[("speed", &speed.to_string())])]
        }
        ActionKind::ClipList => {
            vec![line("goto", &[("clip id", &format!("{delta:+}"))])]
        }
        ActionKind::Position => {
            let magnitude = delta.unsigned_abs().max(1);
            let target = if delta < 0 {
                format!("-{magnitude}")
            } else {
                format!("+{magnitude}")
            };
            vec![line("goto", &[("timeline", &target)])]
        }
        ActionKind::AudioChannels => {
            let Some(current) = reported_audio_channels(snap) else {
                return Vec::new();
            };
            let next = (i64::from(current) + i64::from(delta)).clamp(0, 64);
            vec![line(
                "configuration",
                &[("audio input channels", &next.to_string())],
            )]
        }
        ActionKind::RawDial => {
            let text = if ticks > 0 {
                &params.raw_rotate_up
            } else {
                &params.raw_rotate_down
            };
            raw_line(text).into_iter().collect()
        }
        _ => Vec::new(),
    }
}

fn touch_commands(kind: ActionKind, params: &ActionParams, snap: &DeckSnapshot) -> Vec<String> {
    match kind {
        ActionKind::RawDial => raw_line(&params.raw_touch).into_iter().collect(),
        ActionKind::Shuttle | ActionKind::Jog | ActionKind::Position | ActionKind::ClipList => {
            press_commands(ActionKind::Stop, params, snap, PressKind::Single)
        }
        ActionKind::PlaySpeed => press_commands(kind, params, snap, PressKind::Single),
        _ => Vec::new(),
    }
}

fn play_line(params: &ActionParams) -> String {
    let speed = params.speed.clamp(-5000, 5000).to_string();
    line(
        "play",
        &[
            ("speed", speed.as_str()),
            ("loop", bool_text(params.loop_playback)),
            ("single clip", bool_text(params.single_clip)),
        ],
    )
}

fn record_line(params: &ActionParams) -> String {
    let name = params.name.trim();
    if name.is_empty() {
        line("record", &[])
    } else {
        line("record", &[("name", name)])
    }
}

fn slot_select(params: &ActionParams) -> Vec<String> {
    let slot = params.slot_id.trim();
    let device = params.device.trim();
    if !slot.is_empty() {
        vec![line("slot select", &[("slot id", slot)])]
    } else if !device.is_empty() {
        vec![line("slot select", &[("device", device)])]
    } else {
        Vec::new()
    }
}

fn playrange_set(params: &ActionParams) -> Vec<String> {
    let start = params.range_in.trim();
    let end = params.range_out.trim();
    if !start.is_empty() && !end.is_empty() {
        return vec![line("playrange set", &[("in", start), ("out", end)])];
    }
    let clip = params.clip_id.trim();
    if clip.is_empty() {
        return Vec::new();
    }
    let count = if params.range_count.trim().is_empty() {
        "1"
    } else {
        params.range_count.trim()
    };
    vec![line(
        "playrange set",
        &[("clip id", clip), ("count", count)],
    )]
}

fn goto_command(params: &ActionParams) -> Vec<String> {
    match params.goto_mode.as_str() {
        "clip-start" => vec![line("goto", &[("clip", "start")])],
        "clip-end" => vec![line("goto", &[("clip", "end")])],
        "timeline-end" => vec![line("goto", &[("timeline", "end")])],
        "timecode" => {
            let tc = params.timecode.trim();
            if tc.is_empty() {
                Vec::new()
            } else {
                vec![line("goto", &[("timecode", tc)])]
            }
        }
        _ => vec![line("goto", &[("timeline", "start")])],
    }
}

fn nas_commands(params: &ActionParams) -> Vec<String> {
    let url = params.nas_url.trim();
    match params.command_mode.as_str() {
        "remove" => {
            if url.is_empty() {
                Vec::new()
            } else {
                vec![line("nas remove", &[("url", url)])]
            }
        }
        "deselect" => vec![line("nas deselect", &[])],
        "add" => block(
            "nas add",
            &[
                ("url", url),
                ("username", params.nas_user.trim()),
                ("password", params.nas_password.trim()),
            ],
        )
        .into_iter()
        .collect(),
        _ => block("nas select", &[("url", url)]).into_iter().collect(),
    }
}

fn slate_block(params: &ActionParams) -> Vec<String> {
    block(
        "slate",
        &[
            ("reel", params.slate_reel.trim()),
            ("scene", params.slate_scene.trim()),
            ("take", params.slate_take.trim()),
            ("project", params.slate_project.trim()),
            ("lens", params.slate_lens.trim()),
        ],
    )
    .into_iter()
    .collect()
}

fn config_value(command: &str, key: &str, value: &str) -> Vec<String> {
    one_if_value(command, &[(key, value.trim())])
}

fn one_if_value(command: &str, params: &[(&str, &str)]) -> Vec<String> {
    if params.iter().any(|(_, value)| value.is_empty()) {
        Vec::new()
    } else {
        vec![line(command, params)]
    }
}

fn bool_config(key: &str, snap: &DeckSnapshot) -> Vec<String> {
    vec![line(
        "configuration",
        &[(key, flip(snap.config_value(key)))],
    )]
}

fn format_play_speed(percent: i32) -> String {
    let sign = if percent < 0 { "-" } else { "" };
    let abs = percent.unsigned_abs();
    format!("{sign}{}.{:02}x", abs / 100, abs % 100)
}

fn reported_audio_channels(snap: &DeckSnapshot) -> Option<i32> {
    let count = snap.config_value("audio input channels").trim();
    if count.is_empty() {
        None
    } else {
        count.parse().ok()
    }
}

fn current_clip_id(snap: &DeckSnapshot) -> &str {
    let id = snap.transport_value("clip id").trim();
    if id.is_empty() || id.eq_ignore_ascii_case("none") {
        ""
    } else {
        id
    }
}

fn clip_title(snap: &DeckSnapshot) -> String {
    let name = snap.clip_name();
    if name.is_empty() {
        current_clip_id(snap).to_string()
    } else {
        name
    }
}

/// Dial caption: timeline number, then the clip name, split by a record separator.
fn clip_list_caption(snap: &DeckSnapshot) -> String {
    format!("{}\u{1e}{}", current_clip_id(snap), snap.clip_name())
}

fn clip_indicator(snap: &DeckSnapshot) -> u8 {
    let len = u32::try_from(snap.clips.len()).unwrap_or(0);
    if len == 0 {
        return 0;
    }
    let id = snap.transport_value("clip id");
    let Some(index) = snap.clips.iter().position(|clip| clip.id == id) else {
        return 0;
    };
    percent_of(u32::try_from(index).unwrap_or(0).saturating_add(1), len)
}

/// Integer frame count from a transport or slot `video format` token.
///
/// Fractional rates use the timecode frame count (`29.97` is 30). `None` when
/// the deck has not reported a format this plugin can read.
fn reported_fps(snap: &DeckSnapshot) -> Option<u32> {
    let transport = snap.transport_value("video format").trim();
    let token = if transport.is_empty() {
        snap.slot_value("video format").trim()
    } else {
        transport
    };
    if token.is_empty() {
        None
    } else {
        integer_fps(token)
    }
}

fn integer_fps(token: &str) -> Option<u32> {
    match token {
        "NTSC" => Some(30),
        "PAL" => Some(25),
        "NTSCp" => Some(60),
        "PALp" => Some(50),
        _ => match VideoFormat::parse_ethernet(token).ok()? {
            VideoFormat::Raster { rate, .. } => Some(rate.integer_fps()),
            VideoFormat::Sd(_) => None,
        },
    }
}

/// `HH:MM:SS:FF` for a jog offset.
///
/// One frame is `:01` in every format. A larger step needs the deck's frame
/// rate, because the frame field rolls into seconds at that rate.
fn jog_timecode(frames: u32, fps: Option<u32>) -> Option<String> {
    match fps {
        Some(fps) if fps > 0 => Some(Timecode::from_frames(frames, fps).to_string()),
        _ if (1..24).contains(&frames) => Some(format!("00:00:00:{frames:02}")),
        _ => None,
    }
}

fn step_of(kind: ActionKind, params: &ActionParams) -> i32 {
    if params.step != 0 {
        return params.step;
    }
    match kind {
        ActionKind::Shuttle => 100,
        ActionKind::AudioChannels => 2,
        _ => 1,
    }
}

fn line(name: &str, params: &[(&str, &str)]) -> String {
    let mut out = String::from(name);
    if !params.is_empty() {
        out.push(':');
        for (key, value) in params {
            out.push(' ');
            out.push_str(key);
            out.push_str(": ");
            out.push_str(value);
        }
    }
    out.push_str("\r\n");
    out
}

fn block(name: &str, rows: &[(&str, &str)]) -> Option<String> {
    if rows.iter().all(|(_, value)| value.is_empty()) {
        return None;
    }
    let mut out = format!("{name}:\r\n");
    for (key, value) in rows {
        if value.is_empty() {
            continue;
        }
        out.push_str(key);
        out.push_str(": ");
        out.push_str(value);
        out.push_str("\r\n");
    }
    out.push_str("\r\n");
    Some(out)
}

fn raw_line(text: &str) -> Option<String> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return None;
    }
    let name = trimmed
        .split(':')
        .next()
        .unwrap_or(trimmed)
        .trim()
        .to_ascii_lowercase();
    if name == "format" || name == "reboot" {
        return None;
    }
    let normalized = trimmed.replace("\r\n", "\n").replace('\n', "\r\n");
    if normalized.ends_with("\r\n") {
        Some(normalized)
    } else {
        Some(format!("{normalized}\r\n"))
    }
}

fn playing(status: &str) -> bool {
    matches!(status, "play" | "forward" | "rewind" | "jog" | "shuttle")
}

fn flag(value: &str) -> bool {
    matches!(
        value.trim().to_ascii_lowercase().as_str(),
        "true" | "yes" | "1" | "on"
    )
}

fn flip(value: &str) -> &'static str {
    if flag(value) { "false" } else { "true" }
}

fn bool_text(value: bool) -> &'static str {
    if value { "true" } else { "false" }
}

fn flag_face(value: &str, title: &str) -> Face {
    Face {
        active: flag(value),
        title: if value.is_empty() {
            title.to_string()
        } else {
            value.to_string()
        },
        indicator: 0,
    }
}

fn value_face(current: &str, wanted: &str) -> Face {
    let wanted = wanted.trim();
    Face {
        active: !wanted.is_empty() && current.eq_ignore_ascii_case(wanted),
        title: current.to_string(),
        indicator: 0,
    }
}

fn display_timecode(snap: &DeckSnapshot) -> String {
    let display = snap.transport_value("display timecode");
    let raw = if display.is_empty() {
        snap.transport_value("timecode")
    } else {
        display
    };
    short_timecode(raw)
}

/// `HH:MM:SS:FF` shown as `mm:ss:ff`. Hours appear only when they are greater than zero.
fn short_timecode(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    let (sign, body) = match trimmed.as_bytes().first() {
        Some(b'+' | b'-') => (&trimmed[..1], &trimmed[1..]),
        _ => ("", trimmed),
    };
    let parts: Vec<&str> = body.split([':', ';']).collect();
    if parts.len() != 3 && parts.len() != 4 {
        return trimmed.to_string();
    }
    let hours_ok = !parts[0].is_empty() && parts[0].bytes().all(|byte| byte.is_ascii_digit());
    let rest_ok = parts[1..3]
        .iter()
        .all(|part| part.len() == 2 && part.bytes().all(|byte| byte.is_ascii_digit()));
    let frames_ok = parts.len() == 3
        || (parts[3].len() == 2 && parts[3].bytes().all(|byte| byte.is_ascii_digit()));
    if !hours_ok || !rest_ok || !frames_ok {
        return trimmed.to_string();
    }
    let frames = if parts.len() == 4 {
        let separator = if body.contains(';') { ';' } else { ':' };
        format!("{separator}{}", parts[3])
    } else {
        String::new()
    };
    let hours: u32 = parts[0].parse().unwrap_or(0);
    let shown = if hours > 0 {
        format!("{hours:02}:{}:{}{frames}", parts[1], parts[2])
    } else {
        format!("{}:{}{frames}", parts[1], parts[2])
    };
    format!("{sign}{shown}")
}

fn recording_time(snap: &DeckSnapshot) -> String {
    let time = snap.slot_value("recording time");
    if time.is_empty() {
        snap.status().to_string()
    } else {
        short_timecode(time)
    }
}

fn recording_seconds(snap: &DeckSnapshot) -> u64 {
    snap.slot_value("recording time")
        .split_whitespace()
        .next()
        .unwrap_or("")
        .parse()
        .unwrap_or(0)
}

fn playrange_title(snap: &DeckSnapshot) -> String {
    let start = snap
        .playrange
        .get("timeline in")
        .or_else(|| snap.playrange.get("in"))
        .cloned()
        .unwrap_or_default();
    let end = snap
        .playrange
        .get("timeline out")
        .or_else(|| snap.playrange.get("out"))
        .cloned()
        .unwrap_or_default();
    if start.is_empty() && end.is_empty() {
        if snap.playrange_active() {
            "range".into()
        } else {
            String::new()
        }
    } else {
        format!("{}–{}", short_timecode(&start), short_timecode(&end))
    }
}

fn status_title(params: &ActionParams, snap: &DeckSnapshot) -> String {
    match params.status_field.as_str() {
        "speed" => snap.speed().to_string(),
        "timecode" | "timeline" => short_timecode(snap.transport_value("timecode")),
        "display timecode" => short_timecode(snap.transport_value("display timecode")),
        "clip" => {
            let name = snap.clip_name();
            if name.is_empty() {
                snap.transport_value("clip id").to_string()
            } else {
                format!("{} {name}", snap.transport_value("clip id"))
            }
        }
        "slot" => snap.slot_value("slot name").to_string(),
        "video format" => snap.transport_value("video format").to_string(),
        "model" => snap.model.clone(),
        "protocol" => snap.protocol_version.clone(),
        "uptime" => snap.uptime_seconds.to_string(),
        "cache" => snap.config_value("cache").to_string(),
        _ => snap.status().to_string(),
    }
}

fn position_indicator(snap: &DeckSnapshot) -> u8 {
    let frame = snap.timeline();
    let end = snap
        .playrange
        .get("timeline out")
        .and_then(|value| value.parse::<u32>().ok())
        .unwrap_or(0);
    if end == 0 { 0 } else { percent_of(frame, end) }
}

fn percent_of(value: u32, span: u32) -> u8 {
    if span == 0 {
        0
    } else {
        ((u64::from(value) * 100) / u64::from(span)).min(100) as u8
    }
}

fn first_nonempty<'a>(values: &[&'a str]) -> &'a str {
    values
        .iter()
        .copied()
        .find(|value| !value.is_empty())
        .unwrap_or("")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snap_status(status: &str, speed: &str) -> DeckSnapshot {
        let mut snap = DeckSnapshot::default();
        snap.transport.insert("status".into(), status.into());
        snap.transport.insert("speed".into(), speed.into());
        snap.transport
            .insert("display timecode".into(), "01:00:00:00".into());
        snap
    }

    #[test]
    fn play_toggles_and_record_hold_spills() {
        let stopped = snap_status("stopped", "0");
        let play = commands(
            ActionKind::Play,
            &ActionParams::default(),
            &stopped,
            Gesture::Press(PressKind::Single),
        );
        assert_eq!(
            play,
            vec!["play: speed: 100 loop: false single clip: false\r\n".to_string()]
        );
        let playing = snap_status("play", "100");
        let stop = commands(
            ActionKind::Play,
            &ActionParams::default(),
            &playing,
            Gesture::Press(PressKind::Single),
        );
        assert_eq!(stop, vec!["stop\r\n".to_string()]);
        let spill = commands(
            ActionKind::Record,
            &ActionParams::default(),
            &stopped,
            Gesture::Press(PressKind::Long),
        );
        assert_eq!(spill, vec!["record spill\r\n".to_string()]);
    }

    #[test]
    fn shuttle_rotation_clamps_at_the_protocol_limit() {
        let mut snap = snap_status("shuttle", "4900");
        let params = ActionParams {
            step: 200,
            ..ActionParams::default()
        };
        let sent = commands(ActionKind::Shuttle, &params, &snap, Gesture::Rotate(2));
        assert_eq!(sent, vec!["shuttle: speed: 5000\r\n".to_string()]);
        snap.transport.insert("speed".into(), "100".into());
        let face = face(ActionKind::Shuttle, &params, &snap);
        assert_eq!(face.title, "100%");
        assert_eq!(face.indicator, 2);
    }

    #[test]
    fn displayed_timecode_keeps_frames_and_hides_zero_hours() {
        assert_eq!(short_timecode("00:01:23:04"), "01:23:04");
        assert_eq!(short_timecode("00:00:05;12"), "00:05;12");
        assert_eq!(short_timecode("01:02:03:04"), "01:02:03:04");
        assert_eq!(short_timecode("10:00:00:00"), "10:00:00:00");
        assert_eq!(short_timecode(""), "");
        assert_eq!(short_timecode("stopped"), "stopped");
        let mut snap = snap_status("play", "100");
        snap.transport
            .insert("display timecode".into(), "00:04:05:06".into());
        let shown = face(ActionKind::Jog, &ActionParams::default(), &snap);
        assert_eq!(shown.title, "04:05:06");
    }

    #[test]
    fn play_speed_follows_the_reported_speed() {
        let snap = snap_status("stopped", "0");
        let params = ActionParams::default();
        let shown = face(ActionKind::PlaySpeed, &params, &snap);
        assert_eq!(shown.title, "0.00x");
        assert_eq!(shown.indicator, 0);
        let sent = commands(ActionKind::PlaySpeed, &params, &snap, Gesture::Rotate(1));
        assert_eq!(sent, vec!["play: speed: 1\r\n".to_string()]);
        let playing = snap_status("play", "200");
        let shown = face(ActionKind::PlaySpeed, &params, &playing);
        assert_eq!(shown.title, "2.00x");
        let sent = commands(ActionKind::PlaySpeed, &params, &playing, Gesture::Rotate(1));
        assert_eq!(sent, vec!["play: speed: 201\r\n".to_string()]);
    }

    #[test]
    fn jog_uses_the_reported_frame_rate() {
        let params = ActionParams {
            step: 30,
            ..ActionParams::default()
        };
        let mut snap = snap_status("stopped", "0");
        snap.transport
            .insert("video format".into(), "1080p2997".into());
        let sent = commands(ActionKind::Jog, &params, &snap, Gesture::Rotate(1));
        assert_eq!(sent, vec!["jog: timecode: +00:00:01:00\r\n".to_string()]);

        snap.transport
            .insert("video format".into(), "1080p24".into());
        let sent = commands(ActionKind::Jog, &params, &snap, Gesture::Rotate(-1));
        assert_eq!(sent, vec!["jog: timecode: -00:00:01:06\r\n".to_string()]);

        snap.transport
            .insert("video format".into(), "1080i5994".into());
        let minute = ActionParams {
            step: 60,
            ..ActionParams::default()
        };
        let sent = commands(ActionKind::Jog, &minute, &snap, Gesture::Rotate(60));
        assert_eq!(sent, vec!["jog: timecode: +00:01:00:00\r\n".to_string()]);

        let unknown = snap_status("stopped", "0");
        let one = commands(
            ActionKind::Jog,
            &ActionParams::default(),
            &unknown,
            Gesture::Rotate(1),
        );
        assert_eq!(one, vec!["jog: timecode: +00:00:00:01\r\n".to_string()]);
        let guessed = commands(ActionKind::Jog, &params, &unknown, Gesture::Rotate(1));
        assert!(guessed.is_empty());
    }

    #[test]
    fn position_bar_uses_a_numeric_duration_only() {
        let mut snap = snap_status("play", "100");
        snap.transport.insert("timeline".into(), "50".into());
        snap.playrange
            .insert("timeline out".into(), "00:10:00:00".into());
        let params = ActionParams::default();
        let shown = face(ActionKind::Position, &params, &snap);
        assert_eq!(shown.indicator, 0);
        snap.playrange.insert("timeline out".into(), "200".into());
        let shown = face(ActionKind::Position, &params, &snap);
        assert_eq!(shown.indicator, 25);
    }

    #[test]
    fn clip_list_steps_and_plays_the_current_clip() {
        let mut snap = DeckSnapshot::default();
        snap.transport.insert("clip id".into(), "3".into());
        snap.clips = vec![
            hyperdeck_pool::ClipRow {
                id: "1".into(),
                name: "slate".into(),
            },
            hyperdeck_pool::ClipRow {
                id: "3".into(),
                name: "interview".into(),
            },
        ];
        let params = ActionParams::default();
        let sent = commands(ActionKind::ClipList, &params, &snap, Gesture::Rotate(2));
        assert_eq!(sent, vec!["goto: clip id: +2\r\n".to_string()]);
        let back = commands(ActionKind::ClipList, &params, &snap, Gesture::Rotate(-1));
        assert_eq!(back, vec!["goto: clip id: -1\r\n".to_string()]);
        let play = commands(
            ActionKind::ClipList,
            &params,
            &snap,
            Gesture::Press(PressKind::Single),
        );
        assert_eq!(play, vec!["play: clip id: 3\r\n".to_string()]);
        let stop = commands(ActionKind::ClipList, &params, &snap, Gesture::Touch);
        assert_eq!(stop, vec!["stop\r\n".to_string()]);
        let shown = face(ActionKind::ClipList, &params, &snap);
        assert_eq!(shown.title, "3\u{1e}interview");
        assert_eq!(shown.indicator, 100);
    }

    #[test]
    fn audio_channels_follow_the_reported_count() {
        let params = ActionParams::default();
        let snap = DeckSnapshot::default();
        let shown = face(ActionKind::AudioChannels, &params, &snap);
        assert_eq!(shown.title, "");
        assert_eq!(shown.indicator, 0);
        let sent = commands(
            ActionKind::AudioChannels,
            &params,
            &snap,
            Gesture::Rotate(1),
        );
        assert!(sent.is_empty());

        let mut snap = DeckSnapshot::default();
        snap.config
            .insert("audio input channels".into(), "2".into());
        let shown = face(ActionKind::AudioChannels, &params, &snap);
        assert_eq!(shown.title, "2");
        let sent = commands(
            ActionKind::AudioChannels,
            &params,
            &snap,
            Gesture::Rotate(1),
        );
        assert_eq!(
            sent,
            vec!["configuration: audio input channels: 4\r\n".to_string()]
        );
    }

    #[test]
    fn raw_refuses_format_and_reboot() {
        assert!(raw_line("format: prepare: slot id: 1").is_none());
        assert!(raw_line("reboot").is_none());
        assert_eq!(raw_line("ping").as_deref(), Some("ping\r\n"));
    }
}
