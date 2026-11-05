import { writable } from "svelte/store";
import type { ConnectionView, FileEntryView } from "./types";

export const localEntries = writable<FileEntryView[]>([]);
export const remoteEntries = writable<FileEntryView[]>([]);
export const activePanel = writable<"local" | "remote">("local");
export const connectionState = writable<
  "disconnected" | "connecting" | "connected" | "error"
>("disconnected");

/**
 * Authoritative active connections mirrored from the desktop shell.
 *
 * Keeping this in a store lets independent panels (favorites, tabs, status)
 * react to the same connection lifecycle instead of polling.
 */
export const activeConnections = writable<ConnectionView[]>([]);

export function setActiveConnections(connections: ConnectionView[]): void {
  activeConnections.set(connections);
  if (connections.length === 0) {
    connectionState.set("disconnected");
    return;
  }
  if (connections.some((connection) => connection.state === "error")) {
    connectionState.set("error");
  } else if (connections.some((connection) => connection.state === "connecting")) {
    connectionState.set("connecting");
  } else {
    connectionState.set("connected");
  }
}

export function applyConnectionEvent(connection: ConnectionView): void {
  activeConnections.update((connections) => {
    const index = connections.findIndex(
      (candidate) => candidate.connectionId === connection.connectionId
    );
    if (index === -1) {
      return [...connections, connection];
    }
    return connections.map((candidate, candidateIndex) =>
      candidateIndex === index ? connection : candidate
    );
  });
}

export function removeConnection(connectionId: string): void {
  activeConnections.update((connections) =>
    connections.filter((candidate) => candidate.connectionId !== connectionId)
  );
}

export function connectionForProfile(
  connections: ConnectionView[],
  profileId: string
): ConnectionView | null {
  return connections.find((connection) => connection.profileId === profileId) ?? null;
}
