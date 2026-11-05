import { describe, expect, it } from "vitest";
import { buildEntries, groupEntries, searchEntries } from "./registry";
import type {
  BrowsingSession,
  ConnectionHistoryEntry,
  DirectoryFavorite,
  ServerFavoriteView
} from "../ipc/types";

function server(overrides: Partial<ServerFavoriteView> = {}): ServerFavoriteView {
  return {
    id: "s1",
    connectionProfileId: "p1",
    displayName: "Production",
    groupLabel: "Ops",
    protocol: "sftp",
    host: "prod.example",
    port: 22,
    username: "deploy",
    authMethod: "password",
    timeoutSeconds: 30,
    sshKeyRef: null,
    pinned: false,
    lastUsedAt: null,
    credentialPolicy: "ask",
    hasSavedCredential: false,
    localPath: null,
    remotePath: "/srv/www",
    createdAt: "2026-11-05T00:00:00Z",
    updatedAt: "2026-11-05T00:00:00Z",
    ...overrides
  };
}

function directory(overrides: Partial<DirectoryFavorite> = {}): DirectoryFavorite {
  return {
    id: "d1",
    connectionProfileId: "p1",
    path: "/srv/www",
    label: "Web root",
    sortOrder: 0,
    createdAt: "2026-11-05T00:00:00Z",
    updatedAt: "2026-11-05T00:00:00Z",
    ...overrides
  };
}

function tab(overrides: Partial<BrowsingSession> = {}): BrowsingSession {
  return {
    id: "t1",
    connectionProfileId: "p1",
    currentPath: "/srv/www",
    history: ["/srv/www"],
    historyIndex: 0,
    viewSettings: { sortKey: "name", showHidden: false },
    selectedItems: [],
    createdAt: "2026-11-05T00:00:00Z",
    restored: false,
    ...overrides
  };
}

describe("command palette registry", () => {
  it("groups servers, favorites, tabs, and safe actions", () => {
    const entries = buildEntries({
      servers: [server()],
      directoryFavorites: [directory()],
      tabs: [tab()],
      recent: [],
      connectedProfileIds: ["p1"],
      canRefresh: true
    });
    expect(groupEntries(entries).map((group) => group.category)).toEqual([
      "servers",
      "favorites",
      "tabs",
      "actions"
    ]);
    expect(entries.some((entry) => entry.action.type === "connect-server")).toBe(true);
    expect(entries.some((entry) => entry.action.type === "activate-tab")).toBe(true);
  });

  it("searches case-insensitively across labels and keywords", () => {
    const entries = buildEntries({
      servers: [server()],
      directoryFavorites: [directory()],
      tabs: [tab()],
      recent: [],
      connectedProfileIds: [],
      canRefresh: false
    });
    expect(searchEntries(entries, "prod.example")[0].label).toBe("Production");
    expect(searchEntries(entries, "web root")[0].label).toBe("Web root");
  });

  it("marks unavailable actions disabled and excludes destructive controls", () => {
    const entries = buildEntries({
      servers: [server()],
      directoryFavorites: [directory()],
      tabs: [tab()],
      recent: [],
      connectedProfileIds: [],
      canRefresh: false
    });
    const favorite = entries.find((entry) => entry.action.type === "navigate-favorite");
    expect(favorite?.enabled).toBe(false);
    expect(favorite?.disabledReason).toContain("Connect");
    const labels = entries.map((entry) => entry.label.toLowerCase()).join(" ");
    expect(labels).not.toContain("delete");
    expect(labels).not.toContain("cancel");
  });

  it("suggests recent servers when the query is empty", () => {
    const recent: ConnectionHistoryEntry[] = [
      {
        favoriteId: "s1",
        connectionProfileId: "p1",
        displayName: "Production",
        protocol: "sftp",
        host: "prod.example",
        port: 22,
        username: "deploy",
        lastUsedAt: "2026-11-05T00:00:00Z"
      }
    ];
    const entries = buildEntries({
      servers: [server()],
      directoryFavorites: [],
      tabs: [],
      recent,
      connectedProfileIds: ["p1"],
      canRefresh: true
    });
    expect(entries.find((entry) => entry.category === "servers")?.label).toBe(
      "Production"
    );
  });

  it("searches 200 servers plus favorites within 100 ms", () => {
    const servers = Array.from({ length: 200 }, (_, index) =>
      server({
        id: `s${index}`,
        displayName: `Server ${index}`,
        host: `host-${index}.test`
      })
    );
    const favorites = Array.from({ length: 200 }, (_, index) =>
      directory({ id: `d${index}`, label: `Favorite ${index}`, path: `/srv/${index}` })
    );
    const entries = buildEntries({
      servers,
      directoryFavorites: favorites,
      tabs: [],
      recent: [],
      connectedProfileIds: ["p1"],
      canRefresh: true
    });
    const started = performance.now();
    expect(searchEntries(entries, "server 19").length).toBeGreaterThan(0);
    expect(searchEntries(entries, "favorite 19").length).toBeGreaterThan(0);
    expect(performance.now() - started).toBeLessThan(100);
  });
});
