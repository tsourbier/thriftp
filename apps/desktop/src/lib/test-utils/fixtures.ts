import type {
  BrowsingSession,
  ConnectionHistoryEntry,
  PathResolution,
  ServerFavoriteView,
  SshKeyCandidate,
  SshKeyValidation,
  WorkspaceLayout
} from "../ipc/types";

const now = "2026-11-05T00:00:00Z";

export const workspaceLayout: WorkspaceLayout = {
  splitRatio: 0.5,
  sidebarVisible: true,
  sidebarSections: {
    servers: true,
    directoryFavorites: true,
    automation: true
  },
  hubTab: "favorites"
};

export function serverFavorite(
  overrides: Partial<ServerFavoriteView> = {}
): ServerFavoriteView {
  return {
    id: "favorite-1",
    connectionProfileId: "profile-1",
    displayName: "Production",
    groupLabel: null,
    protocol: "sftp",
    host: "prod.example",
    port: 22,
    username: "deploy",
    authMethod: "password",
    timeoutSeconds: 30,
    sshKeyRef: null,
    pinned: false,
    lastUsedAt: now,
    credentialPolicy: "ask",
    hasSavedCredential: false,
    localPath: "/Users/me/Projects",
    remotePath: "/srv/www",
    createdAt: now,
    updatedAt: now,
    ...overrides
  };
}

export function historyEntry(
  overrides: Partial<ConnectionHistoryEntry> = {}
): ConnectionHistoryEntry {
  return {
    favoriteId: "favorite-1",
    connectionProfileId: "profile-1",
    displayName: "Production",
    protocol: "sftp",
    host: "prod.example",
    port: 22,
    username: "deploy",
    lastUsedAt: now,
    ...overrides
  };
}

export const sshKeyCandidate: SshKeyCandidate = {
  fileName: "id_ed25519",
  path: "/Users/me/.ssh/id_ed25519"
};

export const sshKeyValidation: SshKeyValidation = {
  valid: true
};

export function browsingSession(
  overrides: Partial<BrowsingSession> = {}
): BrowsingSession {
  return {
    id: "tab-1",
    connectionProfileId: "profile-1",
    currentPath: "/srv/www",
    localPath: "/Users/me/Projects",
    history: ["/srv/www"],
    historyIndex: 0,
    viewSettings: { sortKey: "name", showHidden: false, filter: "" },
    selectedItems: [],
    localSelectedItems: [],
    createdAt: now,
    restored: false,
    ...overrides
  };
}

export function pathResolution(overrides: Partial<PathResolution> = {}): PathResolution {
  return {
    originalPath: "/srv/www",
    path: "/srv/www",
    fallback: "none",
    ...overrides
  };
}
