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
