import { useEffect, useState } from "react";
import type { LiveInstance } from "./bridge";
import type { GlobalSettings, InstanceConfig, TargetGroup } from "./generated/contracts";

const PALETTE = ["#4c8dff", "#ef5b5b", "#3cba7a", "#e2b15a", "#b07cff", "#4ec8d4", "#f08bbd", "#9aa4b5"];

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

  useEffect(() => {
    window.refreshHyperdeckConfig = (next, instances) => {
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
      setSettings(bridge.getSettings());
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

  const commit = (next: GlobalSettings) => {
    setSettings(next);
    window.opener?.hyperdeckConfig?.setSettings(next);
  };

  const instances = settings.instances ?? [];
  const updateInstances = (next: InstanceConfig[]) => commit({ ...settings, instances: next });

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
              onChange={(next) => {
                const copy = instances.slice();
                copy[index] = next;
                updateInstances(copy);
              }}
              onRemove={() => updateInstances(instances.filter((_, item) => item !== index))}
              onReconnect={() => window.opener?.hyperdeckConfig?.reconnect(instance.id)}
            />
          ))}
        </div>
        <div className="card-actions">
          <button
            type="button"
            onClick={() =>
              updateInstances([
                ...instances,
                {
                  id: crypto.randomUUID(),
                  name: `HyperDeck ${instances.length + 1}`,
                  host: "127.0.0.1",
                  port: 9993,
                  color: PALETTE[instances.length % PALETTE.length],
                  enabled: true,
                  username: "",
                  password: ""
                }
              ])
            }
          >
            Add HyperDeck
          </button>
          <button type="button" onClick={() => window.opener?.hyperdeckConfig?.reconnect()}>
            Reconnect all
          </button>
        </div>
        <Groups instances={instances} groups={settings.groups ?? []} setGroups={(groups) => commit({ ...settings, groups })} />
        <div className="sdpi-heading">Plugin</div>
        <div className="sdpi-item">
          <div className="sdpi-item-label">Hold</div>
          <input
            className="sdpi-item-value"
            type="number"
            value={settings.longPressMs}
            aria-label="Long press milliseconds"
            onChange={(event) => commit({ ...settings, longPressMs: Number(event.target.value) })}
          />
        </div>
        <div className="sdpi-item">
          <div className="sdpi-item-label">Foreground</div>
          <input
            className="sdpi-item-value"
            type="color"
            value={settings.fgColor}
            aria-label="Foreground"
            onChange={(event) => commit({ ...settings, fgColor: event.target.value })}
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
  setGroups
}: {
  instances: InstanceConfig[];
  groups: TargetGroup[];
  setGroups: (groups: TargetGroup[]) => void;
}) {
  return (
    <>
      <div className="sdpi-heading">Groups</div>
      {groups.map((group, index) => (
        <section className="obs-card" key={group.id}>
          <label className="field">
            Name
            <input
              value={group.name}
              onChange={(event) => {
                const copy = groups.slice();
                copy[index] = { ...group, name: event.target.value };
                setGroups(copy);
              }}
            />
          </label>
          {instances.map((instance) => (
            <label className="field check" key={instance.id}>
              <input
                type="checkbox"
                checked={group.members.includes(instance.id)}
                onChange={(event) => {
                  const members = event.target.checked
                    ? [...group.members, instance.id]
                    : group.members.filter((id) => id !== instance.id);
                  const copy = groups.slice();
                  copy[index] = { ...group, members };
                  setGroups(copy);
                }}
              />
              {instance.name || instance.host}
            </label>
          ))}
          <div className="card-actions">
            <button type="button" onClick={() => setGroups(groups.filter((_, item) => item !== index))}>
              Remove group
            </button>
          </div>
        </section>
      ))}
      <div className="card-actions">
        <button
          type="button"
          onClick={() =>
            setGroups([...groups, { id: crypto.randomUUID(), name: `Group ${groups.length + 1}`, members: [] }])
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
