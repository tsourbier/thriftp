import { ipc } from "../ipc/client";
import type { BrowsingSession } from "../ipc/types";

export type RestoredTab = BrowsingSession & {
  disconnected: boolean;
};

export async function restoreTabs(
  enabled: boolean,
  connectedProfileIds: string[] = []
): Promise<RestoredTab[]> {
  if (!enabled) {
    return [];
  }
  const tabs = await ipc.tabsRestore();
  return tabs.map((tab) => ({
    ...tab,
    restored: true,
    disconnected:
      tab.connectionProfileId === null ||
      !connectedProfileIds.includes(tab.connectionProfileId)
  }));
}

export function newTab(
  connectionProfileId: string | null,
  currentPath = "."
): BrowsingSession {
  return {
    id: crypto.randomUUID(),
    connectionProfileId,
    currentPath,
    localPath: ".",
    history: [currentPath],
    historyIndex: 0,
    viewSettings: { sortKey: "name", showHidden: false, filter: null },
    selectedItems: [],
    localSelectedItems: [],
    createdAt: new Date().toISOString(),
    restored: false
  };
}
