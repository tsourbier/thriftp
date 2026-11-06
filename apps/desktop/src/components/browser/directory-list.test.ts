import { cleanup, fireEvent, render, screen } from "@testing-library/svelte";
import { afterEach, describe, expect, it, vi } from "vitest";
import DirectoryList from "./DirectoryList.svelte";
import type { FileEntryView } from "../../lib/ipc/types";

const clientHeight = Object.getOwnPropertyDescriptor(Element.prototype, "clientHeight");

afterEach(() => {
  cleanup();
  if (clientHeight) {
    Object.defineProperty(Element.prototype, "clientHeight", clientHeight);
  }
});

function entries(count: number): FileEntryView[] {
  return Array.from({ length: count }, (_, index) => ({
    name: `file-${index}.bin`,
    path: `/local/file-${index}.bin`,
    entryType: "file",
    size: index,
    modifiedAt: null,
    permissions: null,
    ownership: null,
    hidden: false,
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
  }));
}

describe("DirectoryList", () => {
  it("sorts from column headers and exposes the active direction", async () => {
    const onsort = vi.fn();
    render(DirectoryList, {
      props: {
        entries: entries(2),
        listingState: "complete",
        sortKey: "name",
        sortDirection: "asc",
        onsort,
        onselect: vi.fn(),
        onopen: vi.fn()
      }
    });

    expect(
      screen.getByRole("columnheader", { name: /Name/ }).getAttribute("aria-sort")
    ).toBe("ascending");
    await fireEvent.click(screen.getByRole("button", { name: /Size/ }));
    expect(onsort).toHaveBeenCalledWith("size");
    expect(screen.getByRole("button", { name: /Name/ }).tabIndex).toBe(0);
  });

  it("reports Shift and Cmd/Ctrl modifiers to the pane", async () => {
    const onselect = vi.fn();
    render(DirectoryList, {
      props: {
        entries: entries(2),
        listingState: "complete",
        onselect,
        onopen: vi.fn()
      }
    });

    await fireEvent.click(screen.getByText(/file-0\.bin/), { shiftKey: true });
    expect(onselect).toHaveBeenCalledWith(
      expect.objectContaining({ path: "/local/file-0.bin" }),
      { shift: true, toggle: false }
    );
  });

  it("renders as many rows as the measured viewport can show", () => {
    Object.defineProperty(Element.prototype, "clientHeight", {
      configurable: true,
      get: () => 800
    });

    render(DirectoryList, {
      props: {
        entries: entries(100),
        listingState: "complete",
        onselect: vi.fn(),
        onopen: vi.fn()
      }
    });

    expect(screen.getAllByRole("option").length).toBeGreaterThan(21);
  });
});
