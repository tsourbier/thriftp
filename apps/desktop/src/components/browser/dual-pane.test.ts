import { cleanup, fireEvent, render, screen } from "@testing-library/svelte";
import { afterEach, describe, expect, it, vi } from "vitest";
import DualPane from "./DualPane.svelte";

vi.mock("../../lib/ipc/client", () => ({
  ipc: {
    openLocal: vi.fn().mockResolvedValue({ listingId: "local-1" }),
    openRemote: vi.fn().mockResolvedValue({ listingId: "remote-1" }),
    cancelListing: vi.fn().mockResolvedValue({ cancelled: true })
  }
}));

afterEach(cleanup);

describe("DualPane split shell", () => {
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
