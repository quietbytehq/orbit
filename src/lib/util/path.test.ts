import { describe, expect, it } from "vitest";
import { isInside, join, normalizePath, split } from "./path";

describe("path helpers", () => {
  it("normalizes separators", () => {
    expect(normalizePath("/a//b/./c/")).toBe("a/b/c");
    expect(normalizePath("/")).toBe("");
  });

  it("joins and splits", () => {
    expect(join("", "Photos")).toBe("Photos");
    expect(join("Photos", "2024")).toBe("Photos/2024");
    expect(split("Photos/2024/cat.jpg")).toEqual(["Photos/2024", "cat.jpg"]);
    expect(split("cat.jpg")).toEqual(["", "cat.jpg"]);
  });

  it("detects a folder inside itself", () => {
    expect(isInside("Photos/2024", "Photos")).toBe(true);
    expect(isInside("Photos", "Photos")).toBe(true);
    expect(isInside("Photos2", "Photos")).toBe(false);
  });
});
