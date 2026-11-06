import { cleanup, fireEvent, render, screen } from "@testing-library/svelte";
import { afterEach, describe, expect, it, vi } from "vitest";

import FavoritesList from "./FavoritesList.svelte";
import SaveFavoriteDialog from "./SaveFavoriteDialog.svelte";
import { focusTrap } from "../../lib/accessibility/focus";
import type { ServerFavoriteView } from "../../lib/ipc/types";

afterEach(() => {
  cleanup();
  document.body.innerHTML = "";
});

function favorite(
  id: string,
  name: string,
  overrides: Partial<ServerFavoriteView> = {}
): ServerFavoriteView {
  return {
    id,
    connectionProfileId: `profile-${id}`,
    displayName: name,
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

describe("server FavoritesList", () => {
  it("shows an empty state with a create action", async () => {
    const oncreate = vi.fn();
    render(FavoritesList, {
      props: {
        favorites: [],
        onconnect: vi.fn(),
        onedit: vi.fn(),
        ondelete: vi.fn(),
        onpin: vi.fn(),
        oncreate
      }
    });
    expect(screen.getByText(/No saved servers/)).toBeTruthy();
    await fireEvent.click(screen.getByRole("button", { name: "Create favorite" }));
    expect(oncreate).toHaveBeenCalledOnce();
  });

  it("renders pinned sections, groups, active state, and sort controls", async () => {
    const onsort = vi.fn();
    render(FavoritesList, {
      props: {
        favorites: [
          favorite("1", "Alpha", { groupLabel: "Production" }),
          favorite("2", "Beta", { pinned: true })
        ],
        preferences: { sortKey: "name", pinnedFirst: true },
        activeFavoriteId: "1",
        connectionStates: { "1": { state: "connected" } },
        onconnect: vi.fn(),
        onedit: vi.fn(),
        ondelete: vi.fn(),
        onpin: vi.fn(),
        onsort
      }
    });

    expect(screen.getByRole("heading", { name: "Pinned" })).toBeTruthy();
    expect(screen.getByRole("heading", { name: "Favorites" })).toBeTruthy();
    expect(screen.getByText("Group: Production")).toBeTruthy();
    expect(screen.getAllByText(/SFTP · example\.test:22/).length).toBeGreaterThan(0);
    expect(screen.getAllByText("connected").length).toBeGreaterThan(0);
    expect(
      screen
        .getByRole("button", { name: "Connect to Alpha" })
        .getAttribute("aria-current")
    ).toBe("true");

    await fireEvent.change(screen.getByLabelText("Sort favorites"), {
      target: { value: "host" }
    });
    expect(onsort).toHaveBeenCalledWith("host");
  });

  it("connects, pins, and edits a favorite", async () => {
    const favorites = [favorite("1", "Alpha"), favorite("2", "Beta")];
    const onconnect = vi.fn();
    const onedit = vi.fn();
    const ondelete = vi.fn();
    const onpin = vi.fn();
    render(FavoritesList, {
      props: { favorites, onconnect, onedit, ondelete, onpin }
    });

    await fireEvent.click(screen.getByRole("button", { name: "Connect to Alpha" }));
    expect(onconnect).toHaveBeenCalledWith(favorites[0]);

    await fireEvent.click(screen.getByRole("button", { name: "Pin Alpha" }));
    expect(onpin).toHaveBeenCalledWith(favorites[0], true);

    await fireEvent.click(screen.getAllByText("Edit")[0]);
    expect(onedit).toHaveBeenCalledWith(favorites[0]);
  });

  it("confirms deletion explicitly", async () => {
    const ondelete = vi.fn();
    render(FavoritesList, {
      props: {
        favorites: [favorite("1", "Alpha")],
        onconnect: vi.fn(),
        onedit: vi.fn(),
        ondelete,
        onpin: vi.fn()
      }
    });

    await fireEvent.click(screen.getByRole("button", { name: "Delete" }));
    expect(ondelete).not.toHaveBeenCalled();
    expect(
      screen.getByRole("alertdialog", { name: "Confirm delete Alpha" })
    ).toBeTruthy();

    await fireEvent.click(screen.getByRole("button", { name: "Confirm delete" }));
    expect(ondelete).toHaveBeenCalledWith(expect.objectContaining({ id: "1" }));
  });

  it("renames a group and assigns a favorite", async () => {
    const ongroup = vi.fn();
    const onrenamegroup = vi.fn();
    const onungroup = vi.fn();
    render(FavoritesList, {
      props: {
        favorites: [favorite("1", "Alpha", { groupLabel: "Prod" })],
        onconnect: vi.fn(),
        onedit: vi.fn(),
        ondelete: vi.fn(),
        onpin: vi.fn(),
        ongroup,
        onrenamegroup,
        onungroup
      }
    });

    await fireEvent.change(screen.getByLabelText("Group to rename"), {
      target: { value: "Prod" }
    });
    await fireEvent.input(screen.getByLabelText("New group label"), {
      target: { value: "Production" }
    });
    await fireEvent.click(screen.getByRole("button", { name: "Rename" }));
    expect(onrenamegroup).toHaveBeenCalledWith("Prod", "Production");

    await fireEvent.click(screen.getByRole("button", { name: "Group Alpha" }));
    await fireEvent.input(screen.getByLabelText("Group"), {
      target: { value: "Staging" }
    });
    await fireEvent.click(screen.getByRole("button", { name: "Assign" }));
    expect(ongroup).toHaveBeenCalledWith(expect.objectContaining({ id: "1" }), "Staging");
  });

  it("supports a keyboard-only create, navigation, pin, edit, and delete flow", async () => {
    const onsave = vi.fn();
    render(SaveFavoriteDialog, {
      props: { onsave, oncancel: vi.fn() }
    });
    await fireEvent.input(screen.getByLabelText("Name"), {
      target: { value: "Keyboard" }
    });
    await fireEvent.input(screen.getByLabelText("Host"), {
      target: { value: "keyboard.test" }
    });
    await fireEvent.input(screen.getByLabelText("Username"), {
      target: { value: "tester" }
    });
    const save = screen.getByRole("button", { name: "Save" });
    save.focus();
    expect(document.activeElement).toBe(save);
    await fireEvent.click(save);
    expect(onsave).toHaveBeenCalledOnce();

    cleanup();

    const favorites = [favorite("1", "Alpha")];
    const onconnect = vi.fn();
    const onpin = vi.fn();
    const onedit = vi.fn();
    const ondelete = vi.fn();
    render(FavoritesList, {
      props: { favorites, onconnect, onpin, onedit, ondelete }
    });

    const connect = screen.getByRole("button", { name: "Connect to Alpha" });
    connect.focus();
    expect(document.activeElement).toBe(connect);
    await fireEvent.click(connect);

    const pin = screen.getByRole("button", { name: "Pin Alpha" });
    pin.focus();
    await fireEvent.click(pin);
    expect(onpin).toHaveBeenCalled();

    const edit = screen.getByRole("button", { name: "Edit" });
    edit.focus();
    await fireEvent.click(edit);
    expect(onedit).toHaveBeenCalled();

    const remove = screen.getByRole("button", { name: "Delete" });
    remove.focus();
    await fireEvent.click(remove);
    const confirm = screen.getByRole("button", { name: "Confirm delete" });
    confirm.focus();
    await fireEvent.click(confirm);
    expect(ondelete).toHaveBeenCalled();
  });

  it("traps focus in a dialog and returns it to the trigger", () => {
    const trigger = document.createElement("button");
    trigger.textContent = "Open";
    document.body.appendChild(trigger);
    trigger.focus();

    const dialog = document.createElement("div");
    dialog.innerHTML =
      '<button type="button">First</button><button type="button">Last</button>';
    document.body.appendChild(dialog);

    const action = focusTrap(dialog);
    const first = screen.getByRole("button", { name: "First" });
    expect(document.activeElement).toBe(first);

    action.destroy();
    expect(document.activeElement).toBe(trigger);
  });
});
