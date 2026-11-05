import { cleanup, fireEvent, render, screen } from "@testing-library/svelte";
import { afterEach, describe, expect, it, vi } from "vitest";
import HostKeyPrompt from "./HostKeyPrompt.svelte";

afterEach(cleanup);

function renderPrompt(changed: boolean) {
  const ontrust = vi.fn();
  const oncancel = vi.fn();
  render(HostKeyPrompt, {
    props: {
      profileName: "Example server",
      host: "example.test",
      port: 22,
      fingerprint: "abcdefghijklmnopqrstuvwxyz012345",
      changed,
      ontrust,
      oncancel
    }
  });
  return { ontrust, oncancel };
}

describe("HostKeyPrompt", () => {
  it("shows the fingerprint and trusts an unknown key", async () => {
    const { ontrust } = renderPrompt(false);
    expect(screen.getByText("SHA256:abcdefghijklmnopqrstuvwxyz012345")).toBeTruthy();
    await fireEvent.click(screen.getByRole("button", { name: "Trust and connect" }));
    expect(ontrust).toHaveBeenCalledOnce();
  });

  it("requires acknowledgement before replacing a changed key", async () => {
    const { ontrust } = renderPrompt(true);
    const trust = screen.getByRole("button", {
      name: "Trust and connect"
    }) as HTMLButtonElement;
    expect(trust.disabled).toBe(true);
    await fireEvent.click(screen.getByRole("checkbox"));
    expect(trust.disabled).toBe(false);
    await fireEvent.click(trust);
    expect(ontrust).toHaveBeenCalledOnce();
  });

  it("can be cancelled without trusting", async () => {
    const { ontrust, oncancel } = renderPrompt(false);
    await fireEvent.click(screen.getByRole("button", { name: "Cancel" }));
    expect(oncancel).toHaveBeenCalledOnce();
    expect(ontrust).not.toHaveBeenCalled();
  });
});
