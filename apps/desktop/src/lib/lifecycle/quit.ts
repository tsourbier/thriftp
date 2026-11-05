import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";

export type QuitPreparation = {
  activeTransfers: number;
  cancelledTransfers: number;
};

export async function prepareAndQuit() {
  let preparation: QuitPreparation | undefined;
  try {
    preparation = await invoke<QuitPreparation>("lifecycle_prepare_quit");
  } finally {
    await getCurrentWindow().close();
  }
  return preparation;
}
