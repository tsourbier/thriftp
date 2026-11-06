import { describe, expect, it } from "vitest";
import { visibleEntries } from "./view";
import type { FileEntryView } from "../ipc/types";

function entry(name: string, size: number, modifiedAt: string | null): FileEntryView {
  return {
    name,
    path: `/dir/${name}`,
    entryType: "file",
    size,
    modifiedAt,
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

const entries = [
  entry("beta", 30, "2026-01-03T00:00:00Z"),
  entry("Alpha", 10, "2026-01-01T00:00:00Z"),
  entry("gamma", 20, "2026-01-02T00:00:00Z")
];

describe("direction-aware sorting", () => {
  it("sorts by name in both directions", () => {
    expect(
      visibleEntries(entries, {
        sortKey: "name",
        sortDirection: "asc",
        showHidden: false,
        filter: ""
      }).map((item) => item.name)
    ).toEqual(["Alpha", "beta", "gamma"]);
    expect(
      visibleEntries(entries, {
        sortKey: "name",
        sortDirection: "desc",
        showHidden: false,
        filter: ""
      }).map((item) => item.name)
    ).toEqual(["gamma", "beta", "Alpha"]);
  });

  it("sorts by size and modified date", () => {
    expect(
      visibleEntries(entries, {
        sortKey: "size",
        sortDirection: "asc",
        showHidden: false,
        filter: ""
      }).map((item) => item.name)
    ).toEqual(["Alpha", "gamma", "beta"]);
    expect(
      visibleEntries(entries, {
        sortKey: "date",
        sortDirection: "desc",
        showHidden: false,
        filter: ""
      }).map((item) => item.name)
    ).toEqual(["beta", "gamma", "Alpha"]);
  });

  it("sorts ten thousand entries without blocking the call", () => {
    const large = Array.from({ length: 10_000 }, (_, index) =>
      entry(`file-${index}`, index, null)
    );
    const started = performance.now();
    const sorted = visibleEntries(large, {
      sortKey: "size",
      sortDirection: "desc",
      showHidden: false,
      filter: ""
    });
    expect(performance.now() - started).toBeLessThan(500);
    expect(sorted[0].size).toBe(9999);
  });
});
