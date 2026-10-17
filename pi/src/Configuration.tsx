import { useEffect, useRef, useState } from "react";
import type { LiveInstance } from "./bridge";
import type { GlobalSettings, InstanceConfig, TargetGroup } from "./generated/contracts";

const PALETTE = ["#4c8dff", "#ef5b5b", "#3cba7a", "#e2b15a", "#b07cff", "#4ec8d4", "#f08bbd", "#9aa4b5"];
const DEFAULT_UPDATE_MS = 200;
const MIN_UPDATE_MS = 50;
const MAX_UPDATE_MS = 5000;

function updateInterval(value: number | undefined): number {
  if (value == null || !Number.isFinite(value) || value <= 0) return DEFAULT_UPDATE_MS;
  return Math.min(MAX_UPDATE_MS, Math.max(MIN_UPDATE_MS, Math.round(value)));
}

const emptySettings = (): GlobalSettings => ({
  instances: [],
  groups: [],
  longPressMs: 500,
  fgColor: "#f4f7fb",
  seeded: true
});

export function Configuration() {
  const [settings, setSettings] = useState<GlobalSettings>(emptySettings);
  const [statuses, setStatuses] = useState<LiveInstance[]>([]);
  const [ready, setReady] = useState(false);
  const [missing, setMissing] = useState(false);
  const settingsRef = useRef(settings);

  const commit = (recipe: (current: GlobalSettings) => GlobalSettings) => {
    const next = recipe(settingsRef.current);
    settingsRef.current = next;
    setSettings(next);
    window.opener?.hyperdeckConfig?.setSettings(next);
  };

  useEffect(() => {
    window.refreshHyperdeckConfig = (next, instances) => {
      settingsRef.current = next;
      setSettings(next);
      setStatuses(instances);
      setReady(true);
    };
    if (!window.opener) {
      setMissing(true);
      return () => {
        delete window.refreshHyperdeckConfig;
      };
    }
    let unsubscribe = () => {};
    const attach = () => {
      const bridge = window.opener?.hyperdeckConfig;
      if (!bridge) return false;
      const loaded = bridge.getSettings();
      settingsRef.current = loaded;
      setSettings(loaded);
      unsubscribe = bridge.subscribe(setStatuses);
      setReady(true);
      return true;
    };
    if (attach()) {
      return () => {
        unsubscribe();
        delete window.refreshHyperdeckConfig;
      };
    }
    const timer = window.setInterval(() => {
      if (attach()) window.clearInterval(timer);
    }, 200);
    return () => {
      window.clearInterval(timer);
      unsubscribe();
      delete window.refreshHyperdeckConfig;
    };
  }, []);

  const instances = settings.instances ?? [];

  if (missing) {
    return (
      <main>
        <header className="config-header">
          <h1>HyperDeck [MikanseiLaboratory]</h1>
        </header>
        <p className="empty-note">Open this window from a key's property inspector.</p>
      </main>
    );
  }

  return (
    <main>
      <header className="config-header">
        <h1>HyperDeck [MikanseiLaboratory]</h1>
      </header>
      <div className="config-body">
        <div className="sdpi-heading">HyperDeck instances</div>
        {!ready && <p className="empty-note">Waiting for the property inspector…</p>}
        {ready && instances.length === 0 && (
          <p className="empty-note">No HyperDeck instances yet. Add one to start controlling it from a key.</p>
        )}
        <div className="obs-grid">
          {instances.map((instance, index) => (
            <InstanceCard
              key={instance.id}
              instance={instance}
              index={index}
              status={statuses.find((item) => item.id === instance.id)}
              onChange={(next) =>
                commit((current) => ({
                  ...current,
                  instances: current.instances.map((item) => (item.id === instance.id ? next : item))
                }))
              }
              onRemove={() =>
                commit((current) => ({
                  ...current,
                  instances: current.instances.filter((item) => item.id !== instance.id),
                  groups: (current.groups ?? []).map((group) => ({
                    ...group,
                    members: group.members.filter((id) => id !== instance.id)
                  }))
                }))
              }
              onReconnect={() => window.opener?.hyperdeckConfig?.reconnect(instance.id)}
            />
          ))}
        </div>
        <div className="card-actions">
          <button
            type="button"
            onClick={() =>
              commit((current) => ({
                ...current,
                instances: [
                  ...current.instances,
                  {
                    id: crypto.randomUUID(),
                    name: `HyperDeck ${current.instances.length + 1}`,
                    host: "127.0.0.1",
                    port: 9993,
                    color: PALETTE[current.instances.length % PALETTE.length],
                    enabled: true,
                    username: "",
                    password: "",
                    updateIntervalMs: DEFAULT_UPDATE_MS
                  }
                ]
              }))
            }
            disabled={!ready}
          >
            Add HyperDeck
          </button>
          <button type="button" disabled={!ready} onClick={() => window.opener?.hyperdeckConfig?.reconnect()}>
            Reconnect all
          </button>
        </div>
        <Groups
          instances={instances}
          groups={settings.groups ?? []}
          ready={ready}
          setGroups={(recipe) => commit((current) => ({ ...current, groups: recipe(current.groups ?? []) }))}
        />
        <div className="sdpi-heading">Plugin</div>
        <div className="sdpi-item">
          <div className="sdpi-item-label">Hold</div>
          <input
            className="sdpi-item-value"
            type="number"
            value={settings.longPressMs}
            aria-label="Long press milliseconds"
            onChange={(event) => commit((current) => ({ ...current, longPressMs: Number(event.target.value) }))}
          />
        </div>
        <div className="sdpi-item">
          <div className="sdpi-item-label">Foreground</div>
          <input
            className="sdpi-item-value"
            type="color"
            value={settings.fgColor}
            aria-label="Foreground"
            onChange={(event) => commit((current) => ({ ...current, fgColor: event.target.value }))}
          />
        </div>
      </div>
    </main>
  );
}

function InstanceCard({
  instance,
  index,
  status,
  onChange,
  onRemove,
  onReconnect
}: {
  instance: InstanceConfig;
  index: number;
  status?: LiveInstance;
  onChange: (instance: InstanceConfig) => void;
  onRemove: () => void;
  onReconnect: () => void;
}) {
  const kind = instance.enabled ? status?.status?.kind : "disabled";
  return (
    <section className="obs-card">
      <div className="obs-card-head">
        <span className={`status-dot ${kind ?? ""}`} title={kind ?? "unknown"} />
        <strong>{instance.name || "HyperDeck"}</strong>
        <span className="status-text">{statusLabel(kind, status?.status)}</span>
        <input type="color" value={instance.color} aria-label="Color" onChange={(event) => onChange({ ...instance, color: event.target.value })} />
      </div>
      <label className="field">
        Name
        <input value={instance.name} onChange={(event) => onChange({ ...instance, name: event.target.value })} />
      </label>
      <label className="field">
        Host
        <input value={instance.host} onChange={(event) => onChange({ ...instance, host: event.target.value })} />
      </label>
      <label className="field">
        Port
        <input
          type="number"
          value={instance.port}
          onChange={(event) => onChange({ ...instance, port: Number(event.target.value) })}
        />
      </label>
      <label className="field">
        Username
        <input value={instance.username} onChange={(event) => onChange({ ...instance, username: event.target.value })} />
      </label>
      <label className="field">
        Password
        <input
          type="password"
          value={instance.password}
          onChange={(event) => onChange({ ...instance, password: event.target.value })}
        />
      </label>
      <label className="field check">
        <input
          type="checkbox"
          checked={instance.enabled}
          onChange={(event) => onChange({ ...instance, enabled: event.target.checked })}
        />
        Enabled
      </label>
      <label className="field">
        Update ms
        <input
          type="number"
          min={MIN_UPDATE_MS}
          max={MAX_UPDATE_MS}
          step={50}
          value={instance.updateIntervalMs ?? DEFAULT_UPDATE_MS}
          aria-label="Update interval milliseconds"
          title="How often this deck is polled, from 50 to 5000 ms"
          onChange={(event) => {
            const value = Number(event.target.value);
            if (!Number.isFinite(value)) return;
            onChange({ ...instance, updateIntervalMs: Math.min(MAX_UPDATE_MS, Math.max(0, Math.round(value))) });
          }}
          onBlur={(event) => onChange({ ...instance, updateIntervalMs: updateInterval(Number(event.target.value)) })}
        />
      </label>
      <div className="card-actions">
        <button type="button" onClick={onReconnect}>
          Reconnect
        </button>
        <button type="button" onClick={onRemove}>
          Remove {index + 1}
        </button>
      </div>
    </section>
  );
}

function Groups({
  instances,
  groups,
  ready,
  setGroups
}: {
  instances: InstanceConfig[];
  groups: TargetGroup[];
  ready: boolean;
  setGroups: (recipe: (groups: TargetGroup[]) => TargetGroup[]) => void;
}) {
  return (
    <>
      <div className="sdpi-heading">Groups</div>
      {ready && groups.length === 0 && <p className="empty-note">No groups yet. Add one to send a key to several decks.</p>}
      {groups.map((group) => (
        <section className="group-card" key={group.id}>
          <div className="sdpi-item">
            <div className="sdpi-item-label">Name</div>
            <input
              className="sdpi-item-value"
              value={group.name}
              onChange={(event) => {
                const name = event.target.value;
                setGroups((current) => current.map((item) => (item.id === group.id ? { ...item, name } : item)));
              }}
            />
          </div>
          <div type="checkbox" className="sdpi-item targets">
            <div className="sdpi-item-label">Members</div>
            <div className="sdpi-item-value">
              {instances.length === 0 && <span>No decks yet</span>}
              {instances.map((instance) => {
                const id = `group-${group.id}-${instance.id}`;
                const on = group.members.includes(instance.id);
                return (
                  <div className="sdpi-item-child" key={instance.id}>
                    <input
                      id={id}
                      type="checkbox"
                      checked={on}
                      onChange={() => {
                        setGroups((current) =>
                          current.map((item) => {
                            if (item.id !== group.id) return item;
                            const members = on
                              ? item.members.filter((member) => member !== instance.id)
                              : item.members.includes(instance.id)
                                ? item.members
                                : [...item.members, instance.id];
                            return { ...item, members };
                          })
                        );
                      }}
                    />
                    <label htmlFor={id}>
                      <span></span>
                      <i className="swatch" style={{ background: instance.color }} />
                      {instance.name || instance.host}
                    </label>
                  </div>
                );
              })}
            </div>
          </div>
          <div className="card-actions">
            <button type="button" onClick={() => setGroups((current) => current.filter((item) => item.id !== group.id))}>
              Remove group
            </button>
          </div>
        </section>
      ))}
      <div className="card-actions">
        <button
          type="button"
          disabled={!ready}
          onClick={() =>
            setGroups((current) => [
              ...current,
              { id: crypto.randomUUID(), name: `Group ${current.length + 1}`, members: [] }
            ])
          }
        >
          Add group
        </button>
      </div>
    </>
  );
}

function statusLabel(kind: string | undefined, status: LiveInstance["status"]) {
  if (kind === "disconnected" && status && "message" in status && status.message) return status.message;
  if (kind === "connected") return "Connected";
  if (kind === "connecting") return "Connecting";
  if (kind === "disabled") return "Disabled";
  return kind ?? "Unknown";
}
