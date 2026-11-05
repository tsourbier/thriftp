import { ipc } from "../ipc/client";
import type {
  CredentialPolicy,
  ServerFavoriteInput,
  ServerFavoriteView
} from "../ipc/types";

/** Prefill the save-as-favorite form from an active connection. */
export async function prefillFavorite(connectionId: string): Promise<{
  initial: ServerFavoriteInput;
  favoriteId?: string;
}> {
  const result = await ipc.getProfile(connectionId);
  const profile = result.profile;
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
      pinned: false
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
