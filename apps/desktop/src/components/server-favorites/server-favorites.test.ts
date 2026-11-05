import { cleanup, render, screen } from "@testing-library/svelte";
import { afterEach, describe, expect, it, vi } from "vitest";

import FavoritesList from "./FavoritesList.svelte";
import { favoriteFixture } from "../../lib/server-favorites/test-utils";

afterEach(cleanup);

describe("server favorites surface", () => {
  it("renders saved server identity and credential state", () => {
    render(FavoritesList, {
      props: {
        favorites: [
          favoriteFixture({
            displayName: "Production",
            credentialPolicy: "save",
            hasSavedCredential: true
          })
        ],
        onconnect: vi.fn(),
        onedit: vi.fn(),
        ondelete: vi.fn(),
        onpin: vi.fn()
      }
    });
    expect(screen.getByText("Production")).toBeTruthy();
    expect(screen.getByText("credentials saved")).toBeTruthy();
  });
});
