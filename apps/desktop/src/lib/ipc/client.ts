import { invoke } from "@tauri-apps/api/core";
import type {
  BrowsingSession,
  BrowserSource,
  ConnectionProfile,
  ConnectionTestResult,
  ConnectionView,
  DesktopError,
  DirectoryBatchPayload,
  DirectoryErrorPayload,
  DirectoryFavorite,
  FileEntryView,
  FileMetadata,
  ListingHandle,
  Preferences,
  PreviewOpenResult,
  PreviewReadResult,
  Priority,
  QueueView,
  ServerFavoriteInput,
  ServerFavoriteView,
  FavoritePreferences,
  ActiveProfileResult,
  CredentialImpact,
  CredentialRemoveResult,
  FavoriteDeleteResult,
  RenameGroupResult,
  ReusableCredential,
  TransferView
} from "./types";

export class IpcError extends Error {
  constructor(public readonly details: DesktopError) {
    super(details.message);
  }
}

async function call<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  try {
    return await invoke<T>(command, args);
  } catch (error) {
    if (
      typeof error === "object" &&
      error !== null &&
      "code" in error &&
      typeof (error as DesktopError).code === "string"
    ) {
      throw new IpcError(error as DesktopError);
    }
    throw error;
  }
}

export const ipc = {
  listProfiles: () => call<ConnectionProfile[]>("connections_list"),
  createProfile: (profile: ConnectionProfile, secret?: string) =>
    call<ConnectionProfile>("connections_create", { profile, secret }),
  updateProfile: (profile: ConnectionProfile, secret?: string) =>
    call<ConnectionProfile>("connections_update", { profile, secret }),
  getProfile: (connectionId: string) =>
    call<ActiveProfileResult>("connections_get_profile", { connectionId }),
  deleteProfile: (id: string) => call<{ deleted: boolean }>("connections_delete", { id }),
  connect: (profileId: string, secret?: string) =>
    call<ConnectionView>("connections_connect", { profileId, secret }),
  reconnect: (connectionId: string, secret?: string) =>
    call<ConnectionView>("connections_reconnect", { connectionId, secret }),
  disconnect: (connectionId: string) =>
    call<{ disconnected: boolean }>("connections_disconnect", { connectionId }),
  test: (profileId: string, secret?: string) =>
    call<ConnectionTestResult>("connections_test", { profileId, secret }),

  openLocal: (path: string) => call<ListingHandle>("browser_open_local", { path }),
  openRemote: (connectionId: string, path: string) =>
    call<ListingHandle>("browser_open_remote", { connectionId, path }),
  refreshListing: (listingId: string) =>
    call<ListingHandle>("browser_refresh", { listingId }),
  cancelListing: (listingId: string) =>
    call<{ cancelled: boolean }>("browser_cancel_listing", { listingId }),
  stat: (source: BrowserSource, path: string, connectionId?: string) =>
    call<FileEntryView>("browser_stat", { source, path, connectionId }),
  mkdir: (source: BrowserSource, path: string, connectionId?: string) =>
    call<FileEntryView>("browser_mkdir", { source, path, connectionId }),
  rename: (source: BrowserSource, from: string, to: string, connectionId?: string) =>
    call<FileEntryView>("browser_rename", { source, from, to, connectionId }),
  deletePath: (
    source: BrowserSource,
    path: string,
    confirmed: boolean,
    connectionId?: string
  ) =>
    call<{ deleted: boolean }>("browser_delete", {
      source,
      path,
      confirmed,
      connectionId
    }),
  chmod: (connectionId: string, path: string, mode: number) =>
    call<FileEntryView>("browser_chmod", { connectionId, path, mode }),

  previewOpen: (
    source: BrowserSource,
    path: string,
    connectionId?: string,
    allowDownload?: boolean
  ) =>
    call<PreviewOpenResult>("preview_open", {
      source,
      path,
      connectionId,
      allowDownload
    }),
  previewRead: (source: BrowserSource, path: string, previewId?: string) =>
    call<PreviewReadResult>("preview_read", { source, path, previewId }),
  previewClose: (previewId: string) =>
    call<{ closed: boolean }>("preview_close", { previewId }),

  transfersStart: (input: {
    direction: "upload" | "download";
    profileId: string;
    sources: string[];
    destination: string;
    conflictPolicy?: "ask" | "overwrite" | "skip" | "rename";
  }) =>
    call<{ transferIds: string[]; skippedLinks?: string[] }>("transfers_start", input),
  transfersList: () => call<TransferView[]>("transfers_list"),
  transfersGet: (transferId: string) =>
    call<TransferView>("transfers_get", { transferId }),
  transfersPause: (transferId: string) =>
    call<TransferView>("transfers_pause", { transferId }),
  transfersResume: (transferId: string) =>
    call<TransferView>("transfers_resume", { transferId }),
  transfersCancel: (transferId: string) =>
    call<TransferView>("transfers_cancel", { transferId }),
  transfersRestart: (transferId: string) =>
    call<TransferView>("transfers_restart", { transferId }),
  transfersDiscardPartial: (transferId: string) =>
    call<TransferView>("transfers_discard_partial", { transferId }),
  transfersClearHistory: () => call<number>("transfers_clear_history"),
  transfersDismiss: (transferId: string) =>
    call<QueueView>("transfers_dismiss", { transferId }),

  queueGet: () => call<QueueView>("queue_get"),
  queuePause: () => call<QueueView>("queue_pause"),
  queueResume: () => call<QueueView>("queue_resume"),
  queueSetConcurrency: (limit: number) =>
    call<QueueView>("queue_set_concurrency", { limit }),
  queueSetBandwidth: (bytesPerSecond: number | null) =>
    call<QueueView>("queue_set_bandwidth", { bytesPerSecond }),
  queueReorder: (transferIds: string[]) =>
    call<QueueView>("queue_reorder", { transferIds }),
  queueSetPriority: (transferId: string, priority: Priority) =>
    call<QueueView>("queue_set_priority", { transferId, priority }),
  queueCancelAll: (confirmed: boolean) =>
    call<QueueView>("queue_cancel_all", { confirmed }),

  favoritesList: (profileId?: string) =>
    call<DirectoryFavorite[]>("directory_favorites_list", { profileId }),
  favoritesCreate: (profileId: string, path: string, label: string) =>
    call<DirectoryFavorite>("directory_favorites_create", { profileId, path, label }),
  favoritesUpdate: (id: string, label?: string, path?: string) =>
    call<DirectoryFavorite>("directory_favorites_update", { id, label, path }),
  favoritesDelete: (id: string) =>
    call<{ deleted: boolean }>("directory_favorites_delete", { id, confirmed: true }),
  favoritesReorder: (profileId: string, ids: string[]) =>
    call<DirectoryFavorite[]>("directory_favorites_reorder", { profileId, ids }),

  tabsList: () => call<BrowsingSession[]>("tabs_list"),
  tabsSave: (tabs: BrowsingSession[]) => call<{ saved: boolean }>("tabs_save", { tabs }),
  tabsRestore: () => call<BrowsingSession[]>("tabs_restore"),

  preferencesGet: () => call<Preferences>("preferences_get"),
  preferencesSet: (preference: Preferences) =>
    call<Preferences>("preferences_set", { preference }),

  inspectorGet: (source: BrowserSource, path: string, connectionId?: string) =>
    call<FileMetadata>("inspector_get", { source, path, connectionId }),

  serverFavoritesList: () => call<ServerFavoriteView[]>("favorites_list"),
  serverFavoritesGet: (id: string) => call<ServerFavoriteView>("favorites_get", { id }),
  serverFavoritesCreate: (
    profile: ServerFavoriteInput,
    credentialPolicy: "save" | "ask",
    credentialRefId?: string,
    secret?: string
  ) =>
    call<ServerFavoriteView>("favorites_create", {
      profile,
      credentialPolicy,
      pinned: profile.pinned,
      credentialRefId,
      secret,
      secretKind: profile.authMethod === "sshPrivateKey" ? "passphrase" : "password"
    }),
  serverFavoritesUpdate: (
    id: string,
    profile: ServerFavoriteInput,
    credentialPolicy: "save" | "ask",
    credentialRefId?: string,
    secret?: string
  ) =>
    call<ServerFavoriteView>("favorites_update", {
      id,
      profile,
      credentialPolicy,
      pinned: profile.pinned,
      credentialRefId,
      secret,
      secretKind: profile.authMethod === "sshPrivateKey" ? "passphrase" : "password"
    }),
  serverFavoritesDelete: (id: string) =>
    call<FavoriteDeleteResult>("favorites_delete", { id }),
  serverFavoritesSetPinned: (id: string, pinned: boolean) =>
    call<ServerFavoriteView>("favorites_set_pinned", { id, pinned }),
  serverFavoritesSetGroup: (id: string, groupLabel: string | null) =>
    call<ServerFavoriteView>("favorites_set_group", { id, groupLabel }),
  serverFavoritesRenameGroup: (oldLabel: string, newLabel: string) =>
    call<RenameGroupResult>("favorites_rename_group", { oldLabel, newLabel }),
  serverFavoritesPreferences: () =>
    call<FavoritePreferences>("favorites_get_preferences"),
  serverFavoritesSetPreferences: (preferences: FavoritePreferences) =>
    call<FavoritePreferences>("favorites_set_preferences", { preferences }),
  serverFavoritesTestConnection: (profile: ServerFavoriteInput, secret?: string) =>
    call<{ ok: boolean; error?: unknown }>("favorites_test_connection", {
      profile,
      secret
    }),
  serverFavoritesConnect: (id: string, secret?: string, secretKind?: string) =>
    call<ConnectionView>("favorites_connect", { id, secret, secretKind }),
  credentialsImpact: (favoriteId: string) =>
    call<CredentialImpact>("credentials_get_impact", { favoriteId }),
  credentialsReusable: () => call<ReusableCredential[]>("credentials_list_reusable"),
  credentialsRemove: (favoriteId: string, confirmed: boolean) =>
    call<CredentialRemoveResult>("credentials_remove", { favoriteId, confirmed }),
  credentialsDelete: (credentialRefId: string, confirmed: boolean) =>
    call<{ deleted: boolean }>("credentials_delete", { credentialRefId, confirmed })
};

export type {
  BrowsingSession,
  BrowserSource,
  ConnectionProfile,
  ConnectionTestResult,
  ConnectionView,
  DesktopError,
  DirectoryBatchPayload,
  DirectoryErrorPayload,
  DirectoryFavorite,
  FileEntryView,
  FileMetadata,
  ListingHandle,
  Preferences,
  PreviewOpenResult,
  PreviewReadResult,
  Priority,
  QueueView,
  TransferView,
  ServerFavoriteInput,
  ServerFavoriteView,
  FavoritePreferences,
  CredentialImpact,
  ReusableCredential,
  ActiveProfileResult,
  CredentialRemoveResult,
  FavoriteDeleteResult,
  RenameGroupResult
};
