import { cleanup, render, screen } from "@testing-library/svelte";
import { afterEach, describe, expect, it, vi } from "vitest";
import BrowserToolbar from "./BrowserToolbar.svelte";

afterEach(cleanup);

describe("BrowserToolbar", () => {
  it("does not render back or forward buttons", () => {
    render(BrowserToolbar, {
      props: {
        path: "/home/user",
        onnavigate: vi.fn(),
        onup: vi.fn(),
        onrefresh: vi.fn(),
        onsort: vi.fn()
      }
    });

    expect(screen.queryByRole("button", { name: "Back" })).toBeNull();
    expect(screen.queryByRole("button", { name: "Forward" })).toBeNull();
    expect(screen.getByRole("button", { name: "Parent directory" })).toBeTruthy();
    expect(screen.getByRole("button", { name: "Refresh" })).toBeTruthy();
  });
});
