use serde_json::Value;
use streamdeck_plugin::{
    ActionContext, ActionPayload, DialRotatePayload, EncoderAction, KeypadAction, Result,
    streamdeck_action,
};

use crate::contracts::ActionSettings;
use crate::kind::ActionKind;
use crate::state::AppState;

macro_rules! hyperdeck_key {
    ($name:ident, $uuid:literal, $kind:expr) => {
        #[derive(Default)]
        pub struct $name;

        #[streamdeck_action(uuid = $uuid, settings = ActionSettings, state = AppState)]
        impl KeypadAction for $name {
            async fn on_will_appear(
                &mut self,
                payload: &ActionPayload,
                ctx: &ActionContext<'_, Self::Settings, Self::State>,
            ) -> Result<()> {
                register(ctx, $kind, payload.is_in_multi_action).await;
                Ok(())
            }

            async fn on_will_disappear(
                &mut self,
                _payload: &ActionPayload,
                ctx: &ActionContext<'_, Self::Settings, Self::State>,
            ) -> Result<()> {
                ctx.state().remove_key(&ctx.identity().context).await;
                Ok(())
            }

            async fn on_did_receive_settings(
                &mut self,
                payload: &ActionPayload,
                ctx: &ActionContext<'_, Self::Settings, Self::State>,
            ) -> Result<()> {
                register(ctx, $kind, payload.is_in_multi_action).await;
                Ok(())
            }

            async fn on_key_down(
                &mut self,
                _payload: &ActionPayload,
                ctx: &ActionContext<'_, Self::Settings, Self::State>,
            ) -> Result<()> {
                ctx.state().begin_press(&ctx.identity().context);
                Ok(())
            }

            async fn on_key_up(
                &mut self,
                _payload: &ActionPayload,
                ctx: &ActionContext<'_, Self::Settings, Self::State>,
            ) -> Result<()> {
                ctx.state().end_press(&ctx.identity().context);
                Ok(())
            }

            async fn on_property_inspector_did_appear(
                &mut self,
                ctx: &ActionContext<'_, Self::Settings, Self::State>,
            ) -> Result<()> {
                ctx.state()
                    .inspector_appeared(&ctx.identity().context)
                    .await;
                Ok(())
            }

            async fn on_property_inspector_did_disappear(
                &mut self,
                ctx: &ActionContext<'_, Self::Settings, Self::State>,
            ) -> Result<()> {
                ctx.state()
                    .inspector_disappeared(&ctx.identity().context)
                    .await;
                Ok(())
            }

            async fn on_property_inspector_message(
                &mut self,
                payload: &Value,
                ctx: &ActionContext<'_, Self::Settings, Self::State>,
            ) -> Result<()> {
                ctx.state()
                    .handle_inspector(&ctx.identity().context, payload.clone());
                Ok(())
            }
        }
    };
}

macro_rules! hyperdeck_dial {
    ($name:ident, $uuid:literal, $kind:expr) => {
        #[derive(Default)]
        pub struct $name;

        #[streamdeck_action(uuid = $uuid, settings = ActionSettings, state = AppState)]
        impl EncoderAction for $name {
            async fn on_will_appear(
                &mut self,
                payload: &ActionPayload,
                ctx: &ActionContext<'_, Self::Settings, Self::State>,
            ) -> Result<()> {
                register(ctx, $kind, payload.is_in_multi_action).await;
                Ok(())
            }

            async fn on_will_disappear(
                &mut self,
                _payload: &ActionPayload,
                ctx: &ActionContext<'_, Self::Settings, Self::State>,
            ) -> Result<()> {
                ctx.state().remove_key(&ctx.identity().context).await;
                Ok(())
            }

            async fn on_did_receive_settings(
                &mut self,
                payload: &ActionPayload,
                ctx: &ActionContext<'_, Self::Settings, Self::State>,
            ) -> Result<()> {
                register(ctx, $kind, payload.is_in_multi_action).await;
                Ok(())
            }

            async fn on_dial_rotate(
                &mut self,
                payload: &DialRotatePayload,
                ctx: &ActionContext<'_, Self::Settings, Self::State>,
            ) -> Result<()> {
                ctx.state().rotate(&ctx.identity().context, payload.ticks);
                Ok(())
            }

            async fn on_dial_down(
                &mut self,
                _payload: &ActionPayload,
                ctx: &ActionContext<'_, Self::Settings, Self::State>,
            ) -> Result<()> {
                ctx.state().dial_down(&ctx.identity().context);
                Ok(())
            }

            async fn on_property_inspector_did_appear(
                &mut self,
                ctx: &ActionContext<'_, Self::Settings, Self::State>,
            ) -> Result<()> {
                ctx.state()
                    .inspector_appeared(&ctx.identity().context)
                    .await;
                Ok(())
            }

            async fn on_property_inspector_did_disappear(
                &mut self,
                ctx: &ActionContext<'_, Self::Settings, Self::State>,
            ) -> Result<()> {
                ctx.state()
                    .inspector_disappeared(&ctx.identity().context)
                    .await;
                Ok(())
            }

            async fn on_property_inspector_message(
                &mut self,
                payload: &Value,
                ctx: &ActionContext<'_, Self::Settings, Self::State>,
            ) -> Result<()> {
                ctx.state()
                    .handle_inspector(&ctx.identity().context, payload.clone());
                Ok(())
            }
        }
    };
}

async fn register(
    ctx: &ActionContext<'_, ActionSettings, AppState>,
    kind: ActionKind,
    multi: bool,
) {
    ctx.state().note_sender(ctx.sender().clone()).await;
    ctx.state()
        .upsert_key(&ctx.identity().context, kind, ctx.settings(), multi)
        .await;
}

hyperdeck_key!(
    PlayAction,
    "dev.mikanseilaboratory.bmd.hyperdeck.play",
    ActionKind::Play
);
hyperdeck_key!(
    RecordAction,
    "dev.mikanseilaboratory.bmd.hyperdeck.record",
    ActionKind::Record
);
hyperdeck_key!(
    StopAction,
    "dev.mikanseilaboratory.bmd.hyperdeck.stop",
    ActionKind::Stop
);
hyperdeck_key!(
    PreviewAction,
    "dev.mikanseilaboratory.bmd.hyperdeck.preview",
    ActionKind::Preview
);
hyperdeck_key!(
    SlotAction,
    "dev.mikanseilaboratory.bmd.hyperdeck.slot",
    ActionKind::Slot
);
hyperdeck_key!(
    ClipAction,
    "dev.mikanseilaboratory.bmd.hyperdeck.clip",
    ActionKind::Clip
);
hyperdeck_key!(
    PlayRangeAction,
    "dev.mikanseilaboratory.bmd.hyperdeck.playrange",
    ActionKind::PlayRange
);
hyperdeck_key!(
    RemoteAction,
    "dev.mikanseilaboratory.bmd.hyperdeck.remote",
    ActionKind::Remote
);
hyperdeck_key!(
    PlayOnStartupAction,
    "dev.mikanseilaboratory.bmd.hyperdeck.playonstartup",
    ActionKind::PlayOnStartup
);
hyperdeck_key!(
    StopModeAction,
    "dev.mikanseilaboratory.bmd.hyperdeck.stopmode",
    ActionKind::StopMode
);
hyperdeck_key!(
    RecordTriggerAction,
    "dev.mikanseilaboratory.bmd.hyperdeck.recordtrigger",
    ActionKind::RecordTrigger
);
hyperdeck_key!(
    VideoInputAction,
    "dev.mikanseilaboratory.bmd.hyperdeck.videoinput",
    ActionKind::VideoInput
);
hyperdeck_key!(
    AudioInputAction,
    "dev.mikanseilaboratory.bmd.hyperdeck.audioinput",
    ActionKind::AudioInput
);
hyperdeck_key!(
    FileFormatAction,
    "dev.mikanseilaboratory.bmd.hyperdeck.fileformat",
    ActionKind::FileFormat
);
hyperdeck_key!(
    AudioCodecAction,
    "dev.mikanseilaboratory.bmd.hyperdeck.audiocodec",
    ActionKind::AudioCodec
);
hyperdeck_key!(
    TimecodeInputAction,
    "dev.mikanseilaboratory.bmd.hyperdeck.timecodeinput",
    ActionKind::TimecodeInput
);
hyperdeck_key!(
    TimecodeOutputAction,
    "dev.mikanseilaboratory.bmd.hyperdeck.timecodeoutput",
    ActionKind::TimecodeOutput
);
hyperdeck_key!(
    DynamicRangeAction,
    "dev.mikanseilaboratory.bmd.hyperdeck.dynamicrange",
    ActionKind::DynamicRange
);
hyperdeck_key!(
    RecordCacheAction,
    "dev.mikanseilaboratory.bmd.hyperdeck.recordcache",
    ActionKind::RecordCache
);
hyperdeck_key!(
    AppendTimestampAction,
    "dev.mikanseilaboratory.bmd.hyperdeck.appendtimestamp",
    ActionKind::AppendTimestamp
);
hyperdeck_key!(
    UsbSpillAction,
    "dev.mikanseilaboratory.bmd.hyperdeck.usbspill",
    ActionKind::UsbSpill
);
hyperdeck_key!(
    ReferenceSourceAction,
    "dev.mikanseilaboratory.bmd.hyperdeck.referencesource",
    ActionKind::ReferenceSource
);
hyperdeck_key!(
    GenlockAction,
    "dev.mikanseilaboratory.bmd.hyperdeck.genlock",
    ActionKind::Genlock
);
hyperdeck_key!(
    ExternalDriveAction,
    "dev.mikanseilaboratory.bmd.hyperdeck.externaldrive",
    ActionKind::ExternalDrive
);
hyperdeck_key!(
    NasAction,
    "dev.mikanseilaboratory.bmd.hyperdeck.nas",
    ActionKind::Nas
);
hyperdeck_key!(
    DroppedFramesAction,
    "dev.mikanseilaboratory.bmd.hyperdeck.droppedframes",
    ActionKind::DroppedFrames
);
hyperdeck_key!(
    RemainingAction,
    "dev.mikanseilaboratory.bmd.hyperdeck.remaining",
    ActionKind::Remaining
);
hyperdeck_key!(
    ReferenceLockAction,
    "dev.mikanseilaboratory.bmd.hyperdeck.referencelock",
    ActionKind::ReferenceLock
);
hyperdeck_key!(
    ClipStepAction,
    "dev.mikanseilaboratory.bmd.hyperdeck.clipstep",
    ActionKind::ClipStep
);
hyperdeck_key!(
    GoToAction,
    "dev.mikanseilaboratory.bmd.hyperdeck.goto",
    ActionKind::GoTo
);
hyperdeck_key!(
    IdentifyAction,
    "dev.mikanseilaboratory.bmd.hyperdeck.identify",
    ActionKind::Identify
);
hyperdeck_key!(
    RecordNamedAction,
    "dev.mikanseilaboratory.bmd.hyperdeck.recordnamed",
    ActionKind::RecordNamed
);
hyperdeck_key!(
    ClipsAddAction,
    "dev.mikanseilaboratory.bmd.hyperdeck.clipsadd",
    ActionKind::ClipsAdd
);
hyperdeck_key!(
    ClipsRemoveAction,
    "dev.mikanseilaboratory.bmd.hyperdeck.clipsremove",
    ActionKind::ClipsRemove
);
hyperdeck_key!(
    ClipsRebuildAction,
    "dev.mikanseilaboratory.bmd.hyperdeck.clipsrebuild",
    ActionKind::ClipsRebuild
);
hyperdeck_key!(
    ClipsClearAction,
    "dev.mikanseilaboratory.bmd.hyperdeck.clipsclear",
    ActionKind::ClipsClear
);
hyperdeck_key!(
    SlotUnblockAction,
    "dev.mikanseilaboratory.bmd.hyperdeck.slotunblock",
    ActionKind::SlotUnblock
);
hyperdeck_key!(
    TimecodePresetAction,
    "dev.mikanseilaboratory.bmd.hyperdeck.timecodepreset",
    ActionKind::TimecodePreset
);
hyperdeck_key!(
    RecordPrefixAction,
    "dev.mikanseilaboratory.bmd.hyperdeck.recordprefix",
    ActionKind::RecordPrefix
);
hyperdeck_key!(
    SlateAction,
    "dev.mikanseilaboratory.bmd.hyperdeck.slate",
    ActionKind::Slate
);
hyperdeck_key!(
    RawAction,
    "dev.mikanseilaboratory.bmd.hyperdeck.raw",
    ActionKind::Raw
);
hyperdeck_key!(
    StatusAction,
    "dev.mikanseilaboratory.bmd.hyperdeck.status",
    ActionKind::Status
);
hyperdeck_dial!(
    ShuttleDial,
    "dev.mikanseilaboratory.bmd.hyperdeck.shuttle",
    ActionKind::Shuttle
);
hyperdeck_dial!(
    JogDial,
    "dev.mikanseilaboratory.bmd.hyperdeck.jog",
    ActionKind::Jog
);
hyperdeck_dial!(
    ClipListDial,
    "dev.mikanseilaboratory.bmd.hyperdeck.cliplist",
    ActionKind::ClipList
);
hyperdeck_dial!(
    PlaySpeedDial,
    "dev.mikanseilaboratory.bmd.hyperdeck.playspeed",
    ActionKind::PlaySpeed
);
hyperdeck_dial!(
    PositionDial,
    "dev.mikanseilaboratory.bmd.hyperdeck.position",
    ActionKind::Position
);
hyperdeck_dial!(
    AudioChannelsDial,
    "dev.mikanseilaboratory.bmd.hyperdeck.audiochannels",
    ActionKind::AudioChannels
);
hyperdeck_dial!(
    RawDial,
    "dev.mikanseilaboratory.bmd.hyperdeck.rawdial",
    ActionKind::RawDial
);

/// Keep every action registration linked into the plugin binary.
pub fn force_link() {
    let _ = (
        std::any::TypeId::of::<PlayAction>(),
        std::any::TypeId::of::<RecordAction>(),
        std::any::TypeId::of::<StopAction>(),
        std::any::TypeId::of::<PreviewAction>(),
        std::any::TypeId::of::<SlotAction>(),
        std::any::TypeId::of::<ClipAction>(),
        std::any::TypeId::of::<PlayRangeAction>(),
        std::any::TypeId::of::<RemoteAction>(),
        std::any::TypeId::of::<PlayOnStartupAction>(),
        std::any::TypeId::of::<StopModeAction>(),
        std::any::TypeId::of::<RecordTriggerAction>(),
        std::any::TypeId::of::<VideoInputAction>(),
        std::any::TypeId::of::<AudioInputAction>(),
        std::any::TypeId::of::<FileFormatAction>(),
        std::any::TypeId::of::<AudioCodecAction>(),
        std::any::TypeId::of::<TimecodeInputAction>(),
        std::any::TypeId::of::<TimecodeOutputAction>(),
        std::any::TypeId::of::<DynamicRangeAction>(),
        std::any::TypeId::of::<RecordCacheAction>(),
        std::any::TypeId::of::<AppendTimestampAction>(),
        std::any::TypeId::of::<UsbSpillAction>(),
        std::any::TypeId::of::<ReferenceSourceAction>(),
        std::any::TypeId::of::<GenlockAction>(),
        std::any::TypeId::of::<ExternalDriveAction>(),
        std::any::TypeId::of::<NasAction>(),
        std::any::TypeId::of::<DroppedFramesAction>(),
        std::any::TypeId::of::<RemainingAction>(),
        std::any::TypeId::of::<ReferenceLockAction>(),
        std::any::TypeId::of::<ClipStepAction>(),
        std::any::TypeId::of::<GoToAction>(),
        std::any::TypeId::of::<IdentifyAction>(),
        std::any::TypeId::of::<RecordNamedAction>(),
        std::any::TypeId::of::<ClipsAddAction>(),
        std::any::TypeId::of::<ClipsRemoveAction>(),
        std::any::TypeId::of::<ClipsRebuildAction>(),
        std::any::TypeId::of::<ClipsClearAction>(),
        std::any::TypeId::of::<SlotUnblockAction>(),
        std::any::TypeId::of::<TimecodePresetAction>(),
        std::any::TypeId::of::<RecordPrefixAction>(),
        std::any::TypeId::of::<SlateAction>(),
        std::any::TypeId::of::<RawAction>(),
        std::any::TypeId::of::<StatusAction>(),
        std::any::TypeId::of::<ShuttleDial>(),
        std::any::TypeId::of::<JogDial>(),
        std::any::TypeId::of::<ClipListDial>(),
        std::any::TypeId::of::<PlaySpeedDial>(),
        std::any::TypeId::of::<PositionDial>(),
        std::any::TypeId::of::<AudioChannelsDial>(),
        std::any::TypeId::of::<RawDial>(),
    );
}
