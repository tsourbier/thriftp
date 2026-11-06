import { cleanup, fireEvent, render, screen } from "@testing-library/svelte";
import { afterEach, describe, expect, it, vi } from "vitest";

import SaveFavoriteDialog from "./SaveFavoriteDialog.svelte";
import type { ReusableCredential } from "../../lib/ipc/types";

vi.mock("../../lib/favorites/path-browser", () => ({
  browseLocalFolder: vi.fn()
}));

import { browseLocalFolder } from "../../lib/favorites/path-browser";

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

  it("fills the local path from the native folder browser", async () => {
    vi.mocked(browseLocalFolder).mockResolvedValue("/Users/me/Downloads");
    render(SaveFavoriteDialog, { props: { onsave: vi.fn(), oncancel: vi.fn() } });

    await fireEvent.click(screen.getByRole("button", { name: "Browse" }));
    expect(await screen.findByDisplayValue("/Users/me/Downloads")).toBeTruthy();
    expect(browseLocalFolder).toHaveBeenCalled();
  });

  it("keeps the previous local path when browsing is cancelled", async () => {
    vi.mocked(browseLocalFolder).mockResolvedValue(null);
    render(SaveFavoriteDialog, {
      props: {
        initial: { localPath: "/Users/me/Projects" },
        onsave: vi.fn(),
        oncancel: vi.fn()
      }
    });

    await fireEvent.click(screen.getByRole("button", { name: "Browse" }));
    expect((screen.getByLabelText("Local path") as HTMLInputElement).value).toBe(
      "/Users/me/Projects"
    );
  });

  it("keeps validation feedback visible with the actions reachable", async () => {
    render(SaveFavoriteDialog, { props: { onsave: vi.fn(), oncancel: vi.fn() } });

    await fireEvent.input(screen.getByLabelText("Name"), {
      target: { value: "Alpha" }
    });
    await fireEvent.input(screen.getByLabelText("Host"), {
      target: { value: "example.test" }
    });
    await fireEvent.input(screen.getByLabelText("Username"), {
      target: { value: "tester" }
    });
    await fireEvent.input(screen.getByLabelText("Remote path"), {
      target: { value: "relative/path" }
    });
    await fireEvent.click(screen.getByRole("button", { name: "Save" }));

    expect(screen.getByText("Remote path must be an absolute server path.")).toBeTruthy();
    expect(screen.getByRole("button", { name: "Save" })).toBeTruthy();
    expect(screen.getByRole("button", { name: "Cancel" })).toBeTruthy();
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
