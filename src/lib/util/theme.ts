import type { Theme } from "$lib/api/types";

/**
 * Sets the colour scheme for this window. "system" removes the override, so the
 * prefers-color-scheme rules in app.css decide.
 */
export function applyTheme(theme: Theme): void {
  const root = document.documentElement;
  if (theme === "system") {
    delete root.dataset.theme;
  } else {
    root.dataset.theme = theme;
  }
}
