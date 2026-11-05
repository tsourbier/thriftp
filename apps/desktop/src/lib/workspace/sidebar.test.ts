import { describe, expect, it } from "vitest";
import { filterSidebar, matchesServer } from "./sidebar";
import type { DirectoryFavorite, ServerFavoriteView } from "../ipc/types";

function server(overrides: Partial<ServerFavoriteView> = {}): ServerFavoriteView {
  return {
    id: "s1",
    connectionProfileId: "p1",
    displayName: "Production",
    groupLabel: "Ops",
    protocol: "sftp",
    host: "prod.example",
    port: 22,
    username: "deploy",
    authMethod: "password",
    timeoutSeconds: 30,
    sshKeyRef: null,
    pinned: false,
    lastUsedAt: null,
    credentialPolicy: "ask",
    hasSavedCredential: false,
    localPath: "/Users/me/Projects",
    remotePath: "/srv/www",
    createdAt: "2026-11-05T00:00:00Z",
    updatedAt: "2026-11-05T00:00:00Z",
    ...overrides
  };
}

function directory(overrides: Partial<DirectoryFavorite> = {}): DirectoryFavorite {
  return {
    id: "d1",
    connectionProfileId: "p1",
    path: "/srv/www",
    label: "Web root",
    sortOrder: 0,
    createdAt: "2026-11-05T00:00:00Z",
    updatedAt: "2026-11-05T00:00:00Z",
    ...overrides
  };
}

describe("workspace sidebar filtering", () => {
  it("matches servers by name, host, protocol, group, and path", () => {
    const favorite = server();
    for (const query of ["production", "prod.example", "sftp", "ops", "/srv/www"]) {
      expect(matchesServer(favorite, query)).toBe(true);
    }
    expect(matchesServer(favorite, "staging")).toBe(false);
  });

  it("filters both sections and supports one-action clearing", () => {
    const result = filterSidebar({
      servers: [
        server(),
        server({ id: "s2", displayName: "Staging", host: "stage.example" })
      ],
      directoryFavorites: [directory()],
      query: "staging"
    });
    expect(result.servers).toHaveLength(1);
    expect(result.servers[0].displayName).toBe("Staging");
    expect(result.directoryFavorites).toHaveLength(0);
    expect(
      filterSidebar({ servers: [server()], directoryFavorites: [directory()], query: "" })
        .servers
    ).toHaveLength(1);
  });

  it("filters 200 servers and 200 favorites within 100 ms", () => {
    const servers = Array.from({ length: 200 }, (_, index) =>
      server({
        id: `s${index}`,
        displayName: `Server ${index}`,
        host: `host-${index}.test`
      })
    );
    const directoryFavorites = Array.from({ length: 200 }, (_, index) =>
      directory({ id: `d${index}`, label: `Favorite ${index}`, path: `/srv/${index}` })
    );
    const started = performance.now();
    const result = filterSidebar({ servers, directoryFavorites, query: "server 19" });
    expect(result.servers.length).toBeGreaterThan(0);
    expect(performance.now() - started).toBeLessThan(100);
  });
});
