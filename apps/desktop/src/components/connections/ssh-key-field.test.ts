import { cleanup, fireEvent, render, screen } from "@testing-library/svelte";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import SshKeyField from "./SshKeyField.svelte";
import { ipc } from "../../lib/ipc/client";

vi.mock("../../lib/ipc/client", () => ({
  ipc: {
    sshKeysList: vi.fn(),
    sshKeysValidate: vi.fn()
  }
}));

const candidatePath = "/Users/me/.ssh/id_ed25519";

beforeEach(() => {
  vi.mocked(ipc.sshKeysList).mockResolvedValue({
    directory: "/Users/me/.ssh",
    exists: true,
    readable: true,
    candidates: [{ fileName: "id_ed25519", path: candidatePath }]
  });
  vi.mocked(ipc.sshKeysValidate).mockResolvedValue({ valid: true });
});

afterEach(cleanup);

describe("SshKeyField", () => {
  it("shows discovered candidates and pre-fills a single key", async () => {
    const onchange = vi.fn();
    render(SshKeyField, { props: { value: "", onchange } });
    expect(await screen.findByRole("button", { name: "id_ed25519" })).toBeTruthy();
    expect(onchange).toHaveBeenCalledWith(candidatePath);
  });

  it("fills the field from a candidate and supports manual entry", async () => {
    render(SshKeyField, { props: { value: "" } });
    const candidate = await screen.findByRole("button", { name: "id_ed25519" });
    await fireEvent.click(candidate);
    expect((screen.getByLabelText("SSH key reference") as HTMLInputElement).value).toBe(
      candidatePath
    );
  });

  it("exposes a key button and manual entry remains available", () => {
    render(SshKeyField, { props: { value: "/custom/key" } });
    expect(screen.getByRole("button", { name: "Browse for a private key" })).toBeTruthy();
    expect((screen.getByLabelText("SSH key reference") as HTMLInputElement).value).toBe(
      "/custom/key"
    );
  });
});
