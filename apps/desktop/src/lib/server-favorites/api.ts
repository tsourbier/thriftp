import { ipc } from "../ipc/client";
import type {
  CredentialPolicy,
  ServerFavoriteInput,
  ServerFavoriteView
} from "../ipc/types";

function isAbsoluteLocal(path: string): boolean {
  return path.startsWith("/") || /^[A-Za-z]:[\\/]/.test(path);
}

function isAbsoluteRemote(path: string): boolean {
  return path.startsWith("/");
}

export async function resolveFavoritePrefillPaths(
  connectionId: string,
  localPath?: string | null,
  remotePath?: string | null
): Promise<{ localPath: string | null; remotePath: string | null }> {
  const resolve = async (
    source: "local" | "remote",
    path: string | null | undefined
  ): Promise<string | null> => {
    if (!path) {
      return null;
    }
    if (source === "local" && !isAbsoluteLocal(path)) {
      return null;
    }
    if (source === "remote" && !isAbsoluteRemote(path)) {
      return null;
    }
    try {
      const resolved = await ipc.resolvePath(
        source,
        path,
        source === "remote" ? connectionId : undefined
      );
      return source === "local"
        ? isAbsoluteLocal(resolved.path)
          ? resolved.path
          : null
        : isAbsoluteRemote(resolved.path)
          ? resolved.path
          : null;
    } catch {
      return null;
    }
  };

  const [resolvedLocal, resolvedRemote] = await Promise.all([
    resolve("local", localPath),
    resolve("remote", remotePath)
  ]);
  return { localPath: resolvedLocal, remotePath: resolvedRemote };
}

/** Prefill the save-as-favorite form from an active connection. */
export async function prefillFavorite(
  connectionId: string,
  localPath?: string | null,
  remotePath?: string | null
): Promise<{
  initial: ServerFavoriteInput;
  favoriteId?: string;
}> {
  const result = await ipc.getProfile(connectionId);
  const profile = result.profile;
  const resolved = await resolveFavoritePrefillPaths(connectionId, localPath, remotePath);
  return {
    initial: {
      displayName: profile.displayName,
      groupLabel: profile.groupLabel ?? null,
      protocol: profile.protocol,
      host: profile.host,
      port: profile.port,
      username: profile.username,
      authMethod: profile.authMethod,
      timeoutSeconds: profile.timeoutSeconds,
      sshKeyRef: profile.sshKeyRef ?? null,
      pinned: false,
      localPath: resolved.localPath,
      remotePath: resolved.remotePath
    },
    favoriteId: result.favoriteId
  };
}

export async function saveFavorite(
  profile: ServerFavoriteInput,
  policy: CredentialPolicy,
  secret?: string
): Promise<ServerFavoriteView> {
  return ipc.serverFavoritesCreate(profile, policy, undefined, secret);
}
