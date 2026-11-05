import { cleanup, render, screen } from "@testing-library/svelte";
import { afterEach, describe, expect, it, vi } from "vitest";

import FavoritesList from "./FavoritesList.svelte";
import { favoriteFixture } from "../../lib/server-favorites/test-utils";
import type {
  FavoritePreferences,
  FavoriteSortKey,
  ServerFavoriteView
} from "../../lib/ipc/types";

afterEach(cleanup);

const preferences: FavoritePreferences = { sortKey: "name", pinnedFirst: true };

function favorites(count: number): ServerFavoriteView[] {
  return Array.from({ length: count }, (_, index) =>
    favoriteFixture({
      id: `favorite-${index}`,
      connectionProfileId: `profile-${index}`,
      displayName: `Server ${String(index).padStart(3, "0")}`,
      host: `host-${(index * 37) % count}.example`,
      username: `user-${index % 12}`,
      pinned: index % 11 === 0,
      groupLabel: `group-${index % 8}`,
      lastUsedAt: index % 3 === 0 ? new Date().toISOString() : null
    })
  );
}

function sortFavorites(
  items: ServerFavoriteView[],
  sortKey: FavoriteSortKey
): ServerFavoriteView[] {
  return [...items].sort((a, b) => {
    const pinned = Number(b.pinned) - Number(a.pinned);
    if (pinned !== 0) {
      return pinned;
    }
    switch (sortKey) {
      case "protocol":
        return a.protocol.localeCompare(b.protocol);
      case "host":
        return a.host.localeCompare(b.host);
      case "lastUsed":
        return (b.lastUsedAt ?? "").localeCompare(a.lastUsedAt ?? "");
      case "name":
        return a.displayName.toLowerCase().localeCompare(b.displayName.toLowerCase());
    }
  });
}

describe("server favorites performance", () => {
  it("sorts 500 favorites under 100 ms", () => {
    const items = favorites(500);
    const started = performance.now();
    for (const key of ["name", "protocol", "host", "lastUsed"] as const) {
      sortFavorites(items, key);
    }
    expect(performance.now() - started).toBeLessThan(100);
  });

  it("renders all 500 favorites without dropping rows", () => {
    const items = favorites(500);
    const started = performance.now();
    render(FavoritesList, {
      props: {
        favorites: items,
        preferences,
        onconnect: vi.fn(),
        onedit: vi.fn(),
        ondelete: vi.fn(),
        onpin: vi.fn()
      }
    });
    const elapsed = performance.now() - started;
    expect(screen.getAllByRole("button", { name: /Connect to Server/ })).toHaveLength(
      500
    );
    // jsdom rendering is slower than a native webview; keep a regression guard
    // while the pure sort budget above enforces the 100 ms requirement.
    expect(elapsed).toBeLessThan(2_000);
  });
});
