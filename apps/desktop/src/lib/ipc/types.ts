export type Protocol = "ftp" | "ftpsExplicit" | "ftpsImplicit" | "sftp";
export type AuthMethod = "password" | "sshPrivateKey";
export type SortKey = "name" | "size" | "date" | "type";
export type BrowserSource = "local" | "remote";
export type Theme = "system" | "light" | "dark";
export type PreviewPreference = "auto" | "native" | "internal";

export type Preferences = {
  theme: Theme;
  showHiddenFiles: boolean;
  defaultConcurrency: number;
  bandwidthLimitBytesPerSecond: number | null;
  restoreTabs: boolean;
  previewMode: PreviewPreference;
  sortKey: SortKey;
  reducedMotion: boolean;
};

export type DesktopError = {
  code: string;
  message: string;
  field?: string;
  retryable?: boolean;
  fingerprint?: string;
};

export type ConnectionProfileInput = {
  displayName: string;
  groupLabel?: string | null;
  protocol: Protocol;
  host: string;
  port: number;
  username: string;
  authMethod: AuthMethod;
  timeoutSeconds: number;
  sshKeyRef?: string | null;
};

export type ConnectionProfile = ConnectionProfileInput & {
  id: string;
  createdAt: string;
  updatedAt: string;
  lastUsedAt?: string | null;
};

export type TransportWarning = {
  code: string;
  message: string;
};

export type ConnectionView = {
  connectionId: string;
  profileId: string;
  protocol: Protocol;
  state: "connecting" | "connected" | "disconnecting" | "error";
  startedAt: string;
  error?: DesktopError;
  transportWarning?: TransportWarning;
  favoriteId?: string | null;
};

export type ConnectionTestResult = {
  ok: boolean;
  error?: DesktopError;
};

export type Ownership = {
  owner: string;
  group?: string | null;
};

export type ProtocolCapabilities = {
  resume: boolean;
  permissions: boolean;
  ownership: boolean;
  hiddenFiles: boolean;
  symbolicLinks: boolean;
  serverSideRename: boolean;
  serverSideDelete: boolean;
  maxPathLength?: number | null;
};

export type FileEntryView = {
  name: string;
  path: string;
  entryType: "file" | "directory" | "symlink" | "other";
  size: number | null;
  modifiedAt: string | null;
  permissions: string | null;
  ownership: Ownership | null;
  hidden: boolean;
  symlinkTarget?: string | null;
  capabilities: ProtocolCapabilities;
};

export type ListingHandle = {
  listingId: string;
};

export type DirectoryBatchPayload = {
  listingId: string;
  entries: FileEntryView[];
  state: "partial" | "complete";
  firstEntryAt?: string;
  completedAt?: string;
};

export type DirectoryErrorPayload = {
  listingId: string;
  error: DesktopError;
};

export type PreviewOpenResult = {
  previewId: string;
  mode: "native" | "internal" | "metadata";
  reason?: string;
};

export type PreviewReadResult = {
  kind: "text" | "image";
  content: string;
  truncated: boolean;
};

export type TransferDirection = "upload" | "download";
export type TransferState =
  "pending" | "active" | "paused" | "completed" | "failed" | "cancelled" | "interrupted";
export type Priority = "low" | "normal" | "high";

export type TransferView = {
  id: string;
  direction: TransferDirection;
  connectionProfileId: string;
  sourcePath: string;
  destinationPath: string;
  totalBytes: number | null;
  transferredBytes: number;
  state: TransferState;
  priority: Priority;
  order: number;
  speedBytesPerSecond: number | null;
  estimatedRemainingSeconds: number | null;
  resumable: boolean;
  partialDataId: string | null;
  error: string | null;
  createdAt: string;
  updatedAt: string;
  startedAt: string | null;
  completedAt: string | null;
};

export type QueueView = {
  paused: boolean;
  concurrencyLimit: number;
  bandwidthLimitBytesPerSecond: number | null;
  orderedTransferIds: string[];
  activeTransferIds: string[];
};

export type TransferProgressEvent = {
  transferId: string;
  transferredBytes: number;
  totalBytes: number | null;
  speedBytesPerSecond: number | null;
  estimatedRemainingSeconds: number | null;
};

export type TransferStateEvent = {
  transferId: string;
  state: TransferState;
  error?: DesktopError;
};

export type ConflictPolicy = "ask" | "overwrite" | "skip" | "rename";

export type DirectoryFavorite = {
  id: string;
  connectionProfileId: string;
  path: string;
  label: string;
  sortOrder: number;
  createdAt: string;
  updatedAt: string;
};

export type ViewSettings = {
  sortKey: SortKey;
  showHidden: boolean;
  filter?: string | null;
};

export type BrowsingSession = {
  id: string;
  connectionProfileId: string | null;
  currentPath: string;
  localPath?: string | null;
  history: string[];
  historyIndex: number;
  viewSettings: ViewSettings;
  selectedItems: string[];
  localSelectedItems?: string[];
  createdAt: string;
  restored: boolean;
};

export type FileMetadata = {
  entry: FileEntryView;
  permissionsSupported: boolean;
  ownershipSupported: boolean;
};

export type CredentialPolicy = "save" | "ask";
export type FavoriteSortKey = "name" | "protocol" | "host" | "lastUsed";
export type HubTab = "favorites" | "history";
export type SidebarSectionKey = "servers" | "directoryFavorites" | "automation";

export type WorkspaceLayout = {
  splitRatio: number;
  sidebarVisible: boolean;
  sidebarSections: Record<SidebarSectionKey, boolean>;
  hubTab: HubTab;
};

export type ConnectionHistoryEntry = {
  favoriteId: string | null;
  connectionProfileId: string;
  displayName: string;
  protocol: Protocol;
  host: string;
  port: number;
  username: string;
  lastUsedAt: string;
};

export type SshKeyCandidate = {
  fileName: string;
  path: string;
};

export type SshKeyListing = {
  directory: string;
  exists: boolean;
  readable: boolean;
  candidates: SshKeyCandidate[];
};

export type SshKeyValidation = {
  valid: boolean;
  code?: "missing" | "not_a_file" | "unreadable" | "public_key" | "unsupported";
  message?: string;
};

export type PathSource = "local" | "remote";
export type PathFallback = "none" | "ancestor" | "home" | "login";

export type PathResolution = {
  originalPath: string;
  path: string;
  fallback: PathFallback;
  notice?: {
    code: string;
    message: string;
  };
};

export type HistoryClearResult = {
  cleared: boolean;
  removed: number;
};

export type ServerFavoriteInput = {
  displayName: string;
  groupLabel?: string | null;
  protocol: Protocol;
  host: string;
  port: number;
  username: string;
  authMethod: AuthMethod;
  timeoutSeconds: number;
  sshKeyRef?: string | null;
  pinned?: boolean;
  localPath?: string | null;
  remotePath?: string | null;
};

export type ServerFavoriteView = ServerFavoriteInput & {
  id: string;
  connectionProfileId: string;
  pinned: boolean;
  lastUsedAt: string | null;
  credentialPolicy: CredentialPolicy;
  hasSavedCredential: boolean;
  localPath: string | null;
  remotePath: string | null;
  createdAt: string;
  updatedAt: string;
  credentialWarning?: {
    code: string;
    message: string;
  } | null;
};

export type FavoritePreferences = {
  sortKey: FavoriteSortKey;
  pinnedFirst: boolean;
};

export type AffectedFavorite = {
  id: string;
  displayName: string;
};

export type FavoriteDeleteResult = {
  deleted: boolean;
  orphanedCredentialRefId?: string;
  credentialImpact?: CredentialImpact;
};

export type CredentialRemoveResult = {
  removed: boolean;
  impact: CredentialImpact;
};

export type RenameGroupResult = {
  updated: ServerFavoriteView[];
};

export type ActiveProfileResult = {
  connectionId: string;
  favoriteId?: string;
  profile: ConnectionProfile;
};

export type CredentialImpact = {
  credentialRefId: string;
  affectedFavorites: AffectedFavorite[];
  isShared: boolean;
};

export type ReusableCredential = {
  credentialRefId: string;
  protocol: Protocol;
  host: string;
  port: number;
  username: string;
  authMethod: AuthMethod;
  affectedFavoriteCount: number;
};
