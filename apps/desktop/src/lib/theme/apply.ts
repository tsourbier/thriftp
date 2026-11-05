export type Theme = "system" | "light" | "dark";
export type ResolvedTheme = "light" | "dark";

export function resolveTheme(theme: Theme, prefersDark: boolean): ResolvedTheme {
  if (theme === "system") {
    return prefersDark ? "dark" : "light";
  }
  return theme;
}

export function applyTheme(theme: Theme, root: HTMLElement = document.documentElement) {
  const prefersDark =
    typeof window !== "undefined" &&
    typeof window.matchMedia === "function" &&
    window.matchMedia("(prefers-color-scheme: dark)").matches;
  const resolved = resolveTheme(theme, prefersDark);
  root.dataset.theme = resolved;
  root.style.colorScheme = resolved;
  return resolved;
}
