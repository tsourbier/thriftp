import { writable } from "svelte/store";
import { ipc } from "../ipc/client";
import type {
  ConnectionView,
  CredentialRemoveResult,
  FavoriteDeleteResult,
  FavoritePreferences,
  ReusableCredential,
  ServerFavoriteView
} from "../ipc/types";

export const serverFavorites = writable<ServerFavoriteView[]>([]);
export const favoritePreferences = writable<FavoritePreferences>({
  sortKey: "name",
  pinnedFirst: true
});
export const serverFavoritesError = writable<string | null>(null);

/** Per-favorite connection state used by the one-action connect control. */
export type FavoriteConnectionState = {
  state: "connecting" | "connected" | "error";
  error?: string;
};

export const favoriteConnectionStates = writable<Record<string, FavoriteConnectionState>>(
  {}
);

export async function loadServerFavorites(): Promise<void> {
  try {
    const [items, preferences] = await Promise.all([
      ipc.serverFavoritesList(),
      ipc.serverFavoritesPreferences()
    ]);
    serverFavorites.set(items);
    favoritePreferences.set(preferences);
    serverFavoritesError.set(null);
  } catch (error) {
    serverFavoritesError.set(
      error instanceof Error ? error.message : "Could not load server favorites."
    );
  }
}

function setFavoriteConnectionState(
  favoriteId: string,
  state: FavoriteConnectionState | null
): void {
  favoriteConnectionStates.update((current) => {
    const next = { ...current };
    if (state) {
      next[favoriteId] = state;
    } else {
      delete next[favoriteId];
    }
    return next;
  });
}

/**
 * Connect with one action and surface connecting/connected/error state.
 *
 * The returned connection still belongs to the caller; the active-connection
 * store is reconciled separately from authoritative connection events.
 */
export async function connectFavorite(
  id: string,
  secret?: string,
  secretKind?: "password" | "privateKey" | "passphrase"
): Promise<ConnectionView> {
  setFavoriteConnectionState(id, { state: "connecting" });
  try {
    const connection = await ipc.serverFavoritesConnect(id, secret, secretKind);
    setFavoriteConnectionState(id, { state: "connected" });
    return connection;
  } catch (error) {
    setFavoriteConnectionState(id, {
      state: "error",
      error: error instanceof Error ? error.message : "Connection failed."
    });
    throw error;
  }
}

export async function disconnectFavorite(
  favoriteId: string,
  connectionId: string
): Promise<void> {
  await ipc.disconnect(connectionId);
  setFavoriteConnectionState(favoriteId, null);
}

/**
 * Reconcile favorite rows with authoritative active connections.
 *
 * Called whenever the connection list changes so deleted, disconnected, and
 * errored sessions are reflected without polling.
 */
export function applyActiveConnections(
  favorites: ServerFavoriteView[],
  connections: ConnectionView[]
): void {
  favoriteConnectionStates.update((current) => {
    const next: Record<string, FavoriteConnectionState> = {};
    for (const favorite of favorites) {
      const connection = connections.find(
        (candidate) => candidate.profileId === favorite.connectionProfileId
      );
      if (!connection) {
        continue;
      }
      const pending = current[favorite.id];
      next[favorite.id] = {
        state:
          connection.state === "error"
            ? "error"
            : connection.state === "connecting"
              ? "connecting"
              : "connected",
        error: connection.error?.message ?? pending?.error
      };
    }
    return next;
  });
}

export function activeFavoriteId(
  favorites: ServerFavoriteView[],
  connections: ConnectionView[]
): string | null {
  const connection = connections.find((candidate) => candidate.state !== "error");
  if (!connection) {
    return null;
  }
  return (
    favorites.find((favorite) => favorite.connectionProfileId === connection.profileId)
      ?.id ?? null
  );
}

export async function setPinned(id: string, pinned: boolean): Promise<void> {
  const updated = await ipc.serverFavoritesSetPinned(id, pinned);
  serverFavorites.update((items) =>
    items.map((item) => (item.id === updated.id ? updated : item))
  );
}

export async function deleteFavorite(id: string): Promise<FavoriteDeleteResult> {
  const result = await ipc.serverFavoritesDelete(id);
  serverFavorites.update((items) => items.filter((item) => item.id !== id));
  setFavoriteConnectionState(id, null);
  return result;
}

export async function reusableCredentials(): Promise<ReusableCredential[]> {
  return ipc.credentialsReusable();
}

export async function setPreferences(preferences: FavoritePreferences): Promise<void> {
  favoritePreferences.set(await ipc.serverFavoritesSetPreferences(preferences));
  await loadServerFavorites();
}

export async function removeCredential(
  favoriteId: string,
  confirmed: boolean
): Promise<CredentialRemoveResult> {
  const result = await ipc.credentialsRemove(favoriteId, confirmed);
  await loadServerFavorites();
  return result;
}
