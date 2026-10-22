import { api, onRcdState, errorMessage } from "$lib/api";
import type { RcdState } from "$lib/api/types";

/** Live view of the rclone daemon. Components read `state` and call `restart()`. */
class RcdStore {
  state = $state<RcdState>({ state: "starting" });
  error = $state<string | null>(null);
  #started = false;

  async start() {
    if (this.#started) return;
    this.#started = true;
    await onRcdState((next) => {
      this.state = next;
    });
    try {
      this.state = await api.getRcdState();
    } catch (err) {
      this.error = errorMessage(err);
    }
  }

  async restart() {
    this.error = null;
    try {
      await api.restartRcd();
    } catch (err) {
      this.error = errorMessage(err);
    }
  }
}

export const rcd = new RcdStore();
