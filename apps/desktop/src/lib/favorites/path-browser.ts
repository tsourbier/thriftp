import { ipc } from "../ipc/client";

function isAbsolute(path: string): boolean {
  return path.startsWith("/") || /^[A-Za-z]:[\\/]/.test(path);
}

export async function browseLocalFolder(current: string): Promise<string | null> {
  const [{ open }, { homeDir }] = await Promise.all([
    import("@tauri-apps/plugin-dialog"),
    import("@tauri-apps/api/path")
  ]);
  let defaultPath = await homeDir();
  const value = current.trim();
  if (value && isAbsolute(value)) {
    try {
      const entry = await ipc.stat("local", value);
      if (entry.entryType === "directory") {
        defaultPath = value;
      }
    } catch {
      // Fall back to the home directory.
    }
  }
  const selected = await open({
    directory: true,
    multiple: false,
    title: "Select local folder",
    defaultPath
  });
  return typeof selected === "string" ? selected : null;
}
