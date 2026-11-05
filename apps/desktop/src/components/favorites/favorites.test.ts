import { cleanup, fireEvent, render, screen } from "@testing-library/svelte";
import { afterEach, describe, expect, it, vi } from "vitest";

import TabBar from "../tabs/TabBar.svelte";
import FavoritesSidebar from "./FavoritesSidebar.svelte";
import SaveDirectoryFavoriteDialog from "./SaveDirectoryFavoriteDialog.svelte";
import type { BrowsingSession, DirectoryFavorite } from "../../lib/ipc/types";

afterEach(cleanup);

function tab(
  id: string,
  path: string,
  profileId: string | null = "profile-1"
): BrowsingSession {
  return {
    id,
    connectionProfileId: profileId,
    currentPath: path,
    history: [path],
    historyIndex: 0,
    viewSettings: { sortKey: "name", showHidden: false, filter: null },
    selectedItems: [],
    createdAt: new Date().toISOString(),
    restored: false
  };
}

function favorite(id: string, label: string, path: string): DirectoryFavorite {
  return {
    id,
    connectionProfileId: "profile-1",
    path,
    label,
    sortOrder: 0,
    createdAt: new Date().toISOString(),
    updatedAt: new Date().toISOString()
  };
}

describe("TabBar", () => {
  it("selects, closes, and adds tabs", async () => {
    const onselect = vi.fn();
    const onclose = vi.fn();
    const onadd = vi.fn();
    render(TabBar, {
      props: {
        tabs: [tab("a", "/one"), tab("b", "/two")],
        activeId: "a",
        labelFor: (session: BrowsingSession) => session.currentPath,
        onselect,
        onclose,
        onadd
      }
    });

    await fireEvent.click(screen.getByRole("tab", { name: "/two" }));
    expect(onselect).toHaveBeenCalledOnce();

    await fireEvent.click(screen.getByRole("button", { name: "Close /one" }));
    expect(onclose).toHaveBeenCalledOnce();

    await fireEvent.click(screen.getByRole("button", { name: "New tab" }));
    expect(onadd).toHaveBeenCalledOnce();
  });

  it("marks restored tabs as disconnected", () => {
    render(TabBar, {
      props: {
        tabs: [{ ...tab("a", "/one", null), restored: true }],
        activeId: "a",
        labelFor: (session: BrowsingSession) => session.currentPath,
        onselect: vi.fn(),
        onclose: vi.fn(),
        onadd: vi.fn()
      }
    });
    expect(screen.getByText("disconnected")).toBeTruthy();
  });
});

describe("FavoritesSidebar", () => {
  const favorites = [favorite("1", "Home", "/home"), favorite("2", "Logs", "/var/log")];

  it("opens and deletes favorites", async () => {
    const onopen = vi.fn();
    const ondelete = vi.fn();
    render(FavoritesSidebar, {
      props: {
        favorites,
        canSave: true,
        onopen,
        onsave: vi.fn(),
        ondelete
      }
    });

    await fireEvent.click(screen.getByText("Home"));
    expect(onopen).toHaveBeenCalledWith(favorites[0]);

    await fireEvent.click(screen.getByRole("button", { name: "Delete Logs" }));
    expect(ondelete).toHaveBeenCalledWith(favorites[1]);
  });

  it("disables save when no directory is available", () => {
    render(FavoritesSidebar, {
      props: {
        favorites: [],
        canSave: false,
        onopen: vi.fn(),
        onsave: vi.fn(),
        ondelete: vi.fn()
      }
    });
    const save = screen.getByRole("button", {
      name: "Save current"
    }) as HTMLButtonElement;
    expect(save.disabled).toBe(true);
    expect(screen.getByText(/No favorite directories/)).toBeTruthy();
  });
});

describe("SaveDirectoryFavoriteDialog", () => {
  it("submits the label and path", async () => {
    const onsave = vi.fn();
    render(SaveDirectoryFavoriteDialog, {
      props: {
        path: "/home/user/docs",
        onsave,
        oncancel: vi.fn()
      }
    });
    const label = screen.getByLabelText("Label") as HTMLInputElement;
    expect(label.value).toBe("docs");

    await fireEvent.input(label, { target: { value: "Documents" } });
    await fireEvent.click(screen.getByRole("button", { name: "Save" }));
    expect(onsave).toHaveBeenCalledWith("Documents", "/home/user/docs");
  });
});
