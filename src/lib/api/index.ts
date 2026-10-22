import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { AppError, RcdState } from "./types";

export type * from "./types";

export const RCD_STATE_EVENT = "rcd://state";

/** Typed wrappers around Tauri commands. Each one returns a promise that rejects with an AppError. */
export const api = {
  getRcdState: () => invoke<RcdState>("get_rcd_state"),
  restartRcd: () => invoke<void>("restart_rcd"),
  splashFinished: () => invoke<void>("splash_finished"),
};

export function onRcdState(handler: (state: RcdState) => void): Promise<UnlistenFn> {
  return listen<RcdState>(RCD_STATE_EVENT, (event) => handler(event.payload));
}

export function isAppError(value: unknown): value is AppError {
  return (
    typeof value === "object" &&
    value !== null &&
    "kind" in value &&
    "message" in value &&
    typeof (value as { message: unknown }).message === "string"
  );
}

/** Turns anything thrown by a command into text fit for the UI. */
export function errorMessage(err: unknown): string {
  if (isAppError(err)) return err.message;
  if (err instanceof Error) return err.message;
  return String(err);
}
