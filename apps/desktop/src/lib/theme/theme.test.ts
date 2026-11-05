import { describe, expect, it } from "vitest";
import { applyTheme, resolveTheme } from "./apply";

describe("theme", () => {
  it("resolves system themes", () => {
    expect(resolveTheme("system", true)).toBe("dark");
    expect(resolveTheme("system", false)).toBe("light");
    expect(resolveTheme("light", true)).toBe("light");
    expect(resolveTheme("dark", false)).toBe("dark");
  });

  it("applies the resolved theme to the root element", () => {
    const root = document.createElement("div");
    expect(applyTheme("dark", root)).toBe("dark");
    expect(root.dataset.theme).toBe("dark");
    expect(root.style.colorScheme).toBe("dark");
  });
});
