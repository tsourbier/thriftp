export type DisconnectPolicy = "continue" | "cancel";

export interface AutomationStatus {
  enabled: boolean;
  mode: string;
  hostRole: "host" | "observer" | "none";
  clients: McpClientView[];
  grants: GrantView[];
  scopes: ScopeView[];
  pendingRequests: EnrollmentRequestView[];
  sessions: SessionView[];
  contractVersion: string;
  protocolVersions: string[];
}

export interface ScopeView {
  id: string;
  grantId: string;
  kind: "remote" | "local";
  profileId: string | null;
  canonicalPath: string;
  recursive: boolean;
}

export interface McpClientView {
  id: string;
  fingerprint: string;
  displayName: string;
  state: string;
  enrolledAt: string | null;
  lastSeenAt: string | null;
}

export interface EnrollmentRequestView {
  id: string;
  fingerprint: string;
  displayName: string;
  requestedAt: string;
  expiresAt: string;
}

export interface SessionView {
  token: string;
  fingerprint: string;
  displayName?: string;
  grantId?: string;
  grantRevision?: number;
  revocationEpoch?: number;
  revision?: number;
  epoch?: number;
  expiresAt: string;
  disconnected?: boolean;
  disconnectPolicy?: DisconnectPolicy;
}

export interface GrantView {
  grantId: string;
  revision: number;
  epoch: number;
  expiresAt: string | null;
  profiles: string[];
  operations: string[];
  disconnectPolicy: DisconnectPolicy;
  scopes: Array<{
    id: string;
    kind: "remote" | "local";
    profileId: string | null;
    canonicalPath: string;
    recursive: boolean;
  }>;
}

export interface AuditEventView {
  seq: number;
  timestamp: string;
  interface: string;
  mode: string;
  clientFingerprint: string | null;
  operation: string;
  target: unknown;
  result: string;
  errorCode: string | null;
}

export interface RetentionReport {
  evaluatedSegments: number;
  expiredSegments: number;
  deletedSegments: string[];
  noticeIssued: boolean;
  noticePending: boolean;
  storageLimitExceeded: boolean;
  totalBytes: number;
}

export interface ApprovalInput {
  requestId: string;
  profileIds: string[];
  directories: string[];
  localDirectories: string[];
  operations: string[];
  expiresAt?: string;
  disconnectPolicy: DisconnectPolicy;
  preApprove: string[];
}
