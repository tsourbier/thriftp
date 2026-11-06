import type { CredentialImpact, ServerFavoriteView } from "../ipc/types";

export function favoriteFixture(
  overrides: Partial<ServerFavoriteView> = {}
): ServerFavoriteView {
  return {
    id: "favorite-1",
    connectionProfileId: "profile-1",
    displayName: "Example",
    groupLabel: null,
    protocol: "sftp",
    host: "example.test",
    port: 22,
    username: "tester",
    authMethod: "password",
    timeoutSeconds: 30,
    sshKeyRef: null,
    pinned: false,
    lastUsedAt: null,
    credentialPolicy: "ask",
    hasSavedCredential: false,
    localPath: null,
    remotePath: null,
    createdAt: new Date().toISOString(),
    updatedAt: new Date().toISOString(),
    ...overrides
  };
}

export function impactFixture(
  overrides: Partial<CredentialImpact> = {}
): CredentialImpact {
  return {
    credentialRefId: "credential-1",
    affectedFavorites: [{ id: "favorite-1", displayName: "Example" }],
    isShared: false,
    ...overrides
  };
}
