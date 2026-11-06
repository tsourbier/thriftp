import { cleanup, render, screen } from "@testing-library/svelte";
import { afterEach, describe, expect, it, vi } from "vitest";

import DirectoryList from "./DirectoryList.svelte";
import InternalPreview from "../previews/InternalPreview.svelte";
import { filterEntries, sortEntries, visibleEntries } from "../../lib/browser/view";
import { handleBrowserKey } from "../../lib/keyboard/browser";
import type { FileEntryView } from "../../lib/ipc/types";

vi.mock("../../lib/ipc/client", () => ({
  ipc: {
    previewOpen: vi.fn(),
    previewRead: vi.fn()
  }
}));

import { ipc } from "../../lib/ipc/client";

afterEach(cleanup);

function entry(
  name: string,
  size: number | null,
  entryType: FileEntryView["entryType"] = "file",
  hidden = name.startsWith("."),
  modifiedAt: string | null = null
): FileEntryView {
  return {
    name,
    path: `/tmp/${name}`,
    entryType,
    size,
    modifiedAt,
    permissions: null,
    ownership: null,
    hidden,
    symlinkTarget: null,
    capabilities: {
      resume: true,
      permissions: false,
      ownership: false,
      hiddenFiles: true,
      symbolicLinks: true,
      serverSideRename: true,
      serverSideDelete: true,
      maxPathLength: null
    }
  };
}

describe("browser view helpers", () => {
  const entries = [
    entry("beta.txt", 20, "file", false, "2026-01-02T00:00:00Z"),
    entry("Alpha", 5, "directory", false, "2026-01-03T00:00:00Z"),
    entry(".hidden", 1, "file", true, "2026-01-01T00:00:00Z")
  ];

  it("hides hidden files unless requested", () => {
    expect(
      filterEntries(entries, { showHidden: false, filter: "" }).map((e) => e.name)
    ).toEqual(["beta.txt", "Alpha"]);
    expect(filterEntries(entries, { showHidden: true, filter: "" })).toHaveLength(3);
  });

  it("filters case-insensitively", () => {
    expect(
      filterEntries(entries, { showHidden: true, filter: "ALP" }).map((e) => e.name)
    ).toEqual(["Alpha"]);
  });

  it("sorts by name, size, date, and type", () => {
    expect(sortEntries(entries, "name").map((e) => e.name)[0]).toBe(".hidden");
    expect(sortEntries(entries, "size").map((e) => e.name)).toEqual([
      ".hidden",
      "Alpha",
      "beta.txt"
    ]);
    expect(sortEntries(entries, "date").map((e) => e.name)).toEqual([
      ".hidden",
      "beta.txt",
      "Alpha"
    ]);
    expect(sortEntries(entries, "type")[0].entryType).toBe("directory");
  });

  it("combines filtering and sorting", () => {
    expect(
      visibleEntries(entries, {
        sortKey: "name",
        sortDirection: "asc",
        showHidden: false,
        filter: ""
      }).map((e) => e.name)
    ).toEqual(["Alpha", "beta.txt"]);
  });
});

describe("DirectoryList", () => {
  const rows = [entry("Alpha", 5, "directory"), entry("beta.txt", 20)];

  it("renders entries and the selected row", () => {
    render(DirectoryList, {
      props: {
        entries: rows,
        selectedPath: "/tmp/beta.txt",
        onselect: () => undefined,
        onopen: () => undefined
      }
    });
    expect(screen.getByText("Alpha")).toBeTruthy();
    expect(screen.getByText("beta.txt")).toBeTruthy();
    expect(screen.getAllByRole("option")[1].getAttribute("aria-selected")).toBe("true");
  });

  it("shows a loading state before the first batch", () => {
    render(DirectoryList, {
      props: {
        entries: [],
        loading: true,
        listingState: "pending",
        onselect: () => undefined,
        onopen: () => undefined
      }
    });
    expect(screen.getByRole("status").textContent).toContain("Loading");
  });

  it("shows an empty state only after completion", () => {
    render(DirectoryList, {
      props: {
        entries: [],
        loading: false,
        listingState: "complete",
        onselect: () => undefined,
        onopen: () => undefined
      }
    });
    expect(screen.getByRole("status").textContent).toContain("empty");
  });
});

describe("browser keyboard navigation", () => {
  const entries = [entry("a.txt", 1), entry("b.txt", 2)];
  const base = {
    entries,
    selectedPath: "/tmp/a.txt",
    onselect: vi.fn(),
    onopen: vi.fn(),
    onup: vi.fn(),
    onrefresh: vi.fn(),
    ontoggleHidden: vi.fn(),
    onsort: vi.fn(),
    ondelete: vi.fn()
  };

  it("moves selection and opens entries", () => {
    expect(
      handleBrowserKey(new KeyboardEvent("keydown", { key: "ArrowDown" }), base)
    ).toBe(true);
    expect(base.onselect).toHaveBeenCalledWith(entries[1], {
      shift: false,
      toggle: false
    });

    expect(handleBrowserKey(new KeyboardEvent("keydown", { key: "Enter" }), base)).toBe(
      true
    );
    expect(base.onopen).toHaveBeenCalledWith(entries[0]);
  });

  it("reports Shift and Cmd/Ctrl modifiers for keyboard selection", () => {
    const actions = { ...base, onselect: vi.fn() };

    handleBrowserKey(
      new KeyboardEvent("keydown", { key: "ArrowDown", shiftKey: true }),
      actions
    );
    expect(actions.onselect).toHaveBeenLastCalledWith(entries[1], {
      shift: true,
      toggle: false
    });

    handleBrowserKey(
      new KeyboardEvent("keydown", { key: "ArrowUp", ctrlKey: true }),
      actions
    );
    expect(actions.onselect).toHaveBeenLastCalledWith(entries[0], {
      shift: false,
      toggle: true
    });
  });

  it("handles navigation, refresh, hidden toggle, and delete", () => {
    expect(
      handleBrowserKey(new KeyboardEvent("keydown", { key: "Backspace" }), base)
    ).toBe(true);
    expect(base.onup).toHaveBeenCalled();

    expect(handleBrowserKey(new KeyboardEvent("keydown", { key: "F5" }), base)).toBe(
      true
    );
    expect(base.onrefresh).toHaveBeenCalled();

    expect(
      handleBrowserKey(new KeyboardEvent("keydown", { key: "h", ctrlKey: true }), base)
    ).toBe(true);
    expect(base.ontoggleHidden).toHaveBeenCalled();

    expect(handleBrowserKey(new KeyboardEvent("keydown", { key: "Delete" }), base)).toBe(
      true
    );
    expect(base.ondelete).toHaveBeenCalledWith(entries[0]);
  });
});

describe("preview fallback", () => {
  it("renders text previews for local files", async () => {
    vi.mocked(ipc.previewOpen).mockResolvedValue({
      previewId: "preview-1",
      mode: "internal"
    });
    vi.mocked(ipc.previewRead).mockResolvedValue({
      kind: "text",
      content: "hello preview",
      truncated: false
    });

    render(InternalPreview, {
      props: {
        source: "local",
        path: "/tmp/notes.txt",
        name: "notes.txt",
        onclose: () => undefined
      }
    });

    expect(await screen.findByText("hello preview")).toBeTruthy();
  });

  it("shows metadata when content cannot be rendered", async () => {
    vi.mocked(ipc.previewOpen).mockResolvedValue({
      previewId: "preview-2",
      mode: "metadata",
      reason: "the file is too large to preview inside the application"
    });

    render(InternalPreview, {
      props: {
        source: "local",
        path: "/tmp/archive.bin",
        name: "archive.bin",
        onclose: () => undefined
      }
    });

    expect(await screen.findByText(/too large/i)).toBeTruthy();
  });
});
