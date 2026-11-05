import { cleanup, fireEvent, render, screen } from "@testing-library/svelte";
import { afterEach, describe, expect, it, vi } from "vitest";
import WorkspaceSidebar from "./WorkspaceSidebar.svelte";
import { workspaceLayout, serverFavorite } from "../../lib/test-utils/fixtures";

afterEach(cleanup);

describe("WorkspaceSidebar", () => {
  it("renders labeled sections and supports search clearing", async () => {
    render(WorkspaceSidebar, {
      props: {
        layout: workspaceLayout,
        servers: [serverFavorite()],
        directoryFavorites: []
      }
    });
    expect(screen.getByText("Saved Servers")).toBeTruthy();
    expect(screen.getByText("Directory Favorites")).toBeTruthy();
    expect(screen.getByText("Automation")).toBeTruthy();
    const search = screen.getByLabelText("Search saved servers and directory favorites");
    await fireEvent.input(search, { target: { value: "missing" } });
    expect(screen.getByText("No matching servers.")).toBeTruthy();
    await fireEvent.click(screen.getByRole("button", { name: "Clear search" }));
    expect(screen.getByText("Production")).toBeTruthy();
  });

  it("toggles sections and opens automation", async () => {
    const ontogglesection = vi.fn();
    const onopenautomation = vi.fn();
    render(WorkspaceSidebar, {
      props: {
        layout: workspaceLayout,
        servers: [],
        directoryFavorites: [],
        ontogglesection,
        onopenautomation
      }
    });
    await fireEvent.click(screen.getByRole("button", { name: /Saved Servers/ }));
    expect(ontogglesection).toHaveBeenCalledWith("servers", false);
    await fireEvent.click(screen.getByRole("button", { name: /MCP automation/ }));
    expect(onopenautomation).toHaveBeenCalledOnce();
  });

  it("shows a reopen control when collapsed", async () => {
    const onreopen = vi.fn();
    render(WorkspaceSidebar, {
      props: {
        layout: { ...workspaceLayout, sidebarVisible: false },
        visible: false,
        onreopen
      }
    });
    await fireEvent.click(screen.getByRole("button", { name: "Show sidebar" }));
    expect(onreopen).toHaveBeenCalledOnce();
  });
});
