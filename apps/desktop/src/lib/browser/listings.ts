import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type {
  DirectoryBatchPayload,
  DirectoryErrorPayload,
  ListingHandle
} from "../ipc/types";

export type ListingHandler = {
  onBatch: (batch: DirectoryBatchPayload) => void;
  onError: (error: DirectoryErrorPayload) => void;
};

type Buffered = {
  batches: DirectoryBatchPayload[];
  errors: DirectoryErrorPayload[];
};

const handlers = new Map<string, ListingHandler>();
const buffered = new Map<string, Buffered>();
let listeners: Promise<UnlistenFn[]> | null = null;

function hasTauri() {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

async function ensureListeners() {
  if (!hasTauri()) {
    return;
  }
  if (!listeners) {
    listeners = Promise.all([
      listen<DirectoryBatchPayload>("directory:batch", (event) => {
        const payload = event.payload;
        const handler = handlers.get(payload.listingId);
        if (handler) {
          handler.onBatch(payload);
        } else {
          const slot = buffered.get(payload.listingId) ?? { batches: [], errors: [] };
          slot.batches.push(payload);
          buffered.set(payload.listingId, slot);
        }
      }),
      listen<DirectoryErrorPayload>("directory:error", (event) => {
        const payload = event.payload;
        const handler = handlers.get(payload.listingId);
        if (handler) {
          handler.onError(payload);
        } else {
          const slot = buffered.get(payload.listingId) ?? { batches: [], errors: [] };
          slot.errors.push(payload);
          buffered.set(payload.listingId, slot);
        }
      })
    ]);
  }
  await listeners;
}

export async function openListing(
  start: () => Promise<ListingHandle>,
  handler: ListingHandler
): Promise<ListingHandle> {
  await ensureListeners();
  const handle = await start();
  const slot = buffered.get(handle.listingId);
  handlers.set(handle.listingId, handler);
  if (slot) {
    buffered.delete(handle.listingId);
    for (const batch of slot.batches) {
      handler.onBatch(batch);
    }
    for (const error of slot.errors) {
      handler.onError(error);
    }
  }
  return handle;
}

export function releaseListing(listingId: string) {
  handlers.delete(listingId);
  buffered.delete(listingId);
}
