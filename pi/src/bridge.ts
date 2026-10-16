import type { GlobalSettings } from "./generated/contracts";

export type Status =
  | { kind: "disabled" }
  | { kind: "connecting" }
  | { kind: "connected" }
  | { kind: "disconnected"; message?: string }
  | { kind: string; message?: string };

export type LiveInstance = {
  id: string;
  name?: string;
  status?: Status;
  model?: string;
};

export type CatalogClip = { id: string; name: string };

export type CatalogDeck = {
  id: string;
  name?: string;
  clips?: CatalogClip[];
  diskClips?: CatalogClip[];
  slot?: Record<string, string>;
  nas?: string;
  externalDrive?: string;
};

export type HyperDeckConfigBridge = {
  getSettings: () => GlobalSettings;
  setSettings: (settings: GlobalSettings) => void;
  reconnect: (id?: string) => void;
  subscribe: (listener: (instances: LiveInstance[]) => void) => () => void;
};

declare global {
  interface Window {
    hyperdeckConfig?: HyperDeckConfigBridge;
    refreshHyperdeckConfig?: (settings: GlobalSettings, instances: LiveInstance[]) => void;
  }
}
