import { describe, expect, it } from "vitest";
import { applyRelativeNavigation, relativeNavigation } from "./linked-navigation";

describe("linked navigation", () => {
  it("moves up and down between sibling folders", () => {
    expect(relativeNavigation("/local/project/a", "/local/project/b")).toEqual({
      up: 1,
      down: ["b"]
    });
    expect(applyRelativeNavigation("/remote/work/a", { up: 1, down: ["b"] })).toBe(
      "/remote/work/b"
    );
  });

  it("mirrors nested ancestor and descendant moves", () => {
    const transition = relativeNavigation("/local/a/b/c", "/local/a/d/e");
    expect(transition).toEqual({ up: 2, down: ["d", "e"] });
    expect(applyRelativeNavigation("/remote/a/b/c", transition!)).toBe("/remote/a/d/e");
  });

  it("preserves relative paths instead of forcing a root", () => {
    const transition = relativeNavigation("project/a", "project/b");
    expect(applyRelativeNavigation("work/a", transition!)).toBe("work/b");
    expect(applyRelativeNavigation("work", { up: 1, down: [] })).toBe(".");
  });

  it("preserves UNC and drive roots", () => {
    const transition = relativeNavigation("C:\\Users\\me\\a", "C:\\Users\\me\\b");
    expect(applyRelativeNavigation("C:\\work\\a", transition!)).toBe("C:\\work\\b");
    expect(applyRelativeNavigation("C:\\work", { up: 1, down: [] })).toBe("C:\\");
    expect(applyRelativeNavigation("\\\\server\\share\\a", { up: 1, down: ["b"] })).toBe(
      "\\\\server\\share\\b"
    );
  });

  it("preserves Windows separators", () => {
    const transition = relativeNavigation("C:\\Users\\me\\a", "C:\\Users\\me\\b");
    expect(applyRelativeNavigation("C:\\work\\a", transition!)).toBe("C:\\work\\b");
  });

  it("returns null when no relative move can be applied", () => {
    expect(relativeNavigation(".", "/remote/a")).toBeNull();
    expect(applyRelativeNavigation(".", { up: 0, down: ["a"] })).toBeNull();
    expect(applyRelativeNavigation("/a", { up: 4, down: [] })).toBeNull();
  });
});
