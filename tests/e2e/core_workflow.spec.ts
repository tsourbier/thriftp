import { beforeEach, describe, expect, it, vi } from "vitest";

import { ipc } from "../../apps/desktop/src/lib/ipc/client";

const invoke = vi.fn();
Object.assign(window, { __TAURI_INTERNALS__: { invoke } });

const profile = {
  id: "profile-1",
  displayName: "Test server",
  groupLabel: null,
  protocol: "sftp" as const,
  host: "example.test",
  port: 22,
  username: "tester",
  authMethod: "password" as const,
  timeoutSeconds: 30,
  sshKeyRef: null,
  createdAt: "2026-01-01T00:00:00Z",
  updatedAt: "2026-01-01T00:00:00Z"
};

describe("core desktop workflow without protocol documentation", () => {
  beforeEach(() => {
    invoke.mockReset();
  });

  it("connects, browses, transfers, and disconnects through the IPC contract", async () => {
    invoke
      .mockResolvedValueOnce([profile])
      .mockResolvedValueOnce({
        connectionId: "connection-1",
        profileId: profile.id,
        protocol: "sftp",
        state: "connected",
        startedAt: "2026-01-01T00:00:00Z"
      })
      .mockResolvedValueOnce({ listingId: "listing-1" })
      .mockResolvedValueOnce({ transferIds: ["transfer-1"] })
      .mockResolvedValueOnce([
        {
          id: "transfer-1",
          direction: "upload",
          connectionProfileId: profile.id,
          sourcePath: "/local/report.txt",
          destinationPath: "/remote/report.txt",
          totalBytes: 6,
          transferredBytes: 6,
          state: "completed",
          priority: "normal",
          order: 0,
          speedBytesPerSecond: null,
          estimatedRemainingSeconds: null,
          resumable: false,
          partialDataId: null,
          error: null,
          createdAt: "2026-01-01T00:00:00Z",
          updatedAt: "2026-01-01T00:00:00Z",
          startedAt: "2026-01-01T00:00:00Z",
          completedAt: "2026-01-01T00:00:01Z"
        }
      ])
      .mockResolvedValueOnce({ disconnected: true });

    const profiles = await ipc.listProfiles();
    expect(profiles).toHaveLength(1);

    const connection = await ipc.connect(profile.id);
    expect(connection.connectionId).toBe("connection-1");

    const listing = await ipc.openRemote(connection.connectionId, ".");
    expect(listing.listingId).toBe("listing-1");

    const started = await ipc.transfersStart({
      direction: "upload",
      profileId: profile.id,
      sources: ["/local/report.txt"],
      destination: "/remote"
    });
    expect(started.transferIds).toEqual(["transfer-1"]);

    const [transfer] = await ipc.transfersList();
    expect(transfer.state).toBe("completed");

    await ipc.disconnect(connection.connectionId);
    const lastCall = invoke.mock.lastCall;
    expect(lastCall?.[0]).toBe("connections_disconnect");
    expect(lastCall?.[1]).toMatchObject({ connectionId: "connection-1" });
  });
});
