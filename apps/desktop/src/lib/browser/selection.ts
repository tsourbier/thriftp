import type { FileEntryView } from "../ipc/types";

export type SelectionModifiers = {
  shift: boolean;
  toggle: boolean;
};

export type SelectionState = {
  selectedPaths: string[];
  anchorPath: string | null;
  activePath: string | null;
};

export function applySelection(
  entries: FileEntryView[],
  current: SelectionState,
  target: FileEntryView,
  modifiers: SelectionModifiers
): SelectionState {
  if (modifiers.shift && current.anchorPath) {
    const anchorIndex = entries.findIndex((entry) => entry.path === current.anchorPath);
    const targetIndex = entries.findIndex((entry) => entry.path === target.path);
    if (anchorIndex >= 0 && targetIndex >= 0) {
      const start = Math.min(anchorIndex, targetIndex);
      const end = Math.max(anchorIndex, targetIndex);
      return {
        selectedPaths: entries.slice(start, end + 1).map((entry) => entry.path),
        anchorPath: current.anchorPath,
        activePath: target.path
      };
    }
  }

  if (modifiers.toggle) {
    const selected = current.selectedPaths.includes(target.path)
      ? current.selectedPaths.filter((path) => path !== target.path)
      : [...current.selectedPaths, target.path];
    return {
      selectedPaths: selected,
      anchorPath: target.path,
      activePath: target.path
    };
  }

  return {
    selectedPaths: [target.path],
    anchorPath: target.path,
    activePath: target.path
  };
}

export function pruneSelection(
  entries: FileEntryView[],
  current: SelectionState
): SelectionState {
  const paths = new Set(entries.map((entry) => entry.path));
  const selectedPaths = current.selectedPaths.filter((path) => paths.has(path));
  const anchorPath =
    current.anchorPath && paths.has(current.anchorPath) ? current.anchorPath : null;
  const activePath =
    current.activePath && paths.has(current.activePath)
      ? current.activePath
      : (selectedPaths[0] ?? null);
  return { selectedPaths, anchorPath, activePath };
}
