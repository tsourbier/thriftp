import { describe, expect, it } from "vitest";
import { connectionStatistics, formatBytes } from "./stats";
import type { ConnectionProfile, ConnectionView, TransferView } from "../ipc/types";

function connection(id: string, profileId: string, startedAt: string): ConnectionView {
  return {
    connectionId: id,
    profileId,
    protocol: "sftp",
    state: "connected",
    startedAt
  };
}

function profile(id: string, displayName: string): ConnectionProfile {
  return {
    id,
    displayName,
    protocol: "sftp",
    host: "example.test",
    port: 22,
    username: "tester",
    authMethod: "password",
    timeoutSeconds: 30,
    createdAt: "2026-01-01T00:00:00Z",
    updatedAt: "2026-01-01T00:00:00Z",
    lastUsedAt: null
  };
}

function transfer(
  id: string,
  profileId: string,
  state: TransferView["state"],
  startedAt: string,
  completedAt: string | null,
  transferredBytes: number
): TransferView {
  return {
    id,
    direction: "download",
    connectionProfileId: profileId,
    sourcePath: `/remote/${id}`,
    destinationPath: `/local/${id}`,
    totalBytes: transferredBytes,
    transferredBytes,
    state,
    priority: "normal",
    order: 0,
    speedBytesPerSecond: null,
    estimatedRemainingSeconds: null,
    resumable: false,
    partialDataId: null,
    error: null,
    createdAt: startedAt,
    updatedAt: completedAt ?? startedAt,
    startedAt,
    completedAt
  };
}

describe("connection transfer statistics", () => {
  const connections = [
    connection("c1", "p1", "2026-01-01T00:00:00Z"),
    connection("c2", "p2", "2026-01-01T00:00:00Z")
  ];
  const profiles = [profile("p1", "Alpha"), profile("p2", "Beta")];

  it("counts only completed transfers for the current connection session", () => {
    const stats = connectionStatistics(
      connections,
      [
        transfer(
          "a",
          "p1",
          "completed",
          "2026-01-01T00:00:10Z",
          "2026-01-01T00:00:20Z",
          100
        ),
        transfer("b", "p1", "failed", "2026-01-01T00:00:10Z", "2026-01-01T00:00:20Z", 50),
        transfer(
          "c",
          "p2",
          "completed",
          "2026-01-01T00:00:10Z",
          "2026-01-01T00:00:30Z",
          200
        )
      ],
      profiles
    );
    expect(stats.find((item) => item.connectionId === "c1")).toMatchObject({
      displayName: "Alpha",
      completedFiles: 1,
      completedBytes: 100,
      averageBytesPerSecond: 10
    });
    expect(stats.find((item) => item.connectionId === "c2")).toMatchObject({
      completedFiles: 1,
      completedBytes: 200,
      averageBytesPerSecond: 10
    });
  });

  it("excludes transfers started before the current session", () => {
    const stats = connectionStatistics(
      [connection("c1", "p1", "2026-01-01T00:01:00Z")],
      [
        transfer(
          "old",
          "p1",
          "completed",
          "2026-01-01T00:00:10Z",
          "2026-01-01T00:00:20Z",
          100
        ),
        transfer(
          "new",
          "p1",
          "completed",
          "2026-01-01T00:01:10Z",
          "2026-01-01T00:01:20Z",
          300
        )
      ],
      profiles
    );
    expect(stats[0]).toMatchObject({ completedFiles: 1, completedBytes: 300 });
  });

  it("uses exact completed transfer bytes instead of a larger total", () => {
    const exact = transfer(
      "exact",
      "p1",
      "completed",
      "2026-01-01T00:00:10Z",
      "2026-01-01T00:00:20Z",
      1000
    );
    exact.totalBytes = 2000;
    const fallback = transfer(
      "fallback",
      "p1",
      "completed",
      "2026-01-01T00:00:10Z",
      "2026-01-01T00:00:20Z",
      0
    );
    fallback.totalBytes = 2000;

    const stats = connectionStatistics(
      [connection("c1", "p1", "2026-01-01T00:00:00Z")],
      [exact, fallback],
      profiles
    );
    expect(stats[0].completedBytes).toBe(3000);
  });

  it("formats volume through byte units", () => {
    expect(formatBytes(900)).toBe("900 B");
    expect(formatBytes(1024)).toBe("1.0 KB");
    expect(formatBytes(1024 * 1024)).toBe("1.0 MB");
    expect(formatBytes(1024 * 1024 * 1024)).toBe("1.0 GB");
  });
});
