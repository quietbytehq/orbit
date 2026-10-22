// Mirrors the serde types in src-tauri/src. Keep in sync when either side changes.

/** Health of the rclone daemon. Mirrors `rclone::supervisor::RcdState`. */
export type RcdState =
  | { state: "starting" }
  | { state: "healthy"; version: string }
  | { state: "restarting"; attempt: number }
  | { state: "failed"; message: string }
  | { state: "stopped" };

/** The one state shown by the tray. Mirrors `status::OverallStatus`. */
export type OverallStatus = "synced" | "syncing" | "disconnected";

/** Mirrors `error::ErrorKind`. */
export type ErrorKind = "notRunning" | "rclone" | "invalidInput" | "io" | "unsupported" | "internal";

/** Error shape returned by every command. Mirrors `error::AppError`. */
export interface AppError {
  kind: ErrorKind;
  message: string;
}

export type AuthKind = "browser" | "credentials";

export interface ChoiceOption {
  value: string;
  label: string;
}

/** Mirrors `providers::FieldKind`. */
export type FieldKind =
  | { kind: "text" }
  | { kind: "secret" }
  | { kind: "number"; min: number; max: number }
  | { kind: "choice"; options: ChoiceOption[] };

/** Mirrors `providers::FieldDef`. `key` is also the rclone parameter name. */
export interface FieldDef {
  key: string;
  label: string;
  help: string;
  kind: FieldKind;
  required: boolean;
  default: string | null;
}

/** Mirrors `providers::ProviderDef`. */
export interface ProviderDef {
  id: string;
  title: string;
  rcloneType: string;
  tagline: string;
  auth: AuthKind;
  fields: FieldDef[];
}

/** Mirrors `remotes::health::RemoteHealth`. */
export type RemoteHealth = { state: "unknown" } | { state: "ok" } | { state: "failed"; message: string };

/** Mirrors `remotes::RemoteSummary`. */
export interface RemoteSummary {
  name: string;
  rcloneType: string;
  providerId: string | null;
  providerTitle: string;
  health: RemoteHealth;
}

/** Mirrors `remotes::RemoteForm`. Secret values are never included. */
export interface RemoteForm {
  name: string;
  providerId: string;
  values: Record<string, string>;
  savedSecrets: string[];
}

/** Mirrors `remotes::ConnectionTest`. */
export interface ConnectionTest {
  ok: boolean;
  message: string;
}

/** Mirrors `remotes::commands::NewRemote`. */
export interface NewRemote {
  providerId: string;
  name: string;
  values: Record<string, string>;
}

/** Mirrors `files::Endpoint`. Remote paths are relative to the remote root. */
export type Endpoint = { kind: "remote"; remote: string; path: string } | { kind: "local"; path: string };

/** Mirrors `files::FileEntry`. */
export interface FileEntry {
  name: string;
  path: string;
  isDir: boolean;
  size: number;
  modTime: string | null;
  mimeType: string | null;
}

/** Mirrors `files::transfers::Mode`. */
export type TransferMode = "copy" | "move";

/** Mirrors `files::transfers::TransferRequest`. */
export interface TransferRequest {
  source: Endpoint;
  destination: Endpoint;
  mode: TransferMode;
  name: string;
  isDir: boolean;
}

/** Mirrors `files::transfers::Direction`. */
export type TransferDirection = "upload" | "download" | "copy" | "move";

/** Mirrors `files::transfers::State`. */
export type TransferState =
  | { state: "queued" }
  | { state: "running" }
  | { state: "done" }
  | { state: "failed"; message: string }
  | { state: "cancelled" };

/** Mirrors `files::transfers::Item`. */
export interface TransferItem {
  id: number;
  name: string;
  direction: TransferDirection;
  isDir: boolean;
  from: string;
  to: string;
  state: TransferState;
  bytes: number;
  totalBytes: number;
  speed: number;
  etaSeconds: number | null;
}

/** Mirrors `files::local::LocalEntry`. */
export interface LocalEntry {
  parent: string;
  name: string;
  isDir: boolean;
}

/** An item named in a delete request. */
export interface ItemRef {
  path: string;
  isDir: boolean;
}
