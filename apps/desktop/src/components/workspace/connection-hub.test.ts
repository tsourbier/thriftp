import { cleanup, fireEvent, render, screen } from "@testing-library/svelte";
import { afterEach, describe, expect, it, vi } from "vitest";
import ConnectionHub from "./ConnectionHub.svelte";
import { historyEntry, serverFavorite } from "../../lib/test-utils/fixtures";

afterEach(cleanup);

describe("ConnectionHub", () => {
  it("renders favorites and switches to history", async () => {
    const onselecttab = vi.fn();
    render(ConnectionHub, {
      props: {
        favorites: [serverFavorite()],
        history: [historyEntry()],
        hubTab: "favorites",
        onselecttab
      }
    });
    expect(screen.getByRole("tab", { name: "Favorites" })).toBeTruthy();
    expect(screen.getByText("Production")).toBeTruthy();
    await fireEvent.click(screen.getByRole("tab", { name: "History" }));
    expect(onselecttab).toHaveBeenCalledWith("history");
  });

  it("renders history rows and clears history", async () => {
    const onclearhistory = vi.fn();
    render(ConnectionHub, {
      props: {
        favorites: [],
        history: [historyEntry()],
        hubTab: "history",
        onclearhistory
      }
    });
    expect(screen.getByText(/Recent successful connections/)).toBeTruthy();
    expect(screen.getByText("Production")).toBeTruthy();
    await fireEvent.click(screen.getByRole("button", { name: "Clear history" }));
    expect(onclearhistory).toHaveBeenCalledOnce();
  });

  it("shows an empty favorites state", () => {
    render(ConnectionHub, { props: { favorites: [], history: [], hubTab: "favorites" } });
    expect(screen.getByText(/Save a server to reconnect/)).toBeTruthy();
  });
});
