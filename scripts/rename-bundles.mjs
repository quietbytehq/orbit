#!/usr/bin/env node
// Renames the bundles Tauri just built so their file names start with "orbit" instead of the
// display name "Orbit for rclone" (Tauri derives file names from productName and offers no
// separate setting). The display name stays everywhere else: window titles, the Start menu, the
// .desktop entry and the installer. Run after `tauri build`; safe to run twice.
import { readdirSync, renameSync, statSync } from "node:fs";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";

const DISPLAY_NAME = "Orbit for rclone";
const SHORT_NAME = "orbit";
const here = dirname(fileURLToPath(import.meta.url));
const roots = [join(here, "..", "src-tauri", "target", "release", "bundle")];

/** Renames every file under `dir` whose name starts with the display name. */
export function renameBundles(dir) {
  const renamed = [];
  let entries;
  try {
    entries = readdirSync(dir);
  } catch {
    return renamed;
  }
  for (const entry of entries) {
    const path = join(dir, entry);
    if (statSync(path).isDirectory()) {
      renamed.push(...renameBundles(path));
    } else if (entry.startsWith(DISPLAY_NAME)) {
      const target = join(dir, SHORT_NAME + entry.slice(DISPLAY_NAME.length));
      renameSync(path, target);
      renamed.push([path, target]);
    }
  }
  return renamed;
}

if (process.argv[1] && fileURLToPath(import.meta.url) === process.argv[1]) {
  const all = roots.flatMap((root) => renameBundles(root));
  for (const [from, to] of all) console.log(`${from} -> ${to}`);
  if (all.length === 0) console.log("No bundles to rename.");
}
