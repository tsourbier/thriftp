import { cleanup, fireEvent, render, screen } from "@testing-library/svelte";
import { afterEach, describe, expect, it, vi } from "vitest";

import EditFavoriteDialog from "./EditFavoriteDialog.svelte";
import { favoriteFixture } from "../../lib/server-favorites/test-utils";

afterEach(cleanup);

describe("EditFavoriteDialog", () => {
  it("prefills every saved connection setting", () => {
    render(EditFavoriteDialog, {
      props: {
        favorite: favoriteFixture({
          displayName: "Production",
          host: "prod.example",
          port: 2222,
          username: "deploy",
          timeoutSeconds: 45,
          pinned: true
        }),
        onsave: vi.fn(),
        oncancel: vi.fn()
      }
    });

    expect((screen.getByLabelText("Name") as HTMLInputElement).value).toBe("Production");
    expect((screen.getByLabelText("Host") as HTMLInputElement).value).toBe(
      "prod.example"
    );
    expect((screen.getByLabelText("Port") as HTMLInputElement).value).toBe("2222");
    expect((screen.getByLabelText("Username") as HTMLInputElement).value).toBe("deploy");
    expect((screen.getByLabelText("Timeout") as HTMLInputElement).value).toBe("45");
    expect((screen.getByLabelText("Pin to the top") as HTMLInputElement).checked).toBe(
      true
    );
  });

  it("submits edited values", async () => {
    const onsave = vi.fn();
    render(EditFavoriteDialog, {
      props: { favorite: favoriteFixture(), onsave, oncancel: vi.fn() }
    });

    await fireEvent.input(screen.getByLabelText("Name"), {
      target: { value: "Renamed" }
    });
    await fireEvent.input(screen.getByLabelText("Username"), {
      target: { value: "operator" }
    });
    await fireEvent.click(screen.getByRole("button", { name: "Save" }));

    expect(onsave).toHaveBeenCalledOnce();
    const [profile] = onsave.mock.calls[0];
    expect(profile.displayName).toBe("Renamed");
    expect(profile.username).toBe("operator");
  });

  it("prompts before discarding unsaved changes", async () => {
    const oncancel = vi.fn();
    render(EditFavoriteDialog, {
      props: { favorite: favoriteFixture(), onsave: vi.fn(), oncancel }
    });

    await fireEvent.input(screen.getByLabelText("Name"), {
      target: { value: "Draft" }
    });
    await fireEvent.click(screen.getByRole("button", { name: "Cancel" }));
    expect(oncancel).not.toHaveBeenCalled();
    expect(
      screen.getByRole("alertdialog", { name: "Discard unsaved changes" })
    ).toBeTruthy();

    await fireEvent.click(screen.getByRole("button", { name: "Keep editing" }));
    expect(
      screen.queryByRole("alertdialog", { name: "Discard unsaved changes" })
    ).toBeNull();

    await fireEvent.click(screen.getByRole("button", { name: "Cancel" }));
    await fireEvent.click(screen.getByRole("button", { name: "Discard changes" }));
    expect(oncancel).toHaveBeenCalledOnce();
  });

  it("tests pending settings without saving", async () => {
    const onsave = vi.fn();
    const ontest = vi.fn().mockResolvedValue({ ok: true });
    render(EditFavoriteDialog, {
      props: { favorite: favoriteFixture(), onsave, ontest, oncancel: vi.fn() }
    });

    await fireEvent.input(screen.getByLabelText("Host"), {
      target: { value: "pending.example" }
    });
    await fireEvent.input(screen.getByLabelText("Test credential"), {
      target: { value: "test-only" }
    });
    await fireEvent.click(screen.getByRole("button", { name: "Test connection" }));

    expect(ontest).toHaveBeenCalledWith(
      expect.objectContaining({ host: "pending.example" }),
      "test-only"
    );
    expect(await screen.findByText("Connection test succeeded.")).toBeTruthy();
    expect(onsave).not.toHaveBeenCalled();
  });

  it("reports connection-test failure feedback", async () => {
    render(EditFavoriteDialog, {
      props: {
        favorite: favoriteFixture(),
        onsave: vi.fn(),
        ontest: vi.fn().mockResolvedValue({
          ok: false,
          error: { message: "The server certificate could not be verified." }
        }),
        oncancel: vi.fn()
      }
    });
    await fireEvent.click(screen.getByRole("button", { name: "Test connection" }));
    expect(
      await screen.findByText("The server certificate could not be verified.")
    ).toBeTruthy();
  });

  it("requires a key reference for SSH private-key authentication", async () => {
    const onsave = vi.fn();
    render(EditFavoriteDialog, {
      props: { favorite: favoriteFixture(), onsave, oncancel: vi.fn() }
    });

    await fireEvent.change(screen.getByLabelText("Authentication"), {
      target: { value: "sshPrivateKey" }
    });
    await fireEvent.click(screen.getByRole("button", { name: "Save" }));
    expect(onsave).not.toHaveBeenCalled();
    expect(screen.getByRole("alert").textContent).toContain(
      "private-key reference is required"
    );

    await fireEvent.input(screen.getByLabelText("SSH key reference"), {
      target: { value: "~/.ssh/id_ed25519" }
    });
    await fireEvent.click(screen.getByRole("button", { name: "Save" }));
    expect(onsave).toHaveBeenCalledWith(
      expect.objectContaining({
        authMethod: "sshPrivateKey",
        sshKeyRef: "~/.ssh/id_ed25519"
      }),
      "ask",
      undefined,
      undefined
    );
  });

  it("forces password authentication for FTP protocols", async () => {
    render(EditFavoriteDialog, {
      props: { favorite: favoriteFixture(), onsave: vi.fn(), oncancel: vi.fn() }
    });
    await fireEvent.change(screen.getByLabelText("Protocol"), {
      target: { value: "ftpsExplicit" }
    });
    expect((screen.getByLabelText("Authentication") as HTMLSelectElement).disabled).toBe(
      true
    );
  });
});

describe("EditFavoriteDialog credential removal", () => {
  it("offers an explicit remove-saved-credential action", async () => {
    const onremovecredential = vi.fn();
    render(EditFavoriteDialog, {
      props: {
        favorite: favoriteFixture({
          credentialPolicy: "save",
          hasSavedCredential: true
        }),
        onsave: vi.fn(),
        onremovecredential,
        oncancel: vi.fn()
      }
    });
    await fireEvent.click(
      screen.getByRole("button", { name: "Remove saved credential" })
    );
    expect(onremovecredential).toHaveBeenCalledOnce();
  });
});
