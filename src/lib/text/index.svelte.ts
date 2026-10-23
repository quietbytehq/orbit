import { api } from "$lib/api";
import { buildStrings, type Platform, type Strings } from "./strings";

/**
 * The UI's wording, in the words of the platform Orbit runs on. `text.s` is reactive: it is
 * rebuilt when the platform becomes known after `load()`.
 */
class Text {
  platform = $state<Platform>("linux");
  version = $state("");
  s: Strings = $derived(buildStrings(this.platform));
  #loaded = false;

  async load() {
    if (this.#loaded) return;
    this.#loaded = true;
    try {
      const info = await api.appInfo();
      this.platform = info.platform;
      this.version = info.version;
    } catch {
      // Outside Tauri (plain browser) the defaults stand.
    }
  }

  get isWindows(): boolean {
    return this.platform === "windows";
  }
}

export const text = new Text();
