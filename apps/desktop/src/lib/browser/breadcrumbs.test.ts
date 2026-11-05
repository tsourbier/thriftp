import { describe, expect, it } from "vitest";
import { navigationTarget, overflowSegments, segmentsForPath } from "./breadcrumbs";

describe("breadcrumbs", () => {
  it("segments POSIX paths and marks the current folder", () => {
    const segments = segmentsForPath("/srv/www/app/");
    expect(segments.map((segment) => segment.label)).toEqual(["/", "srv", "www", "app"]);
    expect(segments.at(-1)?.isCurrent).toBe(true);
    expect(segments.at(-1)?.path).toBe("/srv/www/app");
  });

  it("segments Windows-style local paths", () => {
    const segments = segmentsForPath("C:\\Users\\me\\Projects");
    expect(segments[0].label).toBe("C:\\");
    expect(segments.at(-1)?.path).toBe("C:\\Users\\me\\Projects");
  });

  it("navigates to a selected ancestor", () => {
    const segments = segmentsForPath("/srv/www/app");
    expect(navigationTarget(segments, 2)).toBe("/srv/www");
  });

  it("collapses middle segments while keeping root and current", () => {
    const segments = segmentsForPath("/a/b/c/d/e/f/g");
    const { visible, overflow } = overflowSegments(segments, 5);
    expect(visible[0].label).toBe("/");
    expect(visible.at(-1)?.label).toBe("g");
    expect(overflow.length).toBeGreaterThan(0);
    expect(overflow.map((segment) => segment.label)).not.toContain("a");
  });
});
