import { cleanup, fireEvent, render, screen } from "@testing-library/svelte";
import { get } from "svelte/store";
import { afterEach, describe, expect, it, vi } from "vitest";

vi.mock("../../lib/ipc/client", () => ({
  ipc: {
    serverFavoritesConnect: vi.fn(),
    disconnect: vi.fn(),
    serverFavoritesList: vi.fn(),
    serverFavoritesPreferences: vi.fn(),
    serverFavoritesSetPinned: vi.fn(),
    serverFavoritesDelete: vi.fn(),
    serverFavoritesSetPreferences: vi.fn(),
    credentialsRemove: vi.fn()
  }
}));

import { ipc } from "../../lib/ipc/client";
import CredentialPromptDialog from "./CredentialPromptDialog.svelte";
import FavoritesList from "./FavoritesList.svelte";
import {
  activeFavoriteId,
  applyActiveConnections,
  connectFavorite,
  favoriteConnectionStates
} from "../../lib/server-favorites/stores";
import { favoriteFixture } from "../../lib/server-favorites/test-utils";
import type { ConnectionView } from "../../lib/ipc/types";

afterEach(() => {
  cleanup();
  favoriteConnectionStates.set({});
});

function connection(overrides: Partial<ConnectionView> = {}): ConnectionView {
  return {
    connectionId: "connection-1",
    profileId: "profile-1",
    protocol: "sftp",
    state: "connected",
    startedAt: new Date().toISOString(),
    ...overrides
  };
}

describe("one-action favorite connect", () => {
  it("exposes connecting and connected state around the IPC call", async () => {
    let resolve!: (value: ConnectionView) => void;
    const pending = new Promise<ConnectionView>((done) => (resolve = done));
    vi.mocked(ipc.serverFavoritesConnect).mockReturnValue(pending);

    const result = connectFavorite("favorite-1");
    expect(get(favoriteConnectionStates)["favorite-1"]).toEqual({
      state: "connecting"
    });

    resolve(connection());
    await expect(result).resolves.toMatchObject({ connectionId: "connection-1" });
    expect(get(favoriteConnectionStates)["favorite-1"]).toEqual({
      state: "connected"
    });
  });

  it("reports failure feedback without leaking a secret", async () => {
    vi.mocked(ipc.serverFavoritesConnect).mockRejectedValue(
      new Error("A credential is required to connect to this server.")
    );

    await expect(connectFavorite("favorite-1", "s3cret-sentinel")).rejects.toThrow(
      "A credential is required"
    );
    const state = get(favoriteConnectionStates)["favorite-1"];
    expect(state?.state).toBe("error");
    expect(state?.error).not.toContain("s3cret-sentinel");
  });

  it("reconciles active connections into favorite state", () => {
    const favorites = [
      favoriteFixture({ id: "favorite-1", connectionProfileId: "profile-1" })
    ];
    applyActiveConnections(favorites, [connection({ state: "connecting" })]);
    expect(get(favoriteConnectionStates)["favorite-1"].state).toBe("connecting");

    applyActiveConnections(favorites, [connection({ state: "connected" })]);
    expect(get(favoriteConnectionStates)["favorite-1"].state).toBe("connected");

    applyActiveConnections(favorites, []);
    expect(get(favoriteConnectionStates)["favorite-1"]).toBeUndefined();
    expect(activeFavoriteId(favorites, [connection()])).toBe("favorite-1");
  });

  it("prompts for a rejected credential and names the favorite", async () => {
    const onsubmit = vi.fn();
    render(CredentialPromptDialog, {
      props: {
        code: "credential_rejected",
        favoriteName: "Production",
        onsubmit,
        oncancel: vi.fn()
      }
    });
    expect(screen.getByText(/Production/)).toBeTruthy();
    await fireEvent.input(screen.getByLabelText("Password"), {
      target: { value: "replacement" }
    });
    await fireEvent.click(screen.getByRole("button", { name: "Connect" }));
    expect(onsubmit).toHaveBeenCalledWith("replacement", false);
  });

  it("warns when only a session credential is available", () => {
    render(CredentialPromptDialog, {
      props: {
        code: "credential_store_unavailable",
        onsubmit: vi.fn(),
        oncancel: vi.fn()
      }
    });
    expect(
      screen.getByText(/operating-system credential store is unavailable/)
    ).toBeTruthy();
    expect(screen.getByText(/discarded after the session/)).toBeTruthy();
  });

  it("shows the active-connection state in the favorites list", () => {
    const favorites = [favoriteFixture({ id: "favorite-1", displayName: "Alpha" })];
    render(FavoritesList, {
      props: {
        favorites,
        activeFavoriteId: "favorite-1",
        connectionStates: { "favorite-1": { state: "connected" } },
        onconnect: vi.fn(),
        onedit: vi.fn(),
        ondelete: vi.fn(),
        onpin: vi.fn()
      }
    });
    expect(screen.getByText("connected")).toBeTruthy();
    expect(
      screen
        .getByRole("button", { name: "Connect to Alpha" })
        .getAttribute("aria-current")
    ).toBe("true");
  });
});
