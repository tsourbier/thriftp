import { cleanup, fireEvent, render, screen } from "@testing-library/svelte";
import { afterEach, describe, expect, it, vi } from "vitest";

vi.mock("../../lib/ipc/client", () => ({
  ipc: {
    inspectorGet: vi.fn(),
    chmod: vi.fn()
  }
}));

import { ipc } from "../../lib/ipc/client";
import Inspector from "./Inspector.svelte";
import type { FileMetadata } from "../../lib/ipc/types";

afterEach(cleanup);

function metadata(permissionsSupported = true): FileMetadata {
  return {
    entry: {
      name: "notes.txt",
      path: "/home/user/notes.txt",
      entryType: "file",
      size: 128,
      modifiedAt: "2026-01-01T00:00:00Z",
      permissions: "644",
      ownership: { owner: "user", group: "staff" },
      hidden: false,
      symlinkTarget: null,
      capabilities: {
        resume: true,
        permissions: permissionsSupported,
        ownership: true,
        hiddenFiles: true,
        symbolicLinks: true,
        serverSideRename: true,
        serverSideDelete: true,
        maxPathLength: null
      }
    },
    permissionsSupported,
    ownershipSupported: true
  };
}

describe("Inspector", () => {
  it("renders metadata and ownership", async () => {
    vi.mocked(ipc.inspectorGet).mockResolvedValue(metadata());
    render(Inspector, {
      props: {
        source: "remote",
        connectionId: "connection-1",
        path: "/home/user/notes.txt",
        onclose: vi.fn()
      }
    });

    expect(await screen.findByText("notes.txt")).toBeTruthy();
    expect(screen.getByText("/home/user/notes.txt")).toBeTruthy();
    expect(
      screen.getByText(
        (_, element) =>
          element?.tagName === "DD" &&
          element.textContent?.includes("user") === true &&
          element.textContent?.includes("staff") === true
      )
    ).toBeTruthy();
    expect(screen.getByLabelText("Octal mode")).toBeTruthy();
  });

  it("applies an octal permission change", async () => {
    vi.mocked(ipc.inspectorGet).mockResolvedValue(metadata());
    vi.mocked(ipc.chmod).mockResolvedValue(metadata().entry);
    const onchanged = vi.fn();
    render(Inspector, {
      props: {
        source: "remote",
        connectionId: "connection-1",
        path: "/home/user/notes.txt",
        onclose: vi.fn(),
        onchanged
      }
    });

    const input = await screen.findByLabelText("Octal mode");
    await fireEvent.input(input, { target: { value: "600" } });
    await fireEvent.click(screen.getByRole("button", { name: "Apply" }));
    expect(ipc.chmod).toHaveBeenCalledWith("connection-1", "/home/user/notes.txt", 0o600);
    await vi.waitFor(() => expect(onchanged).toHaveBeenCalledOnce());
  });

  it("explains unsupported permission changes", async () => {
    vi.mocked(ipc.inspectorGet).mockResolvedValue(metadata(false));
    render(Inspector, {
      props: {
        source: "remote",
        connectionId: "connection-1",
        path: "/home/user/notes.txt",
        onclose: vi.fn()
      }
    });

    expect(await screen.findByText(/does not support changing permissions/)).toBeTruthy();
  });
});
