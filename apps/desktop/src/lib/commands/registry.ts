import type {
  BrowsingSession,
  ConnectionHistoryEntry,
  DirectoryFavorite,
  ServerFavoriteView
} from "../ipc/types";

export type PaletteCategory = "servers" | "favorites" | "tabs" | "actions";

export type PaletteAction =
  | { type: "connect-server"; favoriteId: string }
  | { type: "navigate-favorite"; favorite: DirectoryFavorite }
  | { type: "activate-tab"; tabId: string }
  | { type: "new-connection" }
  | { type: "new-favorite" }
  | { type: "preferences" }
  | { type: "refresh-pane" }
  | { type: "toggle-hidden" };

export type CommandPaletteEntry = {
  id: string;
  label: string;
  category: PaletteCategory;
  subtitle?: string;
  keywords: string[];
  enabled: boolean;
  disabledReason?: string;
  action: PaletteAction;
};

export type RegistryInput = {
  servers: ServerFavoriteView[];
  directoryFavorites: DirectoryFavorite[];
  tabs: BrowsingSession[];
  recent: ConnectionHistoryEntry[];
  connectedProfileIds: string[];
  canRefresh: boolean;
};

const categoryOrder: PaletteCategory[] = ["servers", "favorites", "tabs", "actions"];

export function buildEntries(input: RegistryInput): CommandPaletteEntry[] {
  const connected = new Set(input.connectedProfileIds);
  const servers = input.servers.map((favorite) => ({
    id: `server:${favorite.id}`,
    label: favorite.displayName,
    category: "servers" as const,
    subtitle: `${favorite.protocol} · ${favorite.host}`,
    keywords: [
      favorite.host,
      favorite.username,
      favorite.groupLabel ?? "",
      favorite.protocol,
      favorite.remotePath ?? ""
    ],
    enabled: true,
    action: { type: "connect-server" as const, favoriteId: favorite.id }
  }));
  const favorites = input.directoryFavorites.map((favorite) => {
    const enabled = connected.has(favorite.connectionProfileId);
    return {
      id: `favorite:${favorite.id}`,
      label: favorite.label,
      category: "favorites" as const,
      subtitle: favorite.path,
      keywords: [favorite.path],
      enabled,
      ...(enabled ? {} : { disabledReason: "Connect to this server first." }),
      action: { type: "navigate-favorite" as const, favorite }
    };
  });
  const tabs = input.tabs.map((tab) => ({
    id: `tab:${tab.id}`,
    label: tab.currentPath,
    category: "tabs" as const,
    subtitle: tab.connectionProfileId ? "Remote tab" : "Local tab",
    keywords: [tab.currentPath, tab.connectionProfileId ?? ""],
    enabled: true,
    action: { type: "activate-tab" as const, tabId: tab.id }
  }));
  const actions: CommandPaletteEntry[] = [
    {
      id: "action:new-connection",
      label: "New connection",
      category: "actions",
      keywords: ["create", "server"],
      enabled: true,
      action: { type: "new-connection" }
    },
    {
      id: "action:new-favorite",
      label: "New favorite",
      category: "actions",
      keywords: ["save", "bookmark", "server"],
      enabled: true,
      action: { type: "new-favorite" }
    },
    {
      id: "action:preferences",
      label: "Preferences",
      category: "actions",
      keywords: ["settings", "theme"],
      enabled: true,
      action: { type: "preferences" }
    },
    {
      id: "action:refresh",
      label: "Refresh active pane",
      category: "actions",
      keywords: ["reload"],
      enabled: input.canRefresh,
      ...(input.canRefresh ? {} : { disabledReason: "No active connection." }),
      action: { type: "refresh-pane" }
    },
    {
      id: "action:toggle-hidden",
      label: "Toggle hidden files",
      category: "actions",
      keywords: ["dotfiles", "show", "hide"],
      enabled: true,
      action: { type: "toggle-hidden" }
    }
  ];
  return [...servers, ...favorites, ...tabs, ...actions];
}

export function searchEntries(
  entries: CommandPaletteEntry[],
  query: string
): CommandPaletteEntry[] {
  const needle = query.trim().toLocaleLowerCase();
  const matches = entries.filter((entry) => {
    if (!needle) {
      return true;
    }
    return [entry.label, entry.subtitle ?? "", ...entry.keywords].some((value) =>
      value.toLocaleLowerCase().includes(needle)
    );
  });
  return matches.sort((left, right) => {
    const category =
      categoryOrder.indexOf(left.category) - categoryOrder.indexOf(right.category);
    return category !== 0 ? category : left.label.localeCompare(right.label);
  });
}

export function recentOrSuggested(
  entries: CommandPaletteEntry[],
  recent: ConnectionHistoryEntry[]
): CommandPaletteEntry[] {
  const recentFavorites = new Set(
    recent.map((entry) => entry.favoriteId).filter((id): id is string => Boolean(id))
  );
  const preferred = entries.filter((entry) => {
    if (entry.category !== "servers" || entry.action.type !== "connect-server") {
      return false;
    }
    return recentFavorites.has(entry.action.favoriteId);
  });
  const fallbackServers =
    preferred.length > 0
      ? []
      : entries.filter((entry) => entry.category === "servers").slice(0, 5);
  const actions = entries.filter(
    (entry) => entry.category === "actions" && entry.enabled
  );
  return [...preferred, ...fallbackServers, ...actions];
}

export function groupEntries(
  entries: CommandPaletteEntry[]
): Array<{ category: PaletteCategory; entries: CommandPaletteEntry[] }> {
  return categoryOrder
    .map((category) => ({
      category,
      entries: entries.filter((entry) => entry.category === category)
    }))
    .filter((group) => group.entries.length > 0);
}
