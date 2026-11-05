import { cleanup, fireEvent, render, screen } from "@testing-library/svelte";
import { afterEach, describe, expect, it, vi } from "vitest";

import CredentialImpactDialog from "./CredentialImpactDialog.svelte";
import CredentialPromptDialog from "./CredentialPromptDialog.svelte";
import CredentialUnavailableBanner from "./CredentialUnavailableBanner.svelte";
import DeleteCredentialDialog from "./DeleteCredentialDialog.svelte";
import DeleteFavoriteDialog from "./DeleteFavoriteDialog.svelte";
import GroupMenu from "./GroupMenu.svelte";
import type { CredentialImpact, ServerFavoriteView } from "../../lib/ipc/types";

afterEach(cleanup);

function favorite(): ServerFavoriteView {
  return {
    id: "favorite-1",
    connectionProfileId: "profile-1",
    displayName: "Alpha",
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
    createdAt: new Date().toISOString(),
    updatedAt: new Date().toISOString()
  };
}

const impact: CredentialImpact = {
  credentialRefId: "ref-1",
  affectedFavorites: [{ id: "favorite-1", displayName: "Alpha" }],
  isShared: false
};

describe("server favorite dialogs", () => {
  it("assigns a group label", async () => {
    const onassign = vi.fn();
    render(GroupMenu, { props: { onassign, onrename: vi.fn(), onungroup: vi.fn() } });
    await fireEvent.input(screen.getByLabelText("Group"), { target: { value: "Prod" } });
    await fireEvent.click(screen.getByRole("button", { name: "Assign" }));
    expect(onassign).toHaveBeenCalledWith("Prod");
  });

  it("confirms favorite deletion", async () => {
    const onconfirm = vi.fn();
    render(DeleteFavoriteDialog, {
      props: { favorite: favorite(), onconfirm, oncancel: vi.fn() }
    });
    await fireEvent.click(screen.getByRole("button", { name: "Delete" }));
    expect(onconfirm).toHaveBeenCalledOnce();
  });

  it("prompts for a session credential", async () => {
    const onsubmit = vi.fn();
    render(CredentialPromptDialog, { props: { onsubmit, oncancel: vi.fn() } });
    await fireEvent.input(screen.getByLabelText("Password"), {
      target: { value: "secret" }
    });
    await fireEvent.click(screen.getByRole("button", { name: "Connect" }));
    expect(onsubmit).toHaveBeenCalledWith("secret", false);
  });

  it("lists shared credential impact", async () => {
    const onconfirm = vi.fn();
    render(CredentialImpactDialog, {
      props: { impact: { ...impact, isShared: true }, onconfirm, oncancel: vi.fn() }
    });
    expect(screen.getByText("Alpha")).toBeTruthy();
    await fireEvent.click(screen.getByRole("button", { name: "Continue" }));
    expect(onconfirm).toHaveBeenCalledOnce();
  });

  it("offers keep or delete for the last credential", async () => {
    const onkeep = vi.fn();
    render(DeleteCredentialDialog, {
      props: { onkeep, ondelete: vi.fn(), oncancel: vi.fn() }
    });
    await fireEvent.click(screen.getByRole("button", { name: "Keep" }));
    expect(onkeep).toHaveBeenCalledOnce();
  });

  it("shows a repair action for unavailable credentials", async () => {
    const onrepair = vi.fn();
    render(CredentialUnavailableBanner, {
      props: { code: "credential_missing", onrepair }
    });
    await fireEvent.click(screen.getByRole("button", { name: "Repair credential" }));
    expect(onrepair).toHaveBeenCalledOnce();
  });
});
