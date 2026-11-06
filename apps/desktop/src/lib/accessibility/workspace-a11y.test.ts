import { cleanup, render, screen } from "@testing-library/svelte";
import { afterEach, describe, expect, it, vi } from "vitest";
import WorkspaceSidebar from "../../components/workspace/WorkspaceSidebar.svelte";
import BreadcrumbBar from "../../components/browser/BreadcrumbBar.svelte";
import DualPane from "../../components/browser/DualPane.svelte";
import PaneLinkToggle from "../../components/browser/PaneLinkToggle.svelte";
import CommandPalette from "../../components/workspace/CommandPalette.svelte";
import { workspaceLayout } from "../test-utils/fixtures";
import { buildEntries } from "../commands/registry";

vi.mock("../ipc/client", () => ({
  ipc: {
    openLocal: vi.fn().mockResolvedValue({ listingId: "local-1" }),
    openRemote: vi.fn().mockResolvedValue({ listingId: "remote-1" }),
    cancelListing: vi.fn().mockResolvedValue({ cancelled: true })
  }
}));

afterEach(cleanup);

describe("workspace accessibility", () => {
  it("exposes expanded sidebar sections", () => {
    render(WorkspaceSidebar, { props: { layout: workspaceLayout } });
    expect(
      screen.getByRole("button", { name: /Saved Servers/ }).getAttribute("aria-expanded")
    ).toBe("true");
  });

  it("marks the current breadcrumb and exposes full paths", () => {
    render(BreadcrumbBar, { props: { path: "/srv/www/app" } });
    const current = screen.getByRole("button", { name: "/srv/www/app" });
    expect(current.getAttribute("aria-current")).toBe("location");
    expect(current.getAttribute("title")).toBe("/srv/www/app");
  });

  it("exposes the split separator and its value", () => {
    render(DualPane, { props: { connectionId: null } });
    const separator = screen.getByRole("separator");
    expect(separator.getAttribute("aria-valuenow")).toBe("50");
    expect(separator.getAttribute("tabindex")).toBe("0");
  });

  it("exposes the pane link state through an accessible toggle", async () => {
    const ontoggle = vi.fn();
    const { rerender } = render(PaneLinkToggle, {
      props: { linked: false, ontoggle }
    });
    const link = screen.getByRole("button", { name: "Link" });
    expect(link.getAttribute("aria-pressed")).toBe("false");
    expect(link.getAttribute("title")).toMatch(/mirror/i);

    await rerender({ linked: true, ontoggle });
    const unlink = screen.getByRole("button", { name: "Unlink" });
    expect(unlink.getAttribute("aria-pressed")).toBe("true");
  });

  it("renders the palette as a modal dialog", () => {
    const entries = buildEntries({
      servers: [],
      directoryFavorites: [],
      tabs: [],
      recent: [],
      connectedProfileIds: [],
      canRefresh: false
    });
    render(CommandPalette, { props: { entries } });
    expect(screen.getByRole("dialog").getAttribute("aria-modal")).toBe("true");
  });
});
