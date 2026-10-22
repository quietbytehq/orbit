import { api, errorMessage } from "$lib/api";
import type { Settings } from "$lib/api/types";
import { applyTheme } from "$lib/util/theme";

/** App settings. Changes are saved as soon as they are made. */
class SettingsStore {
  value = $state<Settings | null>(null);
  error = $state<string | null>(null);

  async load() {
    try {
      const settings = await api.getSettings();
      this.value = settings;
      applyTheme(settings.theme);
    } catch (err) {
      this.error = errorMessage(err);
    }
  }

  async save(next: Settings): Promise<boolean> {
    this.error = null;
    try {
      this.value = await api.saveSettings(next);
      applyTheme(this.value.theme);
      return true;
    } catch (err) {
      this.error = errorMessage(err);
      await this.load();
      return false;
    }
  }
}

export const settings = new SettingsStore();
