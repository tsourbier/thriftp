import { cleanup, fireEvent, render, screen } from "@testing-library/svelte";
import { afterEach, describe, expect, it, vi } from "vitest";
import DualPane from "./DualPane.svelte";

vi.mock("../../lib/ipc/client", () => ({
  ipc: {
    openLocal: vi.fn().mockResolvedValue({ listingId: "local-1" }),
    openRemote: vi.fn().mockResolvedValue({ listingId: "remote-1" }),
    cancelListing: vi.fn().mockResolvedValue({ cancelled: true }),
    stat: vi.fn(),
    probeDirectory: vi.fn().mockResolvedValue(true)
  }
}));

import { ipc } from "../../lib/ipc/client";

afterEach(cleanup);

describe("DualPane split shell", () => {
  it("places Disconnect beside the remote title and invokes it", async () => {
    const ondisconnect = vi.fn();
    render(DualPane, {
      props: {
        connectionId: "connection-1",
        connectionLabel: "Example",
        ondisconnect
      }
    });

    await fireEvent.click(screen.getByRole("button", { name: "Disconnect" }));
    expect(ondisconnect).toHaveBeenCalledOnce();
  });

  it("hides Disconnect without an active connection", () => {
    render(DualPane, { props: { connectionId: null } });
    expect(screen.queryByRole("button", { name: "Disconnect" })).toBeNull();
  });

  it("renders an independent listing container in each pane", () => {
    render(DualPane, {
      props: { connectionId: "connection-1", connectionLabel: "Example" }
    });
    const listings = screen.getAllByLabelText("Directory contents");
    expect(listings).toHaveLength(2);
    expect(listings[0]).not.toBe(listings[1]);
  });

  it("mirrors the same relative parent move into the other pane when linked", async () => {
    vi.mocked(ipc.stat).mockResolvedValue({
      entryType: "directory"
    } as never);
    const onremotepathchange = vi.fn();
    render(DualPane, {
      props: {
        connectionId: "connection-1",
        linked: true,
        localPath: "/local/a",
        remotePath: "/remote/a",
        onremotepathchange
      }
    });

    const upButtons = screen.getAllByRole("button", { name: "Parent directory" });
    await fireEvent.click(upButtons[0]);

    await vi.waitFor(() =>
      expect(ipc.stat).toHaveBeenCalledWith("remote", "/remote", "connection-1")
    );
    await vi.waitFor(() => expect(onremotepathchange).toHaveBeenCalledWith("/remote"));
  });

  it("leaves the counterpart unchanged when the mirrored location is unavailable", async () => {
    vi.mocked(ipc.stat).mockRejectedValue(new Error("not found"));
    const onremotepathchange = vi.fn();
    render(DualPane, {
      props: {
        connectionId: "connection-1",
        linked: true,
        localPath: "/local/a",
        remotePath: "/remote/a",
        onremotepathchange
      }
    });

    const upButtons = screen.getAllByRole("button", { name: "Parent directory" });
    await fireEvent.click(upButtons[0]);

    await vi.waitFor(() => expect(ipc.stat).toHaveBeenCalled());
    expect(onremotepathchange).not.toHaveBeenCalled();
  });

  it("leaves the counterpart unchanged when the mirrored location is not readable", async () => {
    vi.mocked(ipc.stat).mockResolvedValue({
      entryType: "directory"
    } as never);
    vi.mocked(ipc.probeDirectory).mockResolvedValue(false);
    const onremotepathchange = vi.fn();
    render(DualPane, {
      props: {
        connectionId: "connection-1",
        linked: true,
        localPath: "/local/a",
        remotePath: "/remote/a",
        onremotepathchange
      }
    });

    const upButtons = screen.getAllByRole("button", { name: "Parent directory" });
    await fireEvent.click(upButtons[0]);

    await vi.waitFor(() => expect(ipc.probeDirectory).toHaveBeenCalled());
    expect(onremotepathchange).not.toHaveBeenCalled();
  });

  it("does not navigate on toggle when Link is enabled", async () => {
    const onremotepathchange = vi.fn();
    const { rerender } = render(DualPane, {
      props: {
        connectionId: "connection-1",
        linked: false,
        localPath: "/local/a",
        remotePath: "/remote/a",
        onremotepathchange
      }
    });

    await rerender({
      connectionId: "connection-1",
      linked: true,
      localPath: "/local/a",
      remotePath: "/remote/a",
      onremotepathchange
    });

    expect(ipc.stat).not.toHaveBeenCalled();
    expect(onremotepathchange).not.toHaveBeenCalled();
  });

  it("applies the persisted view settings when the active tab changes", async () => {
    const { rerender } = render(DualPane, {
      props: {
        connectionId: "connection-1",
        tabId: "tab-a",
        defaultSortKey: "name",
        defaultSortDirection: "asc"
      }
    });

    await rerender({
      connectionId: "connection-1",
      tabId: "tab-b",
      defaultSortKey: "size",
      defaultSortDirection: "desc",
      defaultShowHidden: true,
      defaultFilter: "log"
    });

    const sizeHeaders = screen.getAllByRole("columnheader", { name: /Size/ });
    expect(sizeHeaders[1].getAttribute("aria-sort")).toBe("descending");
    const filters = screen.getAllByLabelText("Filter entries") as HTMLInputElement[];
    expect(filters[1].value).toBe("log");
    const hidden = screen.getAllByRole("checkbox") as HTMLInputElement[];
    expect(hidden[1].checked).toBe(true);
  });

  it("mirrors a tab change exactly once while linked", async () => {
    vi.mocked(ipc.stat).mockResolvedValue({
      entryType: "directory"
    } as never);
    vi.mocked(ipc.probeDirectory).mockResolvedValue(true);
    const onlocalpathchange = vi.fn();
    const onremotepathchange = vi.fn();
    const { rerender } = render(DualPane, {
      props: {
        connectionId: "connection-1",
        tabId: "tab-a",
        linked: true,
        localPath: "/local/a",
        remotePath: "/remote/a",
        onlocalpathchange,
        onremotepathchange
      }
    });

    await vi.waitFor(() => expect(ipc.openRemote).toHaveBeenCalled());

    await rerender({
      connectionId: "connection-1",
      tabId: "tab-b",
      linked: true,
      localPath: "/local/b",
      remotePath: "/remote/b",
      onlocalpathchange,
      onremotepathchange
    });

    await vi.waitFor(() => expect(onlocalpathchange).toHaveBeenCalledWith("/local/b"));
    expect(onremotepathchange).not.toHaveBeenCalled();
  });

  it("mirrors breadcrumb ancestor jumps relative to the other pane", async () => {
    vi.mocked(ipc.stat).mockResolvedValue({
      entryType: "directory"
    } as never);
    vi.mocked(ipc.probeDirectory).mockResolvedValue(true);
    const onremotepathchange = vi.fn();
    render(DualPane, {
      props: {
        connectionId: "connection-1",
        linked: true,
        localPath: "/local/a/b",
        remotePath: "/remote/a/b",
        onremotepathchange
      }
    });

    await fireEvent.click(screen.getByRole("button", { name: "/local/a" }));
    await vi.waitFor(() => expect(onremotepathchange).toHaveBeenCalledWith("/remote/a"));
  });

  it("mirrors history direction through keyboard navigation", async () => {
    vi.mocked(ipc.stat).mockResolvedValue({
      entryType: "directory"
    } as never);
    vi.mocked(ipc.probeDirectory).mockResolvedValue(true);
    const onremotepathchange = vi.fn();
    const { container } = render(DualPane, {
      props: {
        connectionId: "connection-1",
        linked: true,
        localPath: "/local/a",
        remotePath: "/remote/a",
        onremotepathchange
      }
    });

    const upButtons = screen.getAllByRole("button", { name: "Parent directory" });
    await fireEvent.click(upButtons[0]);
    await vi.waitFor(() => expect(onremotepathchange).toHaveBeenCalledWith("/remote"));

    const localList = container.querySelector(".list") as HTMLElement;
    await fireEvent.keyDown(localList, { key: "ArrowLeft", altKey: true });
    await vi.waitFor(() =>
      expect(onremotepathchange).toHaveBeenLastCalledWith("/remote/a")
    );
  });

  it("always renders local and remote panes with a keyboard separator", async () => {
    render(DualPane, { props: { connectionId: null } });
    expect(screen.getByRole("region", { name: "Local files" })).toBeTruthy();
    expect(screen.getByRole("region", { name: "Remote files" })).toBeTruthy();
    const separator = screen.getByRole("separator");
    expect(separator.getAttribute("aria-valuenow")).toBe("50");
    await fireEvent.keyDown(separator, { key: "ArrowRight" });
    expect(separator.getAttribute("aria-valuenow")).toBe("52");
    await fireEvent.keyDown(separator, { key: "Enter" });
    expect(separator.getAttribute("aria-valuenow")).toBe("50");
  });
});
