import { describe, expect, it } from "vitest";
import { fileKind } from "./filekind";

describe("fileKind", () => {
  it("maps folders regardless of name", () => {
    expect(fileKind("Photos.zip", true)).toBe("folder");
  });

  it("maps common extensions, ignoring case", () => {
    expect(fileKind("cat.JPG", false)).toBe("image");
    expect(fileKind("holiday.mp4", false)).toBe("video");
    expect(fileKind("song.flac", false)).toBe("audio");
    expect(fileKind("backup.tar.gz", false)).toBe("archive");
    expect(fileKind("main.rs", false)).toBe("code");
    expect(fileKind("budget.xlsx", false)).toBe("spreadsheet");
    expect(fileKind("talk.pptx", false)).toBe("presentation");
    expect(fileKind("notes.md", false)).toBe("document");
    expect(fileKind("scan.pdf", false)).toBe("pdf");
  });

  it("falls back to a plain file", () => {
    expect(fileKind("README", false)).toBe("file");
    expect(fileKind(".bashrc", false)).toBe("file");
    expect(fileKind("archive.", false)).toBe("file");
    expect(fileKind("thing.unknownext", false)).toBe("file");
  });
});
