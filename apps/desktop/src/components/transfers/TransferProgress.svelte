<script lang="ts">
  import type { TransferView } from "../../lib/ipc/types";

  let {
    transfer,
    onpause,
    onresume,
    oncancel,
    ondismiss
  }: {
    transfer: TransferView;
    onpause: (transfer: TransferView) => void;
    onresume: (transfer: TransferView) => void;
    oncancel: (transfer: TransferView) => void;
    ondismiss?: (transfer: TransferView) => void;
  } = $props();

  const percent = $derived(
    transfer.totalBytes && transfer.totalBytes > 0
      ? Math.min(100, Math.round((transfer.transferredBytes / transfer.totalBytes) * 100))
      : null
  );

  function sizeLabel(bytes: number | null) {
    if (bytes === null) {
      return "unknown size";
    }
    const units = ["B", "KB", "MB", "GB", "TB"];
    let value = bytes;
    let unit = 0;
    while (value >= 1024 && unit < units.length - 1) {
      value /= 1024;
      unit += 1;
    }
    return `${unit === 0 ? value : value.toFixed(1)} ${units[unit]}`;
  }

  function durationLabel(seconds: number | null) {
    if (seconds === null) {
      return "";
    }
    if (seconds < 60) {
      return `${seconds}s left`;
    }
    return `${Math.ceil(seconds / 60)}m left`;
  }

  function stateLabel() {
    switch (transfer.state) {
      case "active":
        return `${sizeLabel(transfer.transferredBytes)} of ${sizeLabel(transfer.totalBytes)}`;
      case "completed":
        return "Completed";
      case "paused":
        return "Paused";
      case "failed":
        return transfer.error ?? "Failed";
      case "cancelled":
        return "Cancelled";
      case "interrupted":
        return "Interrupted";
      default:
        return "Queued";
    }
  }
</script>

<article class="transfer" aria-label={`Transfer ${transfer.sourcePath}`}>
  <header>
    <span class="name" title={transfer.sourcePath}>
      {transfer.direction === "upload" ? "↑" : "↓"}
      {transfer.destinationPath.split(/[\\/]/).pop()}
    </span>
    <span class="state {transfer.state}">{stateLabel()}</span>
  </header>

  <div
    class="bar"
    role="progressbar"
    aria-valuemin="0"
    aria-valuemax={transfer.totalBytes ?? 100}
    aria-valuenow={transfer.transferredBytes}
    aria-label="Transfer progress"
  >
    <div class="fill" style={`width: ${percent ?? 0}%`}></div>
  </div>

  <footer>
    <span class="meta">
      {#if transfer.state === "active"}
        {sizeLabel(transfer.speedBytesPerSecond)}/s
        {transfer.estimatedRemainingSeconds !== null
          ? ` · ${durationLabel(transfer.estimatedRemainingSeconds)}`
          : ""}
      {:else if percent !== null}
        {percent}%
      {/if}
    </span>
    <span class="actions">
      {#if transfer.state === "active"}
        <button type="button" onclick={() => onpause(transfer)}>Pause</button>
      {:else if transfer.state === "paused" || transfer.state === "interrupted" || transfer.state === "failed"}
        <button type="button" onclick={() => onresume(transfer)}>Resume</button>
      {/if}
      {#if transfer.state !== "completed" && transfer.state !== "cancelled"}
        <button type="button" onclick={() => oncancel(transfer)}>Cancel</button>
      {/if}
      {#if ["completed", "failed", "cancelled", "interrupted"].includes(transfer.state) && ondismiss}
        <button type="button" onclick={() => ondismiss(transfer)}>Dismiss</button>
      {/if}
    </span>
  </footer>
</article>

<style>
  .transfer {
    display: grid;
    gap: 0.3rem;
    padding: 0.5rem;
    border: 1px solid #e0e0e0;
    border-radius: 0.4rem;
  }
  header,
  footer {
    display: flex;
    justify-content: space-between;
    gap: 0.5rem;
    font-size: 0.75rem;
  }
  .name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-weight: 600;
  }
  .state {
    color: GrayText;
    white-space: nowrap;
  }
  .state.failed {
    color: #8a0000;
  }
  .bar {
    height: 0.4rem;
    border-radius: 0.2rem;
    background: color-mix(in srgb, CanvasText 12%, Canvas);
    overflow: hidden;
  }
  .fill {
    height: 100%;
    background: Highlight;
    transition: width 120ms linear;
  }
  .meta {
    color: GrayText;
  }
  .actions {
    display: flex;
    gap: 0.25rem;
  }
  button {
    font: inherit;
    font-size: 0.7rem;
  }
</style>
