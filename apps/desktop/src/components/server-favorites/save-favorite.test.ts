import { cleanup, fireEvent, render, screen } from "@testing-library/svelte";
import { afterEach, describe, expect, it, vi } from "vitest";

import SaveFavoriteDialog from "./SaveFavoriteDialog.svelte";
import type { ReusableCredential } from "../../lib/ipc/types";

afterEach(cleanup);

describe("SaveFavoriteDialog", () => {
  it("submits connection settings and a saved credential", async () => {
    const onsave = vi.fn();
    render(SaveFavoriteDialog, { props: { onsave, oncancel: vi.fn() } });

    await fireEvent.input(screen.getByLabelText("Name"), { target: { value: "Alpha" } });
    await fireEvent.input(screen.getByLabelText("Host"), {
      target: { value: "example.test" }
    });
    await fireEvent.input(screen.getByLabelText("Username"), {
      target: { value: "tester" }
    });
    await fireEvent.change(screen.getByLabelText("Credential policy"), {
      target: { value: "save" }
    });
    await fireEvent.input(screen.getByLabelText("Password"), {
      target: { value: "secret" }
    });
    await fireEvent.click(screen.getByRole("button", { name: "Save" }));

    expect(onsave).toHaveBeenCalledOnce();
    const [profile, policy, secret] = onsave.mock.calls[0];
    expect(profile.displayName).toBe("Alpha");
    expect(profile.host).toBe("example.test");
    expect(policy).toBe("save");
    expect(secret).toBe("secret");
  });

  it("can cancel without saving", async () => {
    const oncancel = vi.fn();
    render(SaveFavoriteDialog, { props: { onsave: vi.fn(), oncancel } });
    await fireEvent.click(screen.getByRole("button", { name: "Cancel" }));
    expect(oncancel).toHaveBeenCalledOnce();
  });
});

describe("SaveFavoriteDialog credential reuse", () => {
  const reusable: ReusableCredential = {
    credentialRefId: "ref-1",
    protocol: "sftp",
    host: "example.test",
    port: 22,
    username: "tester",
    authMethod: "password",
    affectedFavoriteCount: 2
  };

  it("offers only matching credentials and submits the selected reference", async () => {
    const onsave = vi.fn();
    render(SaveFavoriteDialog, {
      props: {
        reusable: [reusable],
        onsave,
        oncancel: vi.fn()
      }
    });
    await fireEvent.input(screen.getByLabelText("Name"), {
      target: { value: "Alpha" }
    });
    await fireEvent.input(screen.getByLabelText("Host"), {
      target: { value: "example.test" }
    });
    await fireEvent.input(screen.getByLabelText("Username"), {
      target: { value: "tester" }
    });
    await fireEvent.change(screen.getByLabelText("Credential policy"), {
      target: { value: "save" }
    });
    await fireEvent.change(screen.getByLabelText("Reuse saved credential"), {
      target: { value: "ref-1" }
    });
    await fireEvent.click(screen.getByRole("button", { name: "Save" }));

    const [, policy, secret, credentialRefId] = onsave.mock.calls[0];
    expect(policy).toBe("save");
    expect(secret).toBeUndefined();
    expect(credentialRefId).toBe("ref-1");
  });
});
