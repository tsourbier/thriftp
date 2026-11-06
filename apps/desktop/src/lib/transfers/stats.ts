import type { ConnectionProfile, ConnectionView, TransferView } from "../ipc/types";

export type ConnectionTransferStats = {
  connectionId: string;
  connectionProfileId: string;
  displayName: string;
  completedFiles: number;
  completedBytes: number;
  averageBytesPerSecond: number | null;
};

function timestamp(value: string | null | undefined): number | null {
  if (!value) {
    return null;
  }
  const parsed = Date.parse(value);
  return Number.isFinite(parsed) ? parsed : null;
}

export function connectionStatistics(
  connections: ConnectionView[],
  transfers: TransferView[],
  profiles: ConnectionProfile[]
): ConnectionTransferStats[] {
  return connections.map((connection) => {
    const sessionStart = timestamp(connection.startedAt) ?? 0;
    const completed = transfers.filter((transfer) => {
      if (
        transfer.connectionProfileId !== connection.profileId ||
        transfer.state !== "completed"
      ) {
        return false;
      }
      const startedAt = timestamp(transfer.startedAt);
      const completedAt = timestamp(transfer.completedAt);
      return startedAt !== null && completedAt !== null && startedAt >= sessionStart;
    });
    const completedBytes = completed.reduce(
      (total, transfer) =>
        total +
        (transfer.transferredBytes > 0
          ? transfer.transferredBytes
          : (transfer.totalBytes ?? 0)),
      0
    );
    const durationMs = completed.reduce((total, transfer) => {
      const startedAt = timestamp(transfer.startedAt);
      const completedAt = timestamp(transfer.completedAt);
      return startedAt === null || completedAt === null
        ? total
        : total + Math.max(0, completedAt - startedAt);
    }, 0);
    const displayName =
      profiles.find((profile) => profile.id === connection.profileId)?.displayName ??
      connection.profileId;
    return {
      connectionId: connection.connectionId,
      connectionProfileId: connection.profileId,
      displayName,
      completedFiles: completed.length,
      completedBytes,
      averageBytesPerSecond:
        completedBytes > 0 && durationMs > 0
          ? Math.round(completedBytes / (durationMs / 1000))
          : null
    };
  });
}

export function formatBytes(bytes: number): string {
  const units = ["B", "KB", "MB", "GB", "TB"];
  let value = bytes;
  let unit = 0;
  while (value >= 1024 && unit < units.length - 1) {
    value /= 1024;
    unit += 1;
  }
  return unit === 0 ? `${value} ${units[unit]}` : `${value.toFixed(1)} ${units[unit]}`;
}

export function formatSpeed(bytesPerSecond: number | null): string {
  return bytesPerSecond === null ? "—" : `${formatBytes(bytesPerSecond)}/s`;
}
