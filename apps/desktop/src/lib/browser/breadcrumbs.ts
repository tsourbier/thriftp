export type BreadcrumbSegment = {
  label: string;
  path: string;
  isCurrent: boolean;
  fullPath: string;
};

export type BreadcrumbOverflow = {
  visible: BreadcrumbSegment[];
  overflow: BreadcrumbSegment[];
};

function posixSegments(path: string): BreadcrumbSegment[] {
  const normalized = path === "" ? "/" : path;
  const parts = normalized.split("/").filter(Boolean);
  const segments: BreadcrumbSegment[] = [
    { label: "/", path: "/", isCurrent: parts.length === 0, fullPath: "/" }
  ];
  let current = "";
  parts.forEach((part, index) => {
    current += `/${part}`;
    segments.push({
      label: part,
      path: current,
      isCurrent: index === parts.length - 1,
      fullPath: current
    });
  });
  return segments;
}

function windowsSegments(path: string): BreadcrumbSegment[] {
  const match = /^([A-Za-z]:)([\\/])(.*)$/.exec(path);
  if (!match) {
    return posixSegments(path);
  }
  const [, drive, separator, rest] = match;
  const parts = rest.split(/[\\/]/).filter(Boolean);
  const root = `${drive}${separator}`;
  const segments: BreadcrumbSegment[] = [
    { label: root, path: root, isCurrent: parts.length === 0, fullPath: root }
  ];
  let current = root;
  parts.forEach((part, index) => {
    current =
      current.endsWith("\\") || current.endsWith("/")
        ? `${current}${part}`
        : `${current}\\${part}`;
    segments.push({
      label: part,
      path: current,
      isCurrent: index === parts.length - 1,
      fullPath: current
    });
  });
  return segments;
}

export function segmentsForPath(path: string): BreadcrumbSegment[] {
  if (/^[A-Za-z]:[\\/]/.test(path)) {
    return windowsSegments(path);
  }
  return posixSegments(path);
}

export function navigationTarget(
  segments: BreadcrumbSegment[],
  index: number
): string | null {
  return segments[index]?.path ?? null;
}

export function overflowSegments(
  segments: BreadcrumbSegment[],
  maxVisible = 5
): BreadcrumbOverflow {
  if (segments.length <= maxVisible) {
    return { visible: segments, overflow: [] };
  }
  const edge = Math.max(1, Math.floor((maxVisible - 1) / 2));
  const visible = [
    ...segments.slice(0, edge),
    ...segments.slice(segments.length - (maxVisible - edge))
  ];
  const overflow = segments.slice(edge, segments.length - (maxVisible - edge));
  return { visible, overflow };
}
