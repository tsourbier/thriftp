import { afterEach, describe, expect, it, vi } from "vitest";

vi.mock("../ipc/client", () => ({
  ipc: {
    getProfile: vi.fn(),
    serverFavoritesCreate: vi.fn()
  }
}));

import { ipc } from "../ipc/client";
import { prefillFavorite, saveFavorite } from "./api";
import { favoriteFixture } from "./test-utils";

afterEach(() => {
  vi.resetAllMocks();
});

describe("server favorite API", () => {
  it("prefills active-connection settings including timeout and SSH key reference", async () => {
    vi.mocked(ipc.getProfile).mockResolvedValue({
      connectionId: "connection-1",
      favoriteId: "favorite-1",
      profile: {
        id: "profile-1",
        displayName: "Production",
        groupLabel: "Prod",
        protocol: "sftp",
        host: "prod.example",
        port: 2222,
        username: "deploy",
        authMethod: "sshPrivateKey",
        timeoutSeconds: 45,
        sshKeyRef: "~/.ssh/id_ed25519",
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
        lastUsedAt: null
      }
    });

    const prefill = await prefillFavorite("connection-1");
    expect(ipc.getProfile).toHaveBeenCalledWith("connection-1");
    expect(prefill.favoriteId).toBe("favorite-1");
    expect(prefill.initial).toMatchObject({
      displayName: "Production",
      timeoutSeconds: 45,
      sshKeyRef: "~/.ssh/id_ed25519",
      authMethod: "sshPrivateKey"
    });
  });

  it("creates a favorite through the IPC contract", async () => {
    const view = favoriteFixture({ displayName: "New" });
    vi.mocked(ipc.serverFavoritesCreate).mockResolvedValue(view);
    const profile = {
      displayName: "New",
      protocol: "sftp" as const,
      host: "example.test",
      port: 22,
      username: "tester",
      authMethod: "password" as const,
      timeoutSeconds: 30,
      pinned: false
    };
    await expect(saveFavorite(profile, "ask")).resolves.toEqual(view);
    expect(ipc.serverFavoritesCreate).toHaveBeenCalledWith(
      profile,
      "ask",
      undefined,
      undefined
    );
  });
});
