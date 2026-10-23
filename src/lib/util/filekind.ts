/** Broad file categories used to pick an icon in the file browser. */
export type FileKind =
  | "folder"
  | "image"
  | "video"
  | "audio"
  | "archive"
  | "code"
  | "spreadsheet"
  | "presentation"
  | "document"
  | "pdf"
  | "file";

const BY_EXTENSION: Record<string, FileKind> = {};
const add = (kind: FileKind, extensions: string) => {
  for (const ext of extensions.split(" ")) BY_EXTENSION[ext] = kind;
};
add("image", "png jpg jpeg gif webp svg heic heif bmp tif tiff avif ico raw cr2 nef");
add("video", "mp4 mov mkv avi webm m4v wmv flv mpg mpeg 3gp");
add("audio", "mp3 wav flac m4a aac ogg oga opus wma aiff");
add("archive", "zip rar 7z tar gz tgz bz2 xz zst iso dmg");
add("code", "js ts jsx tsx mjs cjs py rs go java kt c cc cpp h hpp cs swift rb php sh ps1 bat html css scss json yaml yml toml xml sql");
add("spreadsheet", "xls xlsx xlsm csv tsv ods numbers gsheet");
add("presentation", "ppt pptx odp key gslides");
add("document", "txt md rtf doc docx odt pages gdoc log");
add("pdf", "pdf");

/** Picks a category from the name's extension. Folders always map to "folder". */
export function fileKind(name: string, isDir: boolean): FileKind {
  if (isDir) return "folder";
  const dot = name.lastIndexOf(".");
  if (dot <= 0 || dot === name.length - 1) return "file";
  return BY_EXTENSION[name.slice(dot + 1).toLowerCase()] ?? "file";
}
