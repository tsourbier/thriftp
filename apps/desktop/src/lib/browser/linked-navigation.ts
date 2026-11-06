export type RelativeNavigation = {
  up: number;
  down: string[];
};

function splitPath(value: string): string[] {
  return value
    .replaceAll("\\", "/")
    .split("/")
    .filter((segment) => segment.length > 0 && segment !== ".");
}

function separatorFor(target: string): string {
  return target.includes("\\") && !target.includes("/") ? "\\" : "/";
}

type PathShape = {
  kind: "posix" | "drive" | "unc" | "relative";
  drive?: string;
};

function shapeOf(target: string): PathShape {
  if (/^[A-Za-z]:[\\/]/.test(target) || /^[A-Za-z]:$/.test(target)) {
    return { kind: "drive", drive: target.slice(0, 2) };
  }
  if (target.startsWith("\\\\") || target.startsWith("//")) {
    return { kind: "unc" };
  }
  if (target.startsWith("/") || target.startsWith("\\")) {
    return { kind: "posix" };
  }
  return { kind: "relative" };
}

export function relativeNavigation(from: string, to: string): RelativeNavigation | null {
  if (!from || !to || from === "." || to === ".") {
    return null;
  }
  const fromSegments = splitPath(from);
  const toSegments = splitPath(to);
  if (fromSegments.length === 0 || toSegments.length === 0) {
    return null;
  }
  let common = 0;
  while (
    common < fromSegments.length &&
    common < toSegments.length &&
    fromSegments[common] === toSegments[common]
  ) {
    common += 1;
  }
  return {
    up: fromSegments.length - common,
    down: toSegments.slice(common)
  };
}

export function applyRelativeNavigation(
  target: string,
  transition: RelativeNavigation
): string | null {
  if (!target || target === ".") {
    return null;
  }
  const segments = splitPath(target);
  if (transition.up > segments.length) {
    return null;
  }
  const next = [
    ...segments.slice(0, segments.length - transition.up),
    ...transition.down
  ];
  const separator = separatorFor(target);
  const shape = shapeOf(target);

  if (shape.kind === "drive") {
    const drive = shape.drive ?? `${next[0] ?? ""}`;
    if (next.length === 0) {
      return `${drive}\\`;
    }
    if (/^[A-Za-z]:$/.test(next[0])) {
      return next.length === 1 ? `${next[0]}\\` : next.join(separator);
    }
    return `${drive}\\${next.join(separator)}`;
  }
  if (next.length === 0) {
    return shape.kind === "relative" ? "." : separator;
  }
  if (shape.kind === "relative") {
    return next.join(separator);
  }
  if (shape.kind === "unc") {
    return `${separator}${separator}${next.join(separator)}`;
  }
  return `${separator}${next.join(separator)}`;
}
