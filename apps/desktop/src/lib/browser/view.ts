import type { FileEntryView, SortKey } from "../ipc/types";

export function sortEntries(entries: FileEntryView[], key: SortKey): FileEntryView[] {
  return [...entries].sort((left, right) => {
    let comparison: number;
    switch (key) {
      case "size":
        comparison = (left.size ?? 0) - (right.size ?? 0);
        break;
      case "date":
        comparison =
          new Date(left.modifiedAt ?? 0).getTime() -
          new Date(right.modifiedAt ?? 0).getTime();
        break;
      case "type":
        comparison = left.entryType.localeCompare(right.entryType);
        break;
      default:
        comparison = left.name.localeCompare(right.name, undefined, {
          sensitivity: "base"
        });
    }
    return comparison || left.name.localeCompare(right.name);
  });
}

export function filterEntries(
  entries: FileEntryView[],
  options: { showHidden: boolean; filter: string }
): FileEntryView[] {
  const query = options.filter.trim().toLowerCase();
  return entries.filter((entry) => {
    if (!options.showHidden && entry.hidden) {
      return false;
    }
    return query.length === 0 || entry.name.toLowerCase().includes(query);
  });
}

export function visibleEntries(
  entries: FileEntryView[],
  options: { sortKey: SortKey; showHidden: boolean; filter: string }
): FileEntryView[] {
  return filterEntries(sortEntries(entries, options.sortKey), options);
}
