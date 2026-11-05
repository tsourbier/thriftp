import { writable } from "svelte/store";
import { ipc } from "../ipc/client";
import type { ConnectionHistoryEntry } from "../ipc/types";

export const connectionHistory = writable<ConnectionHistoryEntry[]>([]);

export async function refreshHistory(): Promise<ConnectionHistoryEntry[]> {
  const entries = await ipc.connectionHistoryList();
  connectionHistory.set(entries);
  return entries;
}

export async function clearHistory(confirmed: boolean): Promise<number> {
  const result = await ipc.connectionHistoryClear(confirmed);
  if (result.cleared) {
    connectionHistory.set([]);
  }
  return result.removed;
}
