import type { DirectoryFavorite, ServerFavoriteView } from "../ipc/types";

export type SidebarFilterInput = {
  servers: ServerFavoriteView[];
  directoryFavorites: DirectoryFavorite[];
  query: string;
};

export type SidebarFilterResult = {
  servers: ServerFavoriteView[];
  directoryFavorites: DirectoryFavorite[];
};

function norm(value: string | null | undefined): string {
  return (value ?? "").toLocaleLowerCase();
}

export function matchesServer(favorite: ServerFavoriteView, query: string): boolean {
  const needle = norm(query).trim();
  if (!needle) {
    return true;
  }
  return [
    favorite.displayName,
    favorite.host,
    favorite.protocol,
    favorite.groupLabel,
    favorite.username,
    favorite.remotePath,
    favorite.localPath
  ].some((value) => norm(value).includes(needle));
}

export function matchesDirectoryFavorite(
  favorite: DirectoryFavorite,
  query: string
): boolean {
  const needle = norm(query).trim();
  if (!needle) {
    return true;
  }
  return [favorite.label, favorite.path].some((value) => norm(value).includes(needle));
}

export function filterSidebar({
  servers,
  directoryFavorites,
  query
}: SidebarFilterInput): SidebarFilterResult {
  return {
    servers: servers.filter((favorite) => matchesServer(favorite, query)),
    directoryFavorites: directoryFavorites.filter((favorite) =>
      matchesDirectoryFavorite(favorite, query)
    )
  };
}
