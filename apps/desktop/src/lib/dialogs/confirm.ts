export async function confirmAction(
  message: string,
  title = "Confirm"
): Promise<boolean> {
  try {
    const { ask } = await import("@tauri-apps/plugin-dialog");
    return await ask(message, { title, kind: "warning" });
  } catch {
    if (typeof window !== "undefined" && typeof window.confirm === "function") {
      return window.confirm(message);
    }
    return true;
  }
}
