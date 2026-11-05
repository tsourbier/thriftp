import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { QueueView, TransferProgressEvent, TransferStateEvent } from "../ipc/types";

export type TransferEventHandlers = {
  onProgress: (event: TransferProgressEvent) => void;
  onState: (event: TransferStateEvent) => void;
  onQueue: (queue: QueueView) => void;
};

function hasTauri() {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

export async function subscribeTransferEvents(
  handlers: TransferEventHandlers
): Promise<() => void> {
  if (!hasTauri()) {
    return () => undefined;
  }
  const unlisten: UnlistenFn[] = await Promise.all([
    listen<TransferProgressEvent>("transfer:progress", (event) =>
      handlers.onProgress(event.payload)
    ),
    listen<TransferStateEvent>("transfer:state", (event) =>
      handlers.onState(event.payload)
    ),
    listen<QueueView>("queue:state", (event) => handlers.onQueue(event.payload))
  ]);
  return () => {
    for (const stop of unlisten) {
      stop();
    }
  };
}
