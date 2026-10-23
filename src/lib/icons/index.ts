/**
 * Icon set for the UI: lucide icons at one size and stroke weight, so every page looks the same.
 * Import icons through this module rather than from lucide directly.
 */
import type { Component } from "svelte";
import type { FileKind } from "$lib/util/filekind";

import Archive from "@lucide/svelte/icons/archive";
import ArrowUp from "@lucide/svelte/icons/arrow-up";
import Box from "@lucide/svelte/icons/box";
import ChevronDown from "@lucide/svelte/icons/chevron-down";
import ChevronRight from "@lucide/svelte/icons/chevron-right";
import Clapperboard from "@lucide/svelte/icons/clapperboard";
import Cloud from "@lucide/svelte/icons/cloud";
import CloudDownload from "@lucide/svelte/icons/cloud-download";
import CloudUpload from "@lucide/svelte/icons/cloud-upload";
import Copy from "@lucide/svelte/icons/copy";
import Database from "@lucide/svelte/icons/database";
import Download from "@lucide/svelte/icons/download";
import Eject from "@lucide/svelte/icons/eject";
import File from "@lucide/svelte/icons/file";
import FileArchive from "@lucide/svelte/icons/file-archive";
import FileCode from "@lucide/svelte/icons/file-code";
import FileImage from "@lucide/svelte/icons/file-image";
import FileMusic from "@lucide/svelte/icons/file-music";
import FileSpreadsheet from "@lucide/svelte/icons/file-spreadsheet";
import FileText from "@lucide/svelte/icons/file-text";
import FileVideoCamera from "@lucide/svelte/icons/file-video-camera";
import Folder from "@lucide/svelte/icons/folder";
import FolderInput from "@lucide/svelte/icons/folder-input";
import FolderPlus from "@lucide/svelte/icons/folder-plus";
import FolderSync from "@lucide/svelte/icons/folder-sync";
import HardDrive from "@lucide/svelte/icons/hard-drive";
import House from "@lucide/svelte/icons/house";
import Info from "@lucide/svelte/icons/info";
import KeyRound from "@lucide/svelte/icons/key-round";
import Network from "@lucide/svelte/icons/network";
import Pencil from "@lucide/svelte/icons/pencil";
import Play from "@lucide/svelte/icons/play";
import Plus from "@lucide/svelte/icons/plus";
import Presentation from "@lucide/svelte/icons/presentation";
import RefreshCw from "@lucide/svelte/icons/refresh-cw";
import Server from "@lucide/svelte/icons/server";
import Settings from "@lucide/svelte/icons/settings";
import Square from "@lucide/svelte/icons/square";
import Terminal from "@lucide/svelte/icons/terminal";
import Trash from "@lucide/svelte/icons/trash";
import TriangleAlert from "@lucide/svelte/icons/triangle-alert";
import Upload from "@lucide/svelte/icons/upload";

export {
  Archive,
  ArrowUp,
  Box,
  ChevronDown,
  ChevronRight,
  Clapperboard,
  Cloud,
  CloudDownload,
  CloudUpload,
  Copy,
  Database,
  Download,
  Eject,
  File,
  Folder,
  FolderInput,
  FolderPlus,
  FolderSync,
  HardDrive,
  House,
  Info,
  KeyRound,
  Network,
  Pencil,
  Play,
  Plus,
  RefreshCw,
  Server,
  Settings,
  Square,
  Terminal,
  Trash,
  TriangleAlert,
  Upload,
};

/** One size and stroke for the whole UI; pass as `{...ICON}` to any icon. */
export const ICON = { size: 16, strokeWidth: 1.75, "aria-hidden": true } as const;

/** Larger variant for cards. */
export const ICON_LG = { size: 22, strokeWidth: 1.6, "aria-hidden": true } as const;

/** Icons for the sidebar, by route. */
export const NAV_ICONS: Record<string, Component> = {
  "/": House,
  "/remotes": Cloud,
  "/sync": FolderSync,
  "/mounts": HardDrive,
  "/settings": Settings,
};

/**
 * Icon per provider id (`providers.rs`). Lucide ships no brand marks, so each service gets a
 * generic symbol for what it is.
 */
export const PROVIDER_ICONS: Record<string, Component> = {
  drive: Cloud,
  onedrive: CloudUpload,
  dropbox: Box,
  s3: Database,
  sftp: Terminal,
  smb: Network,
};

/** Fallback for providers not listed above (for example a remote made outside Orbit). */
export const PROVIDER_ICON_DEFAULT: Component = Server;

export function providerIcon(providerId: string | null): Component {
  return (providerId && PROVIDER_ICONS[providerId]) || PROVIDER_ICON_DEFAULT;
}

/** Icon per file category from `fileKind`. */
export const FILE_ICONS: Record<FileKind, Component> = {
  folder: Folder,
  image: FileImage,
  video: FileVideoCamera,
  audio: FileMusic,
  archive: FileArchive,
  code: FileCode,
  spreadsheet: FileSpreadsheet,
  presentation: Presentation,
  document: FileText,
  pdf: FileText,
  file: File,
};
