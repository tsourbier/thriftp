import { cleanup, fireEvent, render, screen } from "@testing-library/svelte";
import { afterEach, describe, expect, it, vi } from "vitest";
import BreadcrumbBar from "./BreadcrumbBar.svelte";

afterEach(cleanup);

describe("BreadcrumbBar", () => {
  it("navigates to an ancestor segment", async () => {
    const onnavigate = vi.fn();
    render(BreadcrumbBar, { props: { path: "/srv/www/app", onnavigate } });
    await fireEvent.click(screen.getByRole("button", { name: "/srv/www" }));
    expect(onnavigate).toHaveBeenCalledWith("/srv/www");
  });

  it("opens editable mode from the current segment and submits with Enter", async () => {
    const onnavigate = vi.fn();
    render(BreadcrumbBar, { props: { path: "/srv/www/app", onnavigate } });
    await fireEvent.click(screen.getByRole("button", { name: "/srv/www/app" }));
    const input = screen.getByLabelText("Edit current path") as HTMLInputElement;
    await fireEvent.input(input, { target: { value: "/srv/other" } });
    await fireEvent.keyDown(input, { key: "Enter" });
    expect(onnavigate).toHaveBeenCalledWith("/srv/other");
  });

  it("hides breadcrumbs when there is no location", () => {
    render(BreadcrumbBar, { props: { path: "", hasLocation: false } });
    expect(screen.queryByLabelText("Current path")).toBeNull();
  });
});
