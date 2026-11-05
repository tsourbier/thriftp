import { describe, expect, it } from "vitest";
import {
  applySplitKey,
  clampSplit,
  effectiveSidebarVisible,
  normalizeHubTab,
  normalizeLayout
} from "./layout";
import type { WorkspaceLayout } from "../ipc/types";

describe("workspace layout helpers", () => {
  it("clamps the split ratio", () => {
    expect(clampSplit(0.1)).toBe(0.2);
    expect(clampSplit(0.9)).toBe(0.8);
    expect(clampSplit(Number.NaN)).toBe(0.5);
  });

  it("handles keyboard split controls", () => {
    expect(applySplitKey(0.5, "ArrowLeft")).toBe(0.48);
    expect(applySplitKey(0.5, "ArrowRight")).toBe(0.52);
    expect(applySplitKey(0.5, "Home")).toBe(0.2);
    expect(applySplitKey(0.5, "End")).toBe(0.8);
    expect(applySplitKey(0.5, "Enter")).toBe(0.5);
  });

  it("collapses the sidebar before panes drop below the minimum", () => {
    expect(effectiveSidebarVisible({ sidebarVisible: true }, 900)).toBe(true);
    expect(effectiveSidebarVisible({ sidebarVisible: true }, 700)).toBe(false);
    expect(effectiveSidebarVisible({ sidebarVisible: false }, 1400)).toBe(false);
  });

  it("normalizes unknown hub tabs and missing sections", () => {
    const layout: WorkspaceLayout = {
      splitRatio: 0.95,
      sidebarVisible: true,
      sidebarSections: { servers: false, directoryFavorites: true, automation: true },
      hubTab: "favorites"
    };
    expect(normalizeHubTab("future")).toBe("favorites");
    const normalized = normalizeLayout(layout);
    expect(normalized.splitRatio).toBe(0.8);
    expect(normalized.sidebarSections.servers).toBe(false);
  });
});
