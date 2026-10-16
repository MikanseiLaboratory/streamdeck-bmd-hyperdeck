use hyperdeck_pool::DeckSnapshot;

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
            let speed = play_speed_percent(snap);
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
            let count = snap.config_value("audio input channels");
            let parsed = audio_channel_base(snap);
            Face {
                active: false,
                title: if count.is_empty() {
                    "2".into()
                } else {
                    count.to_string()
                },
                indicator: percent_of(parsed.unsigned_abs().min(64), 64),
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
            let tc = frames_to_timecode(frames);
            let signed = if delta < 0 {
                format!("-{tc}")
            } else {
                format!("+{tc}")
            };
            vec![line("jog", &[("timecode", &signed)])]
        }
        ActionKind::PlaySpeed => {
            let speed = (i64::from(play_speed_percent(snap)) + i64::from(delta)).clamp(-5000, 5000);
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
            let current = audio_channel_base(snap);
            let next = (current + delta).clamp(0, 64);
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

/// Stopped decks report speed 0. The play-speed dial treats that as 1.00x.
fn play_speed_percent(snap: &DeckSnapshot) -> i32 {
    let speed = snap.speed();
    if speed == 0 { 100 } else { speed }
}

fn format_play_speed(percent: i32) -> String {
    let sign = if percent < 0 { "-" } else { "" };
    let abs = percent.unsigned_abs();
    format!("{sign}{}.{:02}x", abs / 100, abs % 100)
}

fn audio_channel_base(snap: &DeckSnapshot) -> i32 {
    let count = snap.config_value("audio input channels").trim();
    if count.is_empty() {
        2
    } else {
        count.parse().unwrap_or(0)
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

fn frames_to_timecode(frames: u32) -> String {
    let fps = 30u32;
    let seconds = frames / fps;
    let ff = frames % fps;
    let ss = seconds % 60;
    let mm = (seconds / 60) % 60;
    let hh = seconds / 3600;
    format!("{hh:02}:{mm:02}:{ss:02}:{ff:02}")
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
    if display.is_empty() {
        snap.transport_value("timecode").to_string()
    } else {
        display.to_string()
    }
}

fn recording_time(snap: &DeckSnapshot) -> String {
    let time = snap.slot_value("recording time");
    if time.is_empty() {
        snap.status().to_string()
    } else {
        time.to_string()
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
        format!("{start}–{end}")
    }
}

fn status_title(params: &ActionParams, snap: &DeckSnapshot) -> String {
    match params.status_field.as_str() {
        "speed" => snap.speed().to_string(),
        "timecode" | "timeline" => snap.transport_value("timecode").to_string(),
        "display timecode" => snap.transport_value("display timecode").to_string(),
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
    fn stopped_play_speed_starts_at_one_x() {
        let snap = snap_status("stopped", "0");
        let params = ActionParams::default();
        let shown = face(ActionKind::PlaySpeed, &params, &snap);
        assert_eq!(shown.title, "1.00x");
        let sent = commands(ActionKind::PlaySpeed, &params, &snap, Gesture::Rotate(1));
        assert_eq!(sent, vec!["play: speed: 101\r\n".to_string()]);
        let playing = snap_status("play", "200");
        let shown = face(ActionKind::PlaySpeed, &params, &playing);
        assert_eq!(shown.title, "2.00x");
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
    fn empty_audio_channels_start_at_two() {
        let snap = DeckSnapshot::default();
        let params = ActionParams::default();
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
        assert_eq!(ActionParams::default().range_count, "1");
    }

    #[test]
    fn raw_refuses_format_and_reboot() {
        assert!(raw_line("format: prepare: slot id: 1").is_none());
        assert!(raw_line("reboot").is_none());
        assert_eq!(raw_line("ping").as_deref(), Some("ping\r\n"));
    }
}
