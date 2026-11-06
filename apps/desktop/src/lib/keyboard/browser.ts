import type { FileEntryView, SortKey } from "../ipc/types";

/** Modifier state reported by keyboard selection. */
export type KeyboardSelectionModifiers = {
  shift: boolean;
  toggle: boolean;
};

function selectionModifiers(event: KeyboardEvent): KeyboardSelectionModifiers {
  return {
    shift: event.shiftKey,
    toggle: event.metaKey || event.ctrlKey
  };
}

export type BrowserKeyActions = {
  entries: FileEntryView[];
  selectedPath: string | null;
  onselect: (entry: FileEntryView, modifiers?: KeyboardSelectionModifiers) => void;
  onopen: (entry: FileEntryView) => void;
  onup: () => void;
  onback?: () => void;
  onforward?: () => void;
  onrefresh: () => void;
  ontoggleHidden: () => void;
  onsort: (key: SortKey) => void;
  ondelete: (entry: FileEntryView) => void;
  onrename?: (entry: FileEntryView) => void;
  onnewfolder?: () => void;
  oninspect?: (entry: FileEntryView) => void;
};

export function handleBrowserKey(
  event: KeyboardEvent,
  actions: BrowserKeyActions
): boolean {
  const selectedIndex = actions.entries.findIndex(
    (entry) => entry.path === actions.selectedPath
  );

  switch (event.key) {
    case "ArrowDown":
      if (actions.entries.length > 0) {
        const next =
          actions.entries[Math.min(actions.entries.length - 1, selectedIndex + 1)];
        actions.onselect(next, selectionModifiers(event));
        return true;
      }
      return false;
    case "ArrowUp":
      if (actions.entries.length > 0) {
        const next = actions.entries[Math.max(0, selectedIndex - 1)];
        actions.onselect(next, selectionModifiers(event));
        return true;
      }
      return false;
    case "Enter":
      if (selectedIndex >= 0) {
        actions.onopen(actions.entries[selectedIndex]);
        return true;
      }
      return false;
    case "Backspace":
      actions.onup();
      return true;
    case "F5":
      actions.onrefresh();
      return true;
    case "Delete":
      if (selectedIndex >= 0) {
        actions.ondelete(actions.entries[selectedIndex]);
        return true;
      }
      return false;
    case "F2":
      if (selectedIndex >= 0 && actions.onrename) {
        actions.onrename(actions.entries[selectedIndex]);
        return true;
      }
      return false;
    default:
      if (
        (event.ctrlKey || event.metaKey) &&
        event.shiftKey &&
        event.key.toLowerCase() === "n" &&
        actions.onnewfolder
      ) {
        actions.onnewfolder();
        return true;
      }
      if (
        (event.ctrlKey || event.metaKey) &&
        event.key.toLowerCase() === "i" &&
        selectedIndex >= 0 &&
        actions.oninspect
      ) {
        actions.oninspect(actions.entries[selectedIndex]);
        return true;
      }
      if (event.altKey && event.key === "ArrowLeft") {
        if (actions.onback) {
          actions.onback();
        } else {
          actions.onup();
        }
        return true;
      }
      if (event.altKey && event.key === "ArrowRight") {
        actions.onforward?.();
        return true;
      }
      if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "h") {
        actions.ontoggleHidden();
        return true;
      }
      return false;
  }
}
