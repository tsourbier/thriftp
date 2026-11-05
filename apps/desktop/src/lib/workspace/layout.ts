import type { HubTab, SidebarSectionKey, WorkspaceLayout } from "../ipc/types";

export const MIN_SPLIT_RATIO = 0.2;
export const MAX_SPLIT_RATIO = 0.8;
export const MIN_PANE_WIDTH = 320;
export const SIDEBAR_WIDTH = 240;
export const SIDEBAR_SECTION_KEYS: SidebarSectionKey[] = [
  "servers",
  "directoryFavorites",
  "automation"
];

export function clampSplit(value: number): number {
  if (!Number.isFinite(value)) {
    return 0.5;
  }
  return Math.min(MAX_SPLIT_RATIO, Math.max(MIN_SPLIT_RATIO, value));
}

export function effectiveSidebarVisible(
  layout: Pick<WorkspaceLayout, "sidebarVisible">,
  viewportWidth: number
): boolean {
  if (!layout.sidebarVisible) {
    return false;
  }
  return viewportWidth >= MIN_PANE_WIDTH * 2 + SIDEBAR_WIDTH;
}

export function applySplitKey(current: number, key: string, step = 0.02): number {
  switch (key) {
    case "ArrowLeft":
      return clampSplit(current - step);
    case "ArrowRight":
      return clampSplit(current + step);
    case "Home":
      return MIN_SPLIT_RATIO;
    case "End":
      return MAX_SPLIT_RATIO;
    case "Enter":
      return 0.5;
    default:
      return current;
  }
}

export function normalizeHubTab(value: string | null | undefined): HubTab {
  return value === "history" ? "history" : "favorites";
}

export function normalizeLayout(layout: WorkspaceLayout): WorkspaceLayout {
  const sections = {} as Record<SidebarSectionKey, boolean>;
  for (const key of SIDEBAR_SECTION_KEYS) {
    sections[key] = layout.sidebarSections?.[key] ?? true;
  }
  return {
    splitRatio: clampSplit(layout.splitRatio),
    sidebarVisible: Boolean(layout.sidebarVisible),
    sidebarSections: sections,
    hubTab: normalizeHubTab(layout.hubTab)
  };
}
