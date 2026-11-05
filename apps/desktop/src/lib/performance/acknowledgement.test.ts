import { cleanup, fireEvent, render, screen } from "@testing-library/svelte";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import QueueManager from "../../components/queue/QueueManager.svelte";
import DirectoryList from "../../components/browser/DirectoryList.svelte";
import type { FileEntryView, QueueView, TransferView } from "../ipc/types";

afterEach(() => {
  cleanup();
  vi.useRealTimers();
});

beforeEach(() => {
  vi.useFakeTimers();
});

function transfer(): TransferView {
  return {
    id: "t1",
    direction: "download",
    connectionProfileId: "p1",
    sourcePath: "/remote/a",
    destinationPath: "/local/a",
    totalBytes: 1,
    transferredBytes: 0,
    state: "pending",
    priority: "normal",
    order: 0,
    speedBytesPerSecond: null,
    estimatedRemainingSeconds: null,
    resumable: false,
    partialDataId: null,
    error: null,
    createdAt: new Date().toISOString(),
    updatedAt: new Date().toISOString(),
    startedAt: null,
    completedAt: null
  };
}

function queue(): QueueView {
  return {
    paused: false,
    concurrencyLimit: 4,
    bandwidthLimitBytesPerSecond: null,
    orderedTransferIds: ["t1"],
    activeTransferIds: []
  };
}

function entry(): FileEntryView {
  return {
    name: "file.txt",
    path: "/tmp/file.txt",
    entryType: "file",
    size: 1,
    modifiedAt: null,
    permissions: null,
    ownership: null,
    hidden: false,
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

describe("acknowledgement latency", () => {
  it("acknowledges queue actions without waiting for a timer", async () => {
    const onpausequeue = vi.fn();
    render(QueueManager, {
      props: {
        transfers: [transfer()],
        queue: queue(),
        onpausequeue,
        onresumequeue: vi.fn(),
        onsetconcurrency: vi.fn(),
        onsetbandwidth: vi.fn(),
        onreorder: vi.fn(),
        onsetpriority: vi.fn(),
        onpausetransfer: vi.fn(),
        onresumetransfer: vi.fn(),
        oncanceltransfer: vi.fn(),
        oncancelall: vi.fn()
      }
    });
    await fireEvent.click(screen.getByRole("button", { name: "Pause queue" }));
    expect(onpausequeue).toHaveBeenCalledOnce();
  });

  it("acknowledges browser selection immediately", async () => {
    const onselect = vi.fn();
    render(DirectoryList, {
      props: {
        entries: [entry()],
        onselect,
        onopen: vi.fn()
      }
    });
    await fireEvent.click(screen.getByText("file.txt"));
    expect(onselect).toHaveBeenCalledOnce();
  });
});
