import { cleanup, fireEvent, render, screen } from "@testing-library/svelte";
import { afterEach, describe, expect, it, vi } from "vitest";
import CredentialPrompt from "./CredentialPrompt.svelte";

afterEach(cleanup);

describe("CredentialPrompt", () => {
  it("asks for an SSH key passphrase", async () => {
    const onsubmit = vi.fn();
    render(CredentialPrompt, {
      props: {
        profileName: "Example server",
        message: "That SSH key passphrase was not accepted. Try again.",
        secretKind: "passphrase",
        onsubmit,
        oncancel: vi.fn()
      }
    });

    expect(screen.getByText("SSH key passphrase required")).toBeTruthy();
    expect(screen.getByLabelText("SSH key passphrase")).toBeTruthy();
    await fireEvent.input(screen.getByLabelText("SSH key passphrase"), {
      target: { value: "secret" }
    });
    await fireEvent.click(screen.getByRole("button", { name: "Connect" }));
    expect(onsubmit).toHaveBeenCalledWith("secret");
  });
});
