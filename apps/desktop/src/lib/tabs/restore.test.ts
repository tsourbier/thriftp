import { afterEach, describe, expect, it, vi } from "vitest";

vi.mock("../ipc/client", () => ({
  ipc: {
    tabsRestore: vi.fn()
  }
}));

import { ipc } from "../ipc/client";
import { newTab, restoreTabs } from "./restore";
import type { BrowsingSession } from "../ipc/types";

afterEach(() => {
  vi.resetAllMocks();
});

function session(overrides: Partial<BrowsingSession> = {}): BrowsingSession {
  return {
    id: "tab-1",
    connectionProfileId: "profile-1",
    currentPath: "/srv/www",
    localPath: "/Users/me",
    history: ["/srv/www"],
    historyIndex: 0,
    viewSettings: {
      sortKey: "name",
      sortDirection: "asc",
      showHidden: false,
      filter: null
    },
    selectedItems: [],
    localSelectedItems: [],
    createdAt: new Date().toISOString(),
    restored: false,
    ...overrides
  };
}

describe("tab restore", () => {
  it("defaults new tabs to ascending name sorting", () => {
    const tab = newTab("profile-1");
    expect(tab.viewSettings.sortKey).toBe("name");
    expect(tab.viewSettings.sortDirection).toBe("asc");
  });

  it("uses the supplied sort defaults for new tabs", () => {
    const tab = newTab("profile-1", ".", {
      sortKey: "date",
      sortDirection: "desc"
    });
    expect(tab.viewSettings.sortKey).toBe("date");
    expect(tab.viewSettings.sortDirection).toBe("desc");
  });

  it("preserves persisted sort direction when tabs are restored", async () => {
    vi.mocked(ipc.tabsRestore).mockResolvedValue([
      session({
        viewSettings: {
          sortKey: "size",
          sortDirection: "desc",
          showHidden: false,
          filter: null
        }
      })
    ]);

    const restored = await restoreTabs(true, ["profile-1"]);
    expect(restored[0].viewSettings.sortKey).toBe("size");
    expect(restored[0].viewSettings.sortDirection).toBe("desc");
    expect(restored[0].restored).toBe(true);
    expect(restored[0].disconnected).toBe(false);
  });

  it("returns no tabs when restoration is disabled", async () => {
    await expect(restoreTabs(false)).resolves.toEqual([]);
    expect(ipc.tabsRestore).not.toHaveBeenCalled();
  });
});
