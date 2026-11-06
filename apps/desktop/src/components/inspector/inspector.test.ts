import { cleanup, fireEvent, render, screen } from "@testing-library/svelte";
import { afterEach, describe, expect, it, vi } from "vitest";

vi.mock("../../lib/ipc/client", () => ({
  ipc: {
    inspectorGet: vi.fn(),
    chmod: vi.fn(),
    chmodRecursive: vi.fn()
  }
}));

vi.mock("../../lib/dialogs/confirm", () => ({
  confirmAction: vi.fn().mockResolvedValue(true)
}));

import { confirmAction } from "../../lib/dialogs/confirm";
import { ipc } from "../../lib/ipc/client";
import Inspector from "./Inspector.svelte";
import type { FileMetadata } from "../../lib/ipc/types";

afterEach(cleanup);

function metadata(
  permissionsSupported = true,
  entryType: "file" | "directory" = "file",
  editable = permissionsSupported
): FileMetadata {
  return {
    entry: {
      name: entryType === "directory" ? "docs" : "notes.txt",
      path: entryType === "directory" ? "/home/user/docs" : "/home/user/notes.txt",
      entryType,
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
    ownershipSupported: true,
    permissionDetail: {
      mode: 0o644,
      symbolic: "rw-r--r--",
      user: { read: true, write: true, execute: false },
      group: { read: true, write: false, execute: false },
      world: { read: true, write: false, execute: false },
      editable,
      reason: editable ? undefined : "Local permissions are read-only."
    }
  };
}

describe("Inspector", () => {
  it("renders the permission matrix, octal value, and symbolic recap", async () => {
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
    expect(screen.getByText("Octal: 644")).toBeTruthy();
    expect(screen.getByText("rw-r--r--")).toBeTruthy();
    expect(screen.getByLabelText("User Read")).toBeTruthy();
    expect(screen.getByLabelText("Group Execute")).toBeTruthy();
  });

  it("applies the permission mode from the matrix", async () => {
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

    await fireEvent.click(await screen.findByLabelText("User Write"));
    await fireEvent.click(screen.getByRole("button", { name: "Apply" }));
    expect(ipc.chmod).toHaveBeenCalledWith("connection-1", "/home/user/notes.txt", 0o444);
    await vi.waitFor(() => expect(onchanged).toHaveBeenCalledOnce());
  });

  it("requires confirmation before applying recursively to a directory", async () => {
    vi.mocked(ipc.inspectorGet).mockResolvedValue(metadata(true, "directory"));
    vi.mocked(ipc.chmodRecursive).mockResolvedValue({
      root: "/home/user/docs",
      changed: 2,
      skipped: 0,
      failed: []
    });
    render(Inspector, {
      props: {
        source: "remote",
        connectionId: "connection-1",
        path: "/home/user/docs",
        onclose: vi.fn()
      }
    });

    await fireEvent.click(
      await screen.findByRole("button", { name: "Apply to enclosed…" })
    );
    expect(confirmAction).toHaveBeenCalledOnce();
    expect(ipc.chmodRecursive).toHaveBeenCalledWith(
      "connection-1",
      "/home/user/docs",
      0o644,
      true
    );
  });

  it("does not invent permission values when the detail is unavailable", async () => {
    const unavailable = metadata(true, "file", false);
    unavailable.permissionDetail = {
      mode: null,
      symbolic: null,
      user: { read: false, write: false, execute: false },
      group: { read: false, write: false, execute: false },
      world: { read: false, write: false, execute: false },
      editable: false,
      reason: "The server returned an unrecognized permission value."
    };
    vi.mocked(ipc.inspectorGet).mockResolvedValue(unavailable);
    render(Inspector, {
      props: {
        source: "remote",
        connectionId: "connection-1",
        path: "/home/user/notes.txt",
        onclose: vi.fn()
      }
    });

    expect(
      await screen.findByText("The server returned an unrecognized permission value.")
    ).toBeTruthy();
    expect(screen.queryByText(/Octal:/)).toBeNull();
    expect(screen.queryByText("---------")).toBeNull();
    expect(screen.queryByLabelText("User Read")).toBeNull();
    expect(screen.queryByRole("button", { name: "Apply" })).toBeNull();
  });

  it("keeps local permissions read-only", async () => {
    vi.mocked(ipc.inspectorGet).mockResolvedValue(metadata(true, "file", false));
    render(Inspector, {
      props: {
        source: "local",
        path: "/home/user/notes.txt",
        onclose: vi.fn()
      }
    });

    expect(await screen.findByText("Local permissions are read-only.")).toBeTruthy();
    expect(screen.queryByRole("button", { name: "Apply" })).toBeNull();
  });

  it("copies the remote path and shows inline feedback", async () => {
    const writeText = vi.fn().mockResolvedValue(undefined);
    Object.assign(navigator, { clipboard: { writeText } });
    vi.mocked(ipc.inspectorGet).mockResolvedValue(metadata());
    render(Inspector, {
      props: {
        source: "remote",
        connectionId: "connection-1",
        path: "/home/user/notes.txt",
        onclose: vi.fn()
      }
    });

    await fireEvent.click(await screen.findByRole("button", { name: "Copy path" }));
    expect(writeText).toHaveBeenCalledWith("/home/user/notes.txt");
    expect(await screen.findByText("Path copied")).toBeTruthy();
  });
});
