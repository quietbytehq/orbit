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
  remotes: string[];
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

/** Mirrors `jobs::JobKind`. */
export type JobKind = "sync" | "copy";

/** Mirrors `jobs::RunOutcome`. */
export type RunOutcome =
  | { state: "running" }
  | { state: "succeeded" }
  | { state: "failed"; message: string }
  | { state: "cancelled" };

/**
 * Mirrors `jobs::Schedule`. Schedules fire only while Orbit is running. `every` counts minutes
 * from the newest run; `daily` is a local time of day.
 */
export type Schedule =
  | { kind: "manual" }
  | { kind: "every"; minutes: number }
  | { kind: "daily"; hour: number; minute: number };

/** Mirrors `jobs::RunRecord`. */
export interface RunRecord {
  id: string;
  startedAt: number;
  finishedAt: number | null;
  outcome: RunOutcome;
  files: number;
  bytes: number;
  errors: number;
  logFile: string;
  /** True when the schedule started the run rather than the user. */
  scheduled: boolean;
}

/** Mirrors `jobs::Job`. */
export interface Job {
  id: string;
  name: string;
  kind: JobKind;
  source: Endpoint;
  destination: Endpoint;
  enabled: boolean;
  schedule: Schedule;
  runs: RunRecord[];
}

/** Mirrors `jobs::Progress`. */
export interface JobProgress {
  runId: string;
  bytes: number;
  totalBytes: number;
  files: number;
  errors: number;
  speed: number;
  etaSeconds: number | null;
  currentFile: string | null;
}

/** Mirrors `jobs::JobView`. */
export interface JobView extends Job {
  progress: JobProgress | null;
  /** Unix seconds of the next automatic run. Null for manual or disabled jobs. */
  nextRunAt: number | null;
}

/** Mirrors `jobs::JobInput`. */
export interface JobInput {
  name: string;
  kind: JobKind;
  source: Endpoint;
  destination: Endpoint;
  schedule: Schedule;
}

/** Mirrors `mounts::MountInfo`. */
export interface MountInfo {
  remote: string;
  mountPoint: string;
  mountedOn: string | null;
}

/** Mirrors `mounts::WinFspStatus`. */
export type WinFspStatus = { state: "notApplicable" } | { state: "installed" } | { state: "missing" };

/** Mirrors `mounts::MountSupport`. */
export interface MountSupport {
  platform: "linux" | "windows" | "other";
  available: boolean;
  winfsp: WinFspStatus;
  requirement: string | null;
}

/** Mirrors `settings::Theme`. */
export type Theme = "system" | "light" | "dark";

/** Mirrors `settings::LogLevel`. */
export type RcloneLogLevel = "ERROR" | "NOTICE" | "INFO" | "DEBUG";

/** Mirrors `settings::SplashMode`. */
export type SplashMode = "full" | "short" | "off";

/** Mirrors `settings::SplashPlan`: what the splash window plays this launch. */
export type SplashPlan = "full" | "short" | "off";

/** Mirrors `settings::Settings`. Holds no credentials. */
export interface Settings {
  theme: Theme;
  transferConcurrency: number;
  rclonePath: string | null;
  rcloneLogLevel: RcloneLogLevel;
  splash: SplashMode;
  /** App version that last played the full intro. Managed by the backend. */
  lastSplashVersion: string | null;
}
