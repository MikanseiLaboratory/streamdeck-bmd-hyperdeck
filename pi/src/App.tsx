import { useEffect, useRef, useState, type ReactNode } from "react";
import {
  useGlobalSettings,
  usePluginMessage,
  useSendToPlugin,
  useSettings,
  useStreamDeck
} from "@mikanseilaboratory/streamdeck-pi-client";
import type { CatalogDeck, HyperDeckConfigBridge, LiveInstance } from "./bridge";
import type {
  ActionParams,
  ActionSettings,
  GlobalSettings,
  InstanceConfig,
  TargetGroup,
  TargetSelector
} from "./generated/contracts";

type Settings = ActionSettings & Record<string, unknown>;
type Globals = GlobalSettings & Record<string, unknown>;

const CONFIG_WINDOW = "hyperdeck-config";

const emptyParams = (): ActionParams => ({
  speed: 100,
  loopPlayback: false,
  singleClip: false,
  name: "",
  slotId: "",
  device: "",
  clipId: "",
  commandMode: "select",
  rangeCount: "1",
  rangeIn: "",
  rangeOut: "",
  gotoMode: "timeline-start",
  timecode: "",
  mode: "playback",
  value: "",
  thresholdSeconds: 60,
  statusField: "status",
  raw: "",
  rawRotateUp: "",
  rawRotateDown: "",
  rawPush: "",
  rawTouch: "",
  step: 0,
  direction: "next",
  activeColor: "",
  confirm: false,
  slateReel: "",
  slateScene: "",
  slateTake: "",
  slateProject: "",
  slateLens: "",
  nasUrl: "",
  nasUser: "",
  nasPassword: ""
});

const globalDefaults: Globals = {
  instances: [],
  groups: [],
  longPressMs: 500,
  fgColor: "#f4f7fb",
  seeded: false
};

const actionDefaults: Settings = {
  common: { target: { kind: "all" }, sharedParams: true },
  advanced: { longPress: false, longPressMs: 0 },
  shared: emptyParams(),
  params: {}
};

const LONG_PRESS = new Set(["record", "playrange", "clip"]);

const DIALS = new Set([
  "shuttle",
  "jog",
  "cliplist",
  "playspeed",
  "position",
  "audiochannels",
  "rawdial"
]);

function defaultStep(kind: string): number {
  if (kind === "shuttle") {
    return 100;
  }
  if (kind === "audiochannels") {
    return 2;
  }
  return 1;
}

export function App() {
  const deck = useStreamDeck();
  const kind = deck.actionInfo?.action.split(".").at(-1) ?? "play";
  const global = useGlobalSettings<Globals>(globalDefaults);
  const action = useSettings<Settings>(actionDefaults);
  const send = useSendToPlugin();
  const [statuses, setStatuses] = useState<LiveInstance[]>([]);
  const [decks, setDecks] = useState<CatalogDeck[]>([]);
  const settingsRef = useRef(global.settings);
  const statusRef = useRef(statuses);
  const listenersRef = useRef(new Set<(instances: LiveInstance[]) => void>());
  const configWindowRef = useRef<Window | null>(null);
  const sendRef = useRef(send);
  settingsRef.current = global.settings;
  statusRef.current = statuses;
  sendRef.current = send;

  usePluginMessage((payload: { type?: string; instances?: LiveInstance[]; decks?: CatalogDeck[] }) => {
    if (payload.type === "status" && payload.instances) setStatuses(payload.instances);
    if (payload.type === "catalog" && payload.decks) setDecks(payload.decks);
  });

  useEffect(() => {
    const bridge: HyperDeckConfigBridge = {
      getSettings: () => settingsRef.current,
      setSettings: (next) => global.setSettings(next as Globals),
      reconnect: (id) => sendRef.current({ type: "reconnect", id }),
      subscribe: (listener) => {
        listenersRef.current.add(listener);
        listener(statusRef.current);
        return () => listenersRef.current.delete(listener);
      }
    };
    window.hyperdeckConfig = bridge;
    return () => {
      if (window.hyperdeckConfig === bridge) delete window.hyperdeckConfig;
    };
  }, [global.setSettings]);

  useEffect(() => {
    for (const listener of listenersRef.current) listener(statuses);
  }, [statuses]);

  useEffect(() => {
    const child = configWindowRef.current;
    if (child && !child.closed && child.opener === window) {
      child.refreshHyperdeckConfig?.(settingsRef.current, statusRef.current);
    }
  }, [global.settings]);

  useEffect(() => {
    send({ type: "ready" });
    const retry = window.setTimeout(() => send({ type: "ready" }), 250);
    return () => window.clearTimeout(retry);
  }, [send]);

  const instances = global.settings.instances ?? [];
  const groups = global.settings.groups ?? [];
  const target = action.settings.common?.target ?? { kind: "all" as const };

  const openConfig = () => {
    const features = "width=760,height=840";
    const existing = window.open("", CONFIG_WINDOW);
    if (!existing) return;
    let owned = false;
    try {
      owned = existing.opener === window;
    } catch {
      owned = false;
    }
    if (!owned) {
      existing.close();
      configWindowRef.current = window.open("./configuration.html", CONFIG_WINDOW, features);
      return;
    }
    const href = existing.location.href;
    if (href.includes("configuration.html")) {
      existing.focus();
      existing.refreshHyperdeckConfig?.(settingsRef.current, statusRef.current);
      configWindowRef.current = existing;
      return;
    }
    existing.location.href = new URL("./configuration.html", window.location.href).href;
    configWindowRef.current = existing;
  };

  return (
    <div className="sdpi-wrapper">
      <div className="sdpi-heading">Target</div>
      <TargetPicker
        instances={instances}
        groups={groups}
        statuses={statuses}
        value={target}
        onChange={(next) =>
          action.setSettings((previous) => ({
            ...previous,
            common: { ...previous.common, target: next }
          }))
        }
      />
      <CheckRow
        label="Parameters"
        checked={action.settings.common?.sharedParams !== false}
        text="Same values for every target"
        onChange={(checked) =>
          action.setSettings((previous) => ({
            ...previous,
            common: { ...previous.common, sharedParams: checked }
          }))
        }
      />
      <Fields
        kind={kind}
        instances={instances}
        target={target}
        groups={groups}
        settings={action.settings}
        setSettings={action.setSettings}
        decks={decks}
      />
      <div className="sdpi-heading">Connections</div>
      <div className="sdpi-item">
        <div className="sdpi-item-label">HyperDeck</div>
        <button className="sdpi-item-value" type="button" onClick={openConfig}>
          Manage instances
        </button>
      </div>
    </div>
  );
}

function TargetPicker({
  instances,
  groups,
  statuses,
  value,
  onChange
}: {
  instances: InstanceConfig[];
  groups: TargetGroup[];
  statuses: LiveInstance[];
  value: TargetSelector;
  onChange: (value: TargetSelector) => void;
}) {
  const [preferMultiple, setPreferMultiple] = useState(value.kind === "instances" && value.ids.length !== 1);
  useEffect(() => {
    if (value.kind !== "instances") setPreferMultiple(false);
  }, [value]);
  const mode = preferMultiple ? "multiple" : targetMode(value);
  const selected = value.kind === "instances" ? value.ids : [];
  const missingGroup = value.kind === "group" && !groups.some((group) => group.id === value.id);
  const missingInstance =
    value.kind === "instances" && value.ids.length === 1 && !instances.some((instance) => instance.id === value.ids[0]);
  return (
    <>
      <div type="select" className="sdpi-item">
        <div className="sdpi-item-label">Send to</div>
        <select
          className="sdpi-item-value select"
          value={mode}
          onChange={(event) => {
            const next = event.target.value;
            setPreferMultiple(next === "multiple");
            onChange(selectorFromMode(next, selected));
          }}
        >
          <option value="all">All enabled</option>
          {groups.map((group) => (
            <option key={group.id} value={`group:${group.id}`}>
              Group: {group.name || "Untitled"}
            </option>
          ))}
          {missingGroup && value.kind === "group" && <option value={`group:${value.id}`}>Missing group</option>}
          {instances.map((instance) => (
            <option key={instance.id} value={`instance:${instance.id}`}>
              {instance.name || instance.host}
              {instance.enabled ? "" : " (disabled)"}
            </option>
          ))}
          {missingInstance && value.kind === "instances" && (
            <option value={`instance:${value.ids[0]}`}>Missing deck</option>
          )}
          <option value="multiple">Multiple decks</option>
        </select>
      </div>
      {mode === "multiple" && (
        <div type="checkbox" className="sdpi-item targets">
          <div className="sdpi-item-label">Decks</div>
          <div className="sdpi-item-value">
            {instances.length === 0 && <span>No decks yet</span>}
            {instances.map((instance) => {
              const id = `target-${instance.id}`;
              const on = selected.includes(instance.id);
              const status = statuses.find((item) => item.id === instance.id)?.status?.kind;
              return (
                <div className="sdpi-item-child" key={instance.id}>
                  <input
                    id={id}
                    type="checkbox"
                    checked={on}
                    onChange={() => {
                      const ids = on ? selected.filter((item) => item !== instance.id) : [...selected, instance.id];
                      onChange({ kind: "instances", ids });
                    }}
                  />
                  <label htmlFor={id}>
                    <span></span>
                    <i className="swatch" style={{ background: statusColor(status, instance.color) }} />
                    {instance.name || instance.host}
                  </label>
                </div>
              );
            })}
          </div>
        </div>
      )}
    </>
  );
}

function Fields({
  kind,
  instances,
  target,
  groups,
  settings,
  setSettings,
  decks
}: {
  kind: string;
  instances: InstanceConfig[];
  target: TargetSelector;
  groups: TargetGroup[];
  settings: Settings;
  setSettings: (next: Settings | ((previous: Settings) => Settings)) => void;
  decks: CatalogDeck[];
}) {
  const shared = settings.common?.sharedParams !== false;
  const targeted = targetedIds(instances, groups, target);
  const [editor, setEditor] = useState(targeted[0] ?? "");
  const activeEditor = targeted.includes(editor) ? editor : (targeted[0] ?? "");
  const params = shared ? settings.shared : settings.params?.[activeEditor] ?? settings.shared;
  const clips = uniqueClips(decks);
  const update = (patch: Partial<ActionParams>) => {
    setSettings((previous) => {
      if (previous.common?.sharedParams !== false) {
        return { ...previous, shared: { ...previous.shared, ...patch } };
      }
      return {
        ...previous,
        params: {
          ...previous.params,
          [activeEditor]: { ...(previous.params?.[activeEditor] ?? previous.shared), ...patch }
        }
      };
    });
  };
  return (
    <>
      {!shared && (
        <SelectRow label="Edit" value={activeEditor} onChange={setEditor}>
          {targeted.map((id) => (
            <option key={id} value={id}>
              {instances.find((item) => item.id === id)?.name || id}
            </option>
          ))}
        </SelectRow>
      )}
      {kind === "play" && (
        <>
          <NumberRow label="Speed" value={params.speed} onChange={(speed) => update({ speed })} />
          <CheckRow label="Loop" checked={params.loopPlayback} text="Loop" onChange={(loopPlayback) => update({ loopPlayback })} />
          <CheckRow label="Single" checked={params.singleClip} text="Single clip" onChange={(singleClip) => update({ singleClip })} />
        </>
      )}
      {(kind === "record" || kind === "recordnamed" || kind === "recordprefix" || kind === "clipsadd") && (
        <TextRow label="Name" value={params.name} onChange={(name) => update({ name })} />
      )}
      {(kind === "slot" || kind === "slotunblock") && (
        <TextRow label="Slot" value={params.slotId} onChange={(slotId) => update({ slotId })} />
      )}
      {kind === "slot" && <TextRow label="Device" value={params.device} onChange={(device) => update({ device })} />}
      {(kind === "clip" || kind === "playrange" || kind === "clipsadd" || kind === "clipsremove") && (
        <ClipRow value={params.clipId} clips={clips} onChange={(clipId) => update({ clipId })} />
      )}
      {kind === "clipstep" && (
        <SelectRow label="Step" value={params.direction} onChange={(direction) => update({ direction })}>
          <option value="next">Next</option>
          <option value="previous">Previous</option>
        </SelectRow>
      )}
      {kind === "playrange" && (
        <>
          <TextRow label="Count" value={params.rangeCount} onChange={(rangeCount) => update({ rangeCount })} />
          <TextRow label="In" value={params.rangeIn} onChange={(rangeIn) => update({ rangeIn })} />
          <TextRow label="Out" value={params.rangeOut} onChange={(rangeOut) => update({ rangeOut })} />
        </>
      )}
      {kind === "goto" && (
        <>
          <SelectRow label="Target" value={params.gotoMode} onChange={(gotoMode) => update({ gotoMode })}>
            <option value="timeline-start">Timeline start</option>
            <option value="timeline-end">Timeline end</option>
            <option value="clip-start">Clip start</option>
            <option value="clip-end">Clip end</option>
            <option value="timecode">Timecode</option>
          </SelectRow>
          {params.gotoMode === "timecode" && (
            <TextRow label="Timecode" value={params.timecode} onChange={(timecode) => update({ timecode })} />
          )}
        </>
      )}
      {kind === "stopmode" && (
        <SelectRow label="Mode" value={params.value || "lastframe"} onChange={(value) => update({ value })}>
          <option value="lastframe">Last frame</option>
          <option value="nextframe">Next frame</option>
          <option value="black">Black</option>
        </SelectRow>
      )}
      {kind === "recordtrigger" && (
        <SelectRow label="Trigger" value={params.value || "none"} onChange={(value) => update({ value })}>
          <option value="none">None</option>
          <option value="recordbit">Record bit</option>
          <option value="timecoderun">Timecode run</option>
        </SelectRow>
      )}
      {["videoinput", "audioinput", "fileformat", "audiocodec", "timecodeinput", "timecodeoutput", "referencesource", "externaldrive"].includes(kind) && (
        <TextRow label="Value" value={params.value} onChange={(value) => update({ value })} />
      )}
      {kind === "dynamicrange" && (
        <>
          <SelectRow label="Side" value={params.mode || "playback"} onChange={(mode) => update({ mode })}>
            <option value="playback">Playback</option>
            <option value="record">Record</option>
          </SelectRow>
          <TextRow label="Override" value={params.value} onChange={(value) => update({ value })} />
        </>
      )}
      {kind === "nas" && (
        <>
          <SelectRow label="Action" value={params.commandMode || "select"} onChange={(commandMode) => update({ commandMode })}>
            <option value="select">Select</option>
            <option value="add">Add</option>
            <option value="remove">Remove</option>
            <option value="deselect">Deselect</option>
          </SelectRow>
          <TextRow label="URL" value={params.nasUrl} onChange={(nasUrl) => update({ nasUrl })} />
          <TextRow label="User" value={params.nasUser} onChange={(nasUser) => update({ nasUser })} />
          <TextRow label="Password" value={params.nasPassword} onChange={(nasPassword) => update({ nasPassword })} />
        </>
      )}
      {kind === "remaining" && (
        <NumberRow
          label="Threshold"
          value={params.thresholdSeconds}
          onChange={(thresholdSeconds) => update({ thresholdSeconds })}
        />
      )}
      {kind === "status" && (
        <SelectRow label="Field" value={params.statusField} onChange={(statusField) => update({ statusField })}>
          <option value="status">Status</option>
          <option value="speed">Speed</option>
          <option value="timecode">Timeline timecode</option>
          <option value="display timecode">Display timecode</option>
          <option value="clip">Clip</option>
          <option value="slot">Slot</option>
          <option value="video format">Video format</option>
          <option value="model">Model</option>
          <option value="protocol">Protocol</option>
          <option value="uptime">Uptime</option>
          <option value="cache">Cache</option>
        </SelectRow>
      )}
      {kind === "timecodepreset" && (
        <TextRow label="Preset" value={params.timecode} onChange={(timecode) => update({ timecode })} />
      )}
      {kind === "slate" && (
        <>
          <TextRow label="Reel" value={params.slateReel} onChange={(slateReel) => update({ slateReel })} />
          <TextRow label="Scene" value={params.slateScene} onChange={(slateScene) => update({ slateScene })} />
          <TextRow label="Take" value={params.slateTake} onChange={(slateTake) => update({ slateTake })} />
          <TextRow label="Project" value={params.slateProject} onChange={(slateProject) => update({ slateProject })} />
          <TextRow label="Lens" value={params.slateLens} onChange={(slateLens) => update({ slateLens })} />
        </>
      )}
      {kind === "clipsclear" && (
        <CheckRow
          label="Confirm"
          checked={params.confirm}
          text="Clear the timeline on press"
          onChange={(confirm) => update({ confirm })}
        />
      )}
      {kind === "raw" && <AreaRow label="Command" value={params.raw} onChange={(raw) => update({ raw })} />}
      {["shuttle", "jog", "cliplist", "playspeed", "position", "audiochannels"].includes(kind) && (
        <NumberRow
          label="Step"
          value={params.step === 0 ? defaultStep(kind) : params.step}
          onChange={(step) => update({ step: step === defaultStep(kind) ? 0 : step })}
        />
      )}
      {kind === "rawdial" && (
        <>
          <TextRow label="Clockwise" value={params.rawRotateUp} onChange={(rawRotateUp) => update({ rawRotateUp })} />
          <TextRow label="Counter" value={params.rawRotateDown} onChange={(rawRotateDown) => update({ rawRotateDown })} />
          <TextRow label="Push" value={params.rawPush} onChange={(rawPush) => update({ rawPush })} />
          <TextRow label="Touch" value={params.rawTouch} onChange={(rawTouch) => update({ rawTouch })} />
        </>
      )}
      {LONG_PRESS.has(kind) && (
        <CheckRow
          label="Hold"
          checked={settings.advanced?.longPress === true}
          text={kind === "record" ? "Hold to spill" : kind === "playrange" ? "Hold to clear" : "Hold to play the clip"}
          onChange={(longPress) =>
            setSettings((previous) => ({
              ...previous,
              advanced: { ...previous.advanced, longPress }
            }))
          }
        />
      )}
      {!DIALS.has(kind) && (
        <div className="sdpi-item">
          <div className="sdpi-item-label">Active</div>
          <input
            className="sdpi-item-value"
            type="text"
            placeholder="#4c8dff"
            value={params.activeColor}
            onChange={(event) => update({ activeColor: event.target.value })}
          />
        </div>
      )}
    </>
  );
}

function ClipRow({ value, clips, onChange }: { value: string; clips: { id: string; name: string }[]; onChange: (value: string) => void }) {
  if (clips.length === 0) {
    return <TextRow label="Clip" value={value} onChange={onChange} />;
  }
  const known = clips.some((clip) => clip.id === value);
  return (
    <SelectRow label="Clip" value={value} onChange={onChange}>
      <option value="">Select</option>
      {!known && value !== "" && <option value={value}>{value}</option>}
      {clips.map((clip) => (
        <option key={clip.id} value={clip.id}>
          {clip.name ? `${clip.id}: ${clip.name}` : clip.id}
        </option>
      ))}
    </SelectRow>
  );
}

function TextRow({ label, value, onChange }: { label: string; value: string; onChange: (value: string) => void }) {
  return (
    <div className="sdpi-item">
      <div className="sdpi-item-label">{label}</div>
      <input className="sdpi-item-value" type="text" value={value} onChange={(event) => onChange(event.target.value)} />
    </div>
  );
}

function NumberRow({ label, value, onChange }: { label: string; value: number; onChange: (value: number) => void }) {
  return (
    <div className="sdpi-item">
      <div className="sdpi-item-label">{label}</div>
      <input
        className="sdpi-item-value"
        type="number"
        value={Number.isFinite(value) ? value : 0}
        onChange={(event) => onChange(Number(event.target.value))}
      />
    </div>
  );
}

function AreaRow({ label, value, onChange }: { label: string; value: string; onChange: (value: string) => void }) {
  return (
    <div className="sdpi-item">
      <div className="sdpi-item-label">{label}</div>
      <textarea className="sdpi-item-value" value={value} onChange={(event) => onChange(event.target.value)} />
    </div>
  );
}

function SelectRow({
  label,
  value,
  onChange,
  children
}: {
  label: string;
  value: string;
  onChange: (value: string) => void;
  children: ReactNode;
}) {
  return (
    <div className="sdpi-item">
      <div className="sdpi-item-label">{label}</div>
      <select className="sdpi-item-value select" value={value} onChange={(event) => onChange(event.target.value)}>
        {children}
      </select>
    </div>
  );
}

function CheckRow({
  label,
  checked,
  text,
  onChange
}: {
  label: string;
  checked: boolean;
  text: string;
  onChange: (checked: boolean) => void;
}) {
  const id = `check-${label.replace(/\s+/g, "-").toLowerCase()}`;
  return (
    <div type="checkbox" className="sdpi-item">
      <div className="sdpi-item-label">{label}</div>
      <input id={id} className="sdpi-item-value" type="checkbox" checked={checked} onChange={(event) => onChange(event.target.checked)} />
      <label htmlFor={id}>
        <span></span>
        {text}
      </label>
    </div>
  );
}

function targetMode(value: TargetSelector) {
  if (value.kind === "all") return "all";
  if (value.kind === "group") return `group:${value.id}`;
  if (value.ids.length === 1) return `instance:${value.ids[0]}`;
  return "multiple";
}

function selectorFromMode(mode: string, selected: string[]): TargetSelector {
  if (mode === "all") return { kind: "all" };
  if (mode === "multiple") return { kind: "instances", ids: selected };
  if (mode.startsWith("group:")) return { kind: "group", id: mode.slice("group:".length) };
  if (mode.startsWith("instance:")) return { kind: "instances", ids: [mode.slice("instance:".length)] };
  return { kind: "all" };
}

function statusColor(kind: string | undefined, fallback: string) {
  switch (kind) {
    case "connected":
      return "#3cba7a";
    case "connecting":
      return "#e2b15a";
    case "disconnected":
      return "#8a4a3a";
    case "disabled":
      return "#6d7890";
    default:
      return fallback;
  }
}

function targetedIds(instances: InstanceConfig[], groups: TargetGroup[], target: TargetSelector): string[] {
  if (target.kind === "group") {
    const members = new Set(groups.find((item) => item.id === target.id)?.members ?? []);
    return instances.filter((instance) => members.has(instance.id)).map((instance) => instance.id);
  }
  if (target.kind === "instances") {
    const ids = new Set(target.ids);
    return instances.filter((instance) => ids.has(instance.id)).map((instance) => instance.id);
  }
  return instances.filter((instance) => instance.enabled).map((instance) => instance.id);
}

function uniqueClips(decks: CatalogDeck[]) {
  const seen = new Map<string, string>();
  for (const deck of decks) {
    for (const clip of deck.clips ?? []) {
      if (!seen.has(clip.id)) seen.set(clip.id, clip.name);
    }
  }
  return [...seen.entries()]
    .map(([id, name]) => ({ id, name }))
    .sort((left, right) => {
      const a = Number(left.id);
      const b = Number(right.id);
      if (Number.isFinite(a) && Number.isFinite(b) && a !== b) return a - b;
      return left.id.localeCompare(right.id);
    });
}
