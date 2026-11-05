import { cleanup, fireEvent, render, screen } from "@testing-library/svelte";
import { afterEach, describe, expect, it, vi } from "vitest";

import RecoveryPanel from "./RecoveryPanel.svelte";
import type { TransferView } from "../../lib/ipc/types";

afterEach(cleanup);

function transfer(state: TransferView["state"], transferred = 50): TransferView {
  return {
    id: `transfer-${state}`,
    direction: "download",
    connectionProfileId: "profile-1",
    sourcePath: "/remote/file.bin",
    destinationPath: "/local/file.bin",
    totalBytes: 100,
    transferredBytes: transferred,
    state,
    priority: "normal",
    order: 0,
    speedBytesPerSecond: null,
    estimatedRemainingSeconds: null,
    resumable: true,
    partialDataId: null,
    error: state === "failed" ? "network reset" : null,
    createdAt: new Date().toISOString(),
    updatedAt: new Date().toISOString(),
    startedAt: null,
    completedAt: null
  };
}

const callbacks = {
  onresume: vi.fn(),
  onrestart: vi.fn(),
  oncancel: vi.fn(),
  ondiscard: vi.fn()
};

describe("RecoveryPanel", () => {
  it("lists interrupted and failed transfers with recovery actions", () => {
    render(RecoveryPanel, {
      props: {
        transfers: [transfer("interrupted"), transfer("failed"), transfer("completed")],
        ...callbacks
      }
    });
    expect(screen.getByText(/2 need attention/)).toBeTruthy();
    expect(screen.getByText(/network reset/)).toBeTruthy();
    expect(screen.getAllByRole("button", { name: "Resume" })).toHaveLength(2);
    expect(screen.getAllByRole("button", { name: "Restart" })).toHaveLength(2);
    expect(screen.getAllByRole("button", { name: "Discard partial" })).toHaveLength(2);
  });

  it("dispatches resume, restart, discard, and cancel", async () => {
    render(RecoveryPanel, {
      props: { transfers: [transfer("interrupted")], ...callbacks }
    });
    await fireEvent.click(screen.getByRole("button", { name: "Resume" }));
    expect(callbacks.onresume).toHaveBeenCalledOnce();

    await fireEvent.click(screen.getByRole("button", { name: "Restart" }));
    expect(callbacks.onrestart).toHaveBeenCalledOnce();

    await fireEvent.click(screen.getByRole("button", { name: "Discard partial" }));
    expect(callbacks.ondiscard).toHaveBeenCalledOnce();

    await fireEvent.click(screen.getByRole("button", { name: "Cancel" }));
    expect(callbacks.oncancel).toHaveBeenCalledOnce();
  });

  it("renders nothing when there is no interrupted work", () => {
    const { container } = render(RecoveryPanel, {
      props: { transfers: [transfer("completed")], ...callbacks }
    });
    expect(container.textContent).not.toContain("Recover interrupted transfers");
  });
});
