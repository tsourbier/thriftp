import { cleanup, fireEvent, render, screen } from "@testing-library/svelte";
import { afterEach, describe, expect, it, vi } from "vitest";

import QueueManager from "./QueueManager.svelte";
import type { QueueView, TransferView } from "../../lib/ipc/types";

afterEach(cleanup);

function transfer(id: string, state: TransferView["state"] = "pending"): TransferView {
  return {
    id,
    direction: "download",
    connectionProfileId: "profile-1",
    sourcePath: `/remote/${id}.bin`,
    destinationPath: `/local/${id}.bin`,
    totalBytes: 100,
    transferredBytes: state === "completed" ? 100 : 20,
    state,
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

function queue(overrides: Partial<QueueView> = {}): QueueView {
  return {
    paused: false,
    concurrencyLimit: 4,
    bandwidthLimitBytesPerSecond: null,
    orderedTransferIds: ["a", "b"],
    activeTransferIds: [],
    ...overrides
  };
}

describe("QueueManager", () => {
  it("shows queue state and transfer rows", () => {
    render(QueueManager, {
      props: {
        transfers: [transfer("a", "active"), transfer("b")],
        queue: queue({ activeTransferIds: ["a"] }),
        onpausequeue: vi.fn(),
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
    expect(screen.getByText("Queue")).toBeTruthy();
    expect(screen.getByText(/1 active/)).toBeTruthy();
    expect(screen.getByText("a.bin")).toBeTruthy();
    expect(screen.getByText("b.bin")).toBeTruthy();
  });

  it("hides completed transfers from the queue", () => {
    render(QueueManager, {
      props: {
        transfers: [transfer("done", "completed"), transfer("pending", "pending")],
        queue: queue(),
        onpausequeue: vi.fn(),
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

    expect(screen.queryByText(/done\.bin/)).toBeNull();
    expect(screen.getByText(/pending\.bin/)).toBeTruthy();
  });

  it("collapses and expands the queue panel", async () => {
    render(QueueManager, {
      props: {
        transfers: [transfer("a")],
        queue: queue(),
        onpausequeue: vi.fn(),
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

    expect(screen.getByLabelText("Concurrent transfers")).toBeTruthy();
    await fireEvent.click(screen.getByRole("button", { name: "Collapse" }));
    expect(screen.queryByLabelText("Concurrent transfers")).toBeNull();
    expect(screen.getByText("Queue")).toBeTruthy();

    await fireEvent.click(screen.getByRole("button", { name: "Expand" }));
    expect(screen.getByLabelText("Concurrent transfers")).toBeTruthy();
  });

  it("pauses and resumes the queue", async () => {
    const onpausequeue = vi.fn();
    const onresumequeue = vi.fn();
    const { rerender } = render(QueueManager, {
      props: {
        transfers: [transfer("a")],
        queue: queue(),
        onpausequeue,
        onresumequeue,
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

    await rerender({ queue: queue({ paused: true }) });
    await fireEvent.click(screen.getByRole("button", { name: "Resume queue" }));
    expect(onresumequeue).toHaveBeenCalledOnce();
  });

  it("updates concurrency and cancels all with confirmation", async () => {
    const onsetconcurrency = vi.fn();
    const oncancelall = vi.fn();
    render(QueueManager, {
      props: {
        transfers: [transfer("a")],
        queue: queue(),
        onpausequeue: vi.fn(),
        onresumequeue: vi.fn(),
        onsetconcurrency,
        onsetbandwidth: vi.fn(),
        onreorder: vi.fn(),
        onsetpriority: vi.fn(),
        onpausetransfer: vi.fn(),
        onresumetransfer: vi.fn(),
        oncanceltransfer: vi.fn(),
        oncancelall
      }
    });
    const concurrency = screen.getByLabelText("Concurrent transfers");
    await fireEvent.input(concurrency, { target: { value: "8" } });
    await fireEvent.change(concurrency);
    expect(onsetconcurrency).toHaveBeenCalledWith(8);

    await fireEvent.click(screen.getByRole("button", { name: "Cancel all" }));
    expect(oncancelall).toHaveBeenCalledOnce();
  });

  it("reorders transfers and changes priority", async () => {
    const onreorder = vi.fn();
    const onsetpriority = vi.fn();
    render(QueueManager, {
      props: {
        transfers: [transfer("a"), transfer("b")],
        queue: queue(),
        onpausequeue: vi.fn(),
        onresumequeue: vi.fn(),
        onsetconcurrency: vi.fn(),
        onsetbandwidth: vi.fn(),
        onreorder,
        onsetpriority,
        onpausetransfer: vi.fn(),
        onresumetransfer: vi.fn(),
        oncanceltransfer: vi.fn(),
        oncancelall: vi.fn()
      }
    });
    const moveDown = screen.getAllByRole("button", { name: "Move down" })[0];
    await fireEvent.click(moveDown);
    expect(onreorder).toHaveBeenCalledWith(["b", "a"]);

    const priority = screen.getAllByLabelText("Priority")[0];
    await fireEvent.change(priority, { target: { value: "high" } });
    expect(onsetpriority).toHaveBeenCalledWith("a", "high");
  });
});
