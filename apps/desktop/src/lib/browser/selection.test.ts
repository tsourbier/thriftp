import { describe, expect, it } from "vitest";
import { applySelection, pruneSelection } from "./selection";
import type { FileEntryView } from "../ipc/types";

function entry(name: string): FileEntryView {
  return {
    name,
    path: `/dir/${name}`,
    entryType: "file",
    size: 1,
    modifiedAt: null,
    permissions: null,
    ownership: null,
    hidden: false,
    capabilities: {
      resume: false,
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

const entries = ["a", "b", "c", "d"].map(entry);

describe("selection helpers", () => {
  it("selects a single item and establishes the anchor", () => {
    const result = applySelection(
      entries,
      { selectedPaths: [], anchorPath: null, activePath: null },
      entries[1],
      { shift: false, toggle: false }
    );
    expect(result.selectedPaths).toEqual(["/dir/b"]);
    expect(result.anchorPath).toBe("/dir/b");
  });

  it("extends a contiguous range with Shift", () => {
    const result = applySelection(
      entries,
      { selectedPaths: ["/dir/a"], anchorPath: "/dir/a", activePath: "/dir/a" },
      entries[2],
      { shift: true, toggle: false }
    );
    expect(result.selectedPaths).toEqual(["/dir/a", "/dir/b", "/dir/c"]);
    expect(result.anchorPath).toBe("/dir/a");
  });

  it("toggles individual items with Cmd/Ctrl", () => {
    const result = applySelection(
      entries,
      {
        selectedPaths: ["/dir/a", "/dir/b"],
        anchorPath: "/dir/b",
        activePath: "/dir/b"
      },
      entries[0],
      { shift: false, toggle: true }
    );
    expect(result.selectedPaths).toEqual(["/dir/b"]);
    expect(result.anchorPath).toBe("/dir/a");
  });

  it("falls back to a plain selection when the anchor disappears", () => {
    const result = applySelection(
      entries,
      {
        selectedPaths: ["/dir/missing"],
        anchorPath: "/dir/missing",
        activePath: "/dir/missing"
      },
      entries[3],
      { shift: true, toggle: false }
    );
    expect(result.selectedPaths).toEqual(["/dir/d"]);
  });

  it("prunes selections that no longer exist after refresh", () => {
    const result = pruneSelection(entries.slice(0, 2), {
      selectedPaths: ["/dir/a", "/dir/d"],
      anchorPath: "/dir/d",
      activePath: "/dir/d"
    });
    expect(result.selectedPaths).toEqual(["/dir/a"]);
    expect(result.anchorPath).toBeNull();
    expect(result.activePath).toBe("/dir/a");
  });
});
