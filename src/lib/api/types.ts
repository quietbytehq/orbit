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
