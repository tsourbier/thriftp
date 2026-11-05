<script lang="ts">
  import { onMount, untrack } from "svelte";
  import { ipc } from "../../lib/ipc/client";
  import type { BrowserSource } from "../../lib/ipc/types";

  let {
    source,
    connectionId = null,
    path,
    name,
    onclose
  }: {
    source: BrowserSource;
    connectionId?: string | null;
    path: string;
    name: string;
    onclose: () => void;
  } = $props();

  let state = $state<"idle" | "loading" | "text" | "image" | "metadata" | "error">(
    "idle"
  );
  let content = $state("");
  let message = $state("");
  let truncated = $state(false);
  let previewId = $state<string | null>(null);
  let consentNeeded = $state(untrack(() => source === "remote"));

  onMount(() => {
    if (source === "local") {
      void openPreview(false);
    }
  });

  async function openPreview(allowDownload: boolean) {
    consentNeeded = false;
    state = "loading";
    try {
      const result = await ipc.previewOpen(
        source,
        path,
        connectionId ?? undefined,
        allowDownload
      );
      previewId = result.previewId;
      if (result.mode === "metadata") {
        message = result.reason ?? "Only file metadata is available for this file.";
        state = "metadata";
        return;
      }
      if (result.mode === "native") {
        message = "Opened with the operating system's default application.";
        state = "metadata";
        return;
      }
      const preview = await ipc.previewRead(source, path, previewId ?? undefined);
      content = preview.content;
      truncated = preview.truncated;
      state = preview.kind;
    } catch (error) {
      message = error instanceof Error ? error.message : String(error);
      state = "error";
    }
  }

  async function close() {
    if (previewId) {
      try {
        await ipc.previewClose(previewId);
      } catch {
        // Closing a preview must never block the UI.
      }
    }
    onclose();
  }

  function mimeType() {
    const extension = name.split(".").pop()?.toLowerCase() ?? "";
    switch (extension) {
      case "png":
        return "image/png";
      case "jpg":
      case "jpeg":
        return "image/jpeg";
      case "gif":
        return "image/gif";
      case "webp":
        return "image/webp";
      case "bmp":
        return "image/bmp";
      case "svg":
        return "image/svg+xml";
      default:
        return "application/octet-stream";
    }
  }
</script>

<div class="preview" role="dialog" aria-modal="true" aria-label="File preview">
  <header>
    <h2>{name}</h2>
    <button type="button" onclick={close} aria-label="Close preview">Close</button>
  </header>

  {#if consentNeeded}
    <p>
      This file is on the remote server. Previewing it downloads the file temporarily and
      is limited to 25 MB.
    </p>
    <button type="button" onclick={() => openPreview(true)}>Download for preview</button>
  {:else if state === "loading"}
    <p role="status">Loading preview…</p>
  {:else if state === "text"}
    {#if truncated}
      <p class="notice" role="status">Preview truncated at the inline size limit.</p>
    {/if}
    <pre>{content}</pre>
  {:else if state === "image"}
    <img src={`data:${mimeType()};base64,${content}`} alt={name} />
  {:else if state === "metadata" || state === "error"}
    <p class:error={state === "error"} role={state === "error" ? "alert" : "status"}>
      {message}
    </p>
  {/if}
</div>

<style>
  .preview {
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
    width: min(70rem, 90vw);
    max-height: 85vh;
  }
  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 1rem;
  }
  h2 {
    font-size: 1rem;
    margin: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  pre {
    margin: 0;
    padding: 0.5rem;
    overflow: auto;
    background: color-mix(in srgb, CanvasText 6%, Canvas);
    max-height: 60vh;
    font-size: 0.8rem;
  }
  img {
    max-width: 100%;
    max-height: 60vh;
    object-fit: contain;
  }
  .notice {
    color: GrayText;
    font-size: 0.8rem;
  }
  .error {
    color: #8a0000;
  }
</style>
