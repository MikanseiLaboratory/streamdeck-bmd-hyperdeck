/// One Stream Deck action.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActionKind {
    Play,
    Record,
    Stop,
    Preview,
    Loop,
    Slot,
    Clip,
    PlayRange,
    Remote,
    PlayOnStartup,
    StopMode,
    RecordTrigger,
    VideoInput,
    AudioInput,
    FileFormat,
    AudioCodec,
    TimecodeInput,
    TimecodeOutput,
    DynamicRange,
    RecordCache,
    AppendTimestamp,
    UsbSpill,
    ReferenceSource,
    Genlock,
    ExternalDrive,
    Nas,
    DroppedFrames,
    Remaining,
    ReferenceLock,
    ClipStep,
    GoTo,
    Identify,
    RecordNamed,
    ClipsAdd,
    ClipsRemove,
    ClipsRebuild,
    ClipsClear,
    SlotUnblock,
    TimecodePreset,
    RecordPrefix,
    Slate,
    Raw,
    Status,
    Shuttle,
    Jog,
    ClipList,
    PlaySpeed,
    Position,
    AudioChannels,
    RawDial,
}

impl ActionKind {
    /// Manifest UUID.
    pub fn uuid(self) -> &'static str {
        match self {
            Self::Play => "dev.mikanseilaboratory.bmd.hyperdeck.play",
            Self::Record => "dev.mikanseilaboratory.bmd.hyperdeck.record",
            Self::Stop => "dev.mikanseilaboratory.bmd.hyperdeck.stop",
            Self::Preview => "dev.mikanseilaboratory.bmd.hyperdeck.preview",
            Self::Loop => "dev.mikanseilaboratory.bmd.hyperdeck.loop",
            Self::Slot => "dev.mikanseilaboratory.bmd.hyperdeck.slot",
            Self::Clip => "dev.mikanseilaboratory.bmd.hyperdeck.clip",
            Self::PlayRange => "dev.mikanseilaboratory.bmd.hyperdeck.playrange",
            Self::Remote => "dev.mikanseilaboratory.bmd.hyperdeck.remote",
            Self::PlayOnStartup => "dev.mikanseilaboratory.bmd.hyperdeck.playonstartup",
            Self::StopMode => "dev.mikanseilaboratory.bmd.hyperdeck.stopmode",
            Self::RecordTrigger => "dev.mikanseilaboratory.bmd.hyperdeck.recordtrigger",
            Self::VideoInput => "dev.mikanseilaboratory.bmd.hyperdeck.videoinput",
            Self::AudioInput => "dev.mikanseilaboratory.bmd.hyperdeck.audioinput",
            Self::FileFormat => "dev.mikanseilaboratory.bmd.hyperdeck.fileformat",
            Self::AudioCodec => "dev.mikanseilaboratory.bmd.hyperdeck.audiocodec",
            Self::TimecodeInput => "dev.mikanseilaboratory.bmd.hyperdeck.timecodeinput",
            Self::TimecodeOutput => "dev.mikanseilaboratory.bmd.hyperdeck.timecodeoutput",
            Self::DynamicRange => "dev.mikanseilaboratory.bmd.hyperdeck.dynamicrange",
            Self::RecordCache => "dev.mikanseilaboratory.bmd.hyperdeck.recordcache",
            Self::AppendTimestamp => "dev.mikanseilaboratory.bmd.hyperdeck.appendtimestamp",
            Self::UsbSpill => "dev.mikanseilaboratory.bmd.hyperdeck.usbspill",
            Self::ReferenceSource => "dev.mikanseilaboratory.bmd.hyperdeck.referencesource",
            Self::Genlock => "dev.mikanseilaboratory.bmd.hyperdeck.genlock",
            Self::ExternalDrive => "dev.mikanseilaboratory.bmd.hyperdeck.externaldrive",
            Self::Nas => "dev.mikanseilaboratory.bmd.hyperdeck.nas",
            Self::DroppedFrames => "dev.mikanseilaboratory.bmd.hyperdeck.droppedframes",
            Self::Remaining => "dev.mikanseilaboratory.bmd.hyperdeck.remaining",
            Self::ReferenceLock => "dev.mikanseilaboratory.bmd.hyperdeck.referencelock",
            Self::ClipStep => "dev.mikanseilaboratory.bmd.hyperdeck.clipstep",
            Self::GoTo => "dev.mikanseilaboratory.bmd.hyperdeck.goto",
            Self::Identify => "dev.mikanseilaboratory.bmd.hyperdeck.identify",
            Self::RecordNamed => "dev.mikanseilaboratory.bmd.hyperdeck.recordnamed",
            Self::ClipsAdd => "dev.mikanseilaboratory.bmd.hyperdeck.clipsadd",
            Self::ClipsRemove => "dev.mikanseilaboratory.bmd.hyperdeck.clipsremove",
            Self::ClipsRebuild => "dev.mikanseilaboratory.bmd.hyperdeck.clipsrebuild",
            Self::ClipsClear => "dev.mikanseilaboratory.bmd.hyperdeck.clipsclear",
            Self::SlotUnblock => "dev.mikanseilaboratory.bmd.hyperdeck.slotunblock",
            Self::TimecodePreset => "dev.mikanseilaboratory.bmd.hyperdeck.timecodepreset",
            Self::RecordPrefix => "dev.mikanseilaboratory.bmd.hyperdeck.recordprefix",
            Self::Slate => "dev.mikanseilaboratory.bmd.hyperdeck.slate",
            Self::Raw => "dev.mikanseilaboratory.bmd.hyperdeck.raw",
            Self::Status => "dev.mikanseilaboratory.bmd.hyperdeck.status",
            Self::Shuttle => "dev.mikanseilaboratory.bmd.hyperdeck.shuttle",
            Self::Jog => "dev.mikanseilaboratory.bmd.hyperdeck.jog",
            Self::ClipList => "dev.mikanseilaboratory.bmd.hyperdeck.cliplist",
            Self::PlaySpeed => "dev.mikanseilaboratory.bmd.hyperdeck.playspeed",
            Self::Position => "dev.mikanseilaboratory.bmd.hyperdeck.position",
            Self::AudioChannels => "dev.mikanseilaboratory.bmd.hyperdeck.audiochannels",
            Self::RawDial => "dev.mikanseilaboratory.bmd.hyperdeck.rawdial",
        }
    }

    /// Two hardware states, used when the key lights up.
    pub fn has_toggle_state(self) -> bool {
        !matches!(
            self,
            Self::ClipStep
                | Self::GoTo
                | Self::Identify
                | Self::RecordNamed
                | Self::ClipsAdd
                | Self::ClipsRemove
                | Self::ClipsRebuild
                | Self::ClipsClear
                | Self::SlotUnblock
                | Self::TimecodePreset
                | Self::RecordPrefix
                | Self::Slate
                | Self::Raw
                | Self::Status
                | Self::Shuttle
                | Self::Jog
                | Self::ClipList
                | Self::PlaySpeed
                | Self::Position
                | Self::AudioChannels
                | Self::RawDial
        )
    }

    /// Show the OK check after a successful momentary press.
    pub fn confirms_press(self) -> bool {
        matches!(
            self,
            Self::ClipStep
                | Self::GoTo
                | Self::Identify
                | Self::RecordNamed
                | Self::ClipsAdd
                | Self::ClipsRemove
                | Self::ClipsRebuild
                | Self::ClipsClear
                | Self::SlotUnblock
                | Self::TimecodePreset
                | Self::RecordPrefix
                | Self::Slate
                | Self::Raw
        )
    }

    /// Encoder action.
    pub fn is_dial(self) -> bool {
        matches!(
            self,
            Self::Shuttle
                | Self::Jog
                | Self::ClipList
                | Self::PlaySpeed
                | Self::Position
                | Self::AudioChannels
                | Self::RawDial
        )
    }

    /// Heading drawn on a dial.
    pub fn dial_heading(self) -> &'static str {
        match self {
            Self::Shuttle => "Shuttle",
            Self::Jog => "Jog",
            Self::ClipList => "Clip",
            Self::PlaySpeed => "Speed",
            Self::Position => "Position",
            Self::AudioChannels => "Channels",
            Self::RawDial => "Raw",
            _ => "",
        }
    }
}

/// Short press or a press held past the long-press threshold.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PressKind {
    Single,
    Long,
}
