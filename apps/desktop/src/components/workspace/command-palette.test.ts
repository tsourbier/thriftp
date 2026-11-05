import { cleanup, fireEvent, render, screen } from "@testing-library/svelte";
import { afterEach, describe, expect, it, vi } from "vitest";
import CommandPalette from "./CommandPalette.svelte";
import { buildEntries } from "../../lib/commands/registry";
import { serverFavorite } from "../../lib/test-utils/fixtures";

afterEach(cleanup);

function entries() {
  return buildEntries({
    servers: [serverFavorite()],
    directoryFavorites: [],
    tabs: [],
    recent: [],
    connectedProfileIds: ["profile-1"],
    canRefresh: true
  });
}

describe("CommandPalette", () => {
  it("focuses the search field and runs the highlighted result", async () => {
    const onrun = vi.fn();
    render(CommandPalette, { props: { entries: entries(), onrun, onclose: vi.fn() } });
    const input = screen.getByLabelText("Search commands") as HTMLInputElement;
    expect(document.activeElement).toBe(input);
    await fireEvent.input(input, { target: { value: "production" } });
    await fireEvent.keyDown(input, { key: "Enter" });
    expect(onrun).toHaveBeenCalledOnce();
  });

  it("shows an empty state and closes with Escape", async () => {
    const onclose = vi.fn();
    render(CommandPalette, { props: { entries: entries(), onrun: vi.fn(), onclose } });
    const input = screen.getByLabelText("Search commands");
    await fireEvent.input(input, { target: { value: "no-such-command" } });
    expect(screen.getByText(/No matching commands/)).toBeTruthy();
    await fireEvent.keyDown(input, { key: "Escape" });
    expect(onclose).toHaveBeenCalledOnce();
  });

  it("marks unavailable results disabled", async () => {
    render(CommandPalette, {
      props: { entries: entries(), onrun: vi.fn(), onclose: vi.fn() }
    });
    const input = screen.getByLabelText("Search commands");
    await fireEvent.input(input, { target: { value: "toggle hidden" } });
    const button = screen.getByRole("button", { name: /Toggle hidden files/ });
    expect(button.getAttribute("aria-disabled")).toBe("false");
  });
});
