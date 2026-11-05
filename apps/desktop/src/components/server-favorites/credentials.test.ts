import { cleanup, fireEvent, render, screen } from "@testing-library/svelte";
import { afterEach, describe, expect, it, vi } from "vitest";

vi.mock("../../lib/ipc/client", () => ({
  ipc: {
    credentialsRemove: vi.fn(),
    serverFavoritesList: vi.fn(),
    serverFavoritesPreferences: vi.fn(),
    serverFavoritesConnect: vi.fn(),
    serverFavoritesSetPinned: vi.fn(),
    serverFavoritesDelete: vi.fn(),
    serverFavoritesSetPreferences: vi.fn(),
    disconnect: vi.fn()
  }
}));

import { ipc } from "../../lib/ipc/client";
import CredentialImpactDialog from "./CredentialImpactDialog.svelte";
import CredentialUnavailableBanner from "./CredentialUnavailableBanner.svelte";
import DeleteCredentialDialog from "./DeleteCredentialDialog.svelte";
import EditFavoriteDialog from "./EditFavoriteDialog.svelte";
import ReuseCredentialDialog from "./ReuseCredentialDialog.svelte";
import { removeCredential } from "../../lib/server-favorites/stores";
import { favoriteFixture, impactFixture } from "../../lib/server-favorites/test-utils";
import type { ReusableCredential } from "../../lib/ipc/types";

afterEach(cleanup);

const reusable: ReusableCredential = {
  credentialRefId: "ref-1",
  protocol: "sftp",
  host: "shared.test",
  port: 22,
  username: "tester",
  authMethod: "password",
  affectedFavoriteCount: 2
};

describe("credential lifecycle", () => {
  it("saves a credential from the edit form and updates the existing secret", async () => {
    const onsave = vi.fn();
    render(EditFavoriteDialog, {
      props: {
        favorite: favoriteFixture({
          credentialPolicy: "save",
          hasSavedCredential: true
        }),
        onsave,
        oncancel: vi.fn()
      }
    });

    await fireEvent.change(screen.getByLabelText("Credential policy"), {
      target: { value: "save" }
    });
    await fireEvent.input(screen.getByLabelText("Password"), {
      target: { value: "updated-secret" }
    });
    await fireEvent.click(screen.getByRole("button", { name: "Save" }));

    expect(onsave).toHaveBeenCalledOnce();
    const [, policy, secret] = onsave.mock.calls[0];
    expect(policy).toBe("save");
    expect(secret).toBe("updated-secret");
  });

  it("requires a new secret when identity fields change", async () => {
    const onsave = vi.fn();
    render(EditFavoriteDialog, {
      props: {
        favorite: favoriteFixture({
          credentialPolicy: "save",
          hasSavedCredential: true
        }),
        onsave,
        oncancel: vi.fn()
      }
    });

    await fireEvent.input(screen.getByLabelText("Host"), {
      target: { value: "moved.example" }
    });
    await fireEvent.click(screen.getByRole("button", { name: "Save" }));

    expect(onsave).not.toHaveBeenCalled();
    expect(screen.getByRole("alert").textContent).toContain(
      "Connection identity changed"
    );
  });

  it("removes a credential only after confirmation", async () => {
    vi.mocked(ipc.credentialsRemove).mockResolvedValue({
      removed: true,
      impact: impactFixture()
    });
    vi.mocked(ipc.serverFavoritesList).mockResolvedValue([]);
    vi.mocked(ipc.serverFavoritesPreferences).mockResolvedValue({
      sortKey: "name",
      pinnedFirst: true
    });

    await removeCredential("favorite-1", true);
    expect(ipc.credentialsRemove).toHaveBeenCalledWith("favorite-1", true);
    expect(ipc.serverFavoritesList).toHaveBeenCalled();
  });

  it("lists every favorite affected by a shared credential update", async () => {
    const onconfirm = vi.fn();
    render(CredentialImpactDialog, {
      props: {
        impact: impactFixture({
          isShared: true,
          affectedFavorites: [
            { id: "favorite-1", displayName: "Alpha" },
            { id: "favorite-2", displayName: "Beta" }
          ]
        }),
        onconfirm,
        oncancel: vi.fn()
      }
    });
    expect(screen.getByText("Alpha")).toBeTruthy();
    expect(screen.getByText("Beta")).toBeTruthy();
    expect(screen.getByText(/applies to every favorite/)).toBeTruthy();
    await fireEvent.click(screen.getByRole("button", { name: "Continue" }));
    expect(onconfirm).toHaveBeenCalledOnce();
  });

  it("offers only identity-compatible reusable credentials", async () => {
    const onselect = vi.fn();
    render(ReuseCredentialDialog, {
      props: { credentials: [reusable], onselect, oncancel: vi.fn() }
    });
    expect(screen.getByText(/2 favorites/)).toBeTruthy();
    await fireEvent.click(screen.getByRole("button", { name: /sftp/ }));
    expect(onselect).toHaveBeenCalledWith("ref-1");
  });

  it("warns about an unavailable store and offers repair for a corrupt reference", async () => {
    const onrepair = vi.fn();
    const { unmount } = render(CredentialUnavailableBanner, {
      props: { code: "credential_store_unavailable", onretry: vi.fn() }
    });
    expect(screen.getByText(/No plaintext fallback/)).toBeTruthy();
    unmount();

    render(CredentialUnavailableBanner, {
      props: { code: "credential_missing", onrepair }
    });
    expect(screen.getByText(/saved credential is missing/)).toBeTruthy();
    await fireEvent.click(screen.getByRole("button", { name: "Repair credential" }));
    expect(onrepair).toHaveBeenCalledOnce();
  });

  it("prompts before deleting the last reference and supports both outcomes", async () => {
    const onkeep = vi.fn();
    const ondelete = vi.fn();
    render(DeleteCredentialDialog, {
      props: { onkeep, ondelete, oncancel: vi.fn() }
    });
    expect(screen.getByText(/No favorite references this credential/)).toBeTruthy();
    await fireEvent.click(screen.getByRole("button", { name: "Keep" }));
    expect(onkeep).toHaveBeenCalledOnce();

    await fireEvent.click(screen.getByRole("button", { name: "Delete" }));
    expect(ondelete).toHaveBeenCalledOnce();
  });
});
