import { describe, expect, it } from "vitest";
import { buildStrings } from "./strings";

function flatten(value: unknown, out: string[] = []): string[] {
  if (typeof value === "string") out.push(value);
  else if (typeof value === "function") {
    // Call text functions with harmless sample values so their templates are checked too.
    try {
      out.push(String((value as (...args: unknown[]) => unknown)("Sample", "Other", 3)));
    } catch {
      // ignore
    }
  } else if (value && typeof value === "object") Object.values(value).forEach((v) => flatten(v, out));
  return out;
}

describe("UI strings", () => {
  it("use platform words", () => {
    const win = buildStrings("windows");
    const linux = buildStrings("linux");
    expect(win.nav.drives).toBe("Drives");
    expect(linux.nav.drives).toBe("Folders");
    expect(win.drives.show).toContain("drive");
    expect(linux.drives.show).toContain("folder");
    expect(win.drives.intro).toContain("File Explorer");
    expect(win.drives.intro).toContain("This PC");
    expect(linux.drives.intro).not.toContain("File Explorer");
  });

  it("never show internals or jargon", () => {
    for (const platform of ["windows", "linux"] as const) {
      for (const text of flatten(buildStrings(platform))) {
        const lower = text.toLowerCase();
        expect(lower, text).not.toMatch(/\brcd\b/);
        expect(lower, text).not.toMatch(/\brc\b/);
        expect(lower, text).not.toMatch(/\bremotes?\b/);
        expect(lower, text).not.toMatch(/\bun?mount(ed|ing|s)?\b/);
      }
    }
  });

  it("name rclone only where the person is about to deal with it", () => {
    const strings = buildStrings("windows");
    const mentions = flatten(strings).filter((text) => text.toLowerCase().includes("rclone"));
    const allowed = flatten({ overview: strings.overview, settings: strings.settings, app: strings.app });
    for (const text of mentions) expect(allowed, text).toContain(text);
  });
});
