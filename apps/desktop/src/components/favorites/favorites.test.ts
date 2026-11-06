import { cleanup, fireEvent, render, screen } from "@testing-library/svelte";
import { afterEach, describe, expect, it, vi } from "vitest";

import TabBar from "../tabs/TabBar.svelte";
import SaveDirectoryFavoriteDialog from "./SaveDirectoryFavoriteDialog.svelte";
import type { BrowsingSession } from "../../lib/ipc/types";

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
    viewSettings: {
      sortKey: "name",
      sortDirection: "asc",
      showHidden: false,
      filter: null
    },
    selectedItems: [],
    createdAt: new Date().toISOString(),
    restored: false
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
