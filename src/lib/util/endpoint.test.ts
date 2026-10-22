import { describe, expect, it } from "vitest";
import { endpointLabel } from "./endpoint";

describe("endpointLabel", () => {
  it("labels remote roots and folders", () => {
    expect(endpointLabel({ kind: "remote", remote: "work", path: "" })).toBe("work:");
    expect(endpointLabel({ kind: "remote", remote: "work", path: "Photos/2024" })).toBe("work:/Photos/2024");
  });

  it("labels local folders by path", () => {
    expect(endpointLabel({ kind: "local", path: "/home/ana/Pictures" })).toBe("/home/ana/Pictures");
  });
});
