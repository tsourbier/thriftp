<script lang="ts">
  import type { BrowsingSession } from "../../lib/ipc/types";

  let {
    tabs,
    activeId,
    labelFor,
    onselect,
    onclose,
    onadd,
    connectedProfileIds = [],
    onretry,
    onreopen,
    canReopen = false
  }: {
    tabs: BrowsingSession[];
    activeId: string | null;
    labelFor: (tab: BrowsingSession) => string;
    onselect: (tab: BrowsingSession) => void;
    onclose: (tab: BrowsingSession) => void;
    onadd: () => void;
    connectedProfileIds?: string[];
    onretry?: (tab: BrowsingSession) => void;
    onreopen?: () => void;
    canReopen?: boolean;
  } = $props();
</script>

<div class="tabbar" role="tablist" aria-label="Browsing tabs">
  {#each tabs as tab (tab.id)}
    <div class="tab" class:active={tab.id === activeId} class:disconnected={tab.restored}>
      <button
        type="button"
        role="tab"
        aria-selected={tab.id === activeId}
        onclick={() => onselect(tab)}
        title={tab.currentPath}
      >
        {labelFor(tab)}
        {#if tab.restored && (!tab.connectionProfileId || !connectedProfileIds.includes(tab.connectionProfileId))}
          <span class="state">disconnected</span>
        {/if}
      </button>
      {#if tab.restored && onretry && (!tab.connectionProfileId || !connectedProfileIds.includes(tab.connectionProfileId))}
        <button
          type="button"
          class="retry"
          aria-label={`Retry ${labelFor(tab)}`}
          onclick={() => onretry(tab)}
        >
          Retry
        </button>
      {/if}
      <button
        type="button"
        class="close"
        aria-label={`Close ${labelFor(tab)}`}
        onclick={() => onclose(tab)}
      >
        ×
      </button>
    </div>
  {/each}
  <button type="button" class="add" onclick={onadd} aria-label="New tab">+</button>
  {#if canReopen && onreopen}
    <button type="button" class="add" onclick={onreopen}>Reopen</button>
  {/if}
</div>

<style>
  .tabbar {
    display: flex;
    align-items: stretch;
    gap: 2px;
    padding: 0.25rem 0.5rem 0;
    border-bottom: 1px solid #d0d0d0;
    overflow-x: auto;
  }
  .tab {
    display: flex;
    align-items: center;
    border: 1px solid #d0d0d0;
    border-bottom: none;
    border-radius: 0.35rem 0.35rem 0 0;
    background: color-mix(in srgb, CanvasText 4%, Canvas);
  }
  .tab.active {
    background: Canvas;
    font-weight: 600;
  }
  .tab.disconnected .state {
    color: #8a0000;
    font-size: 0.65rem;
    margin-left: 0.3rem;
  }
  .tab > button[role="tab"] {
    border: none;
    background: transparent;
    font: inherit;
    font-size: 0.8rem;
    padding: 0.3rem 0.5rem;
    max-width: 14rem;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .close {
    border: none;
    background: transparent;
    font: inherit;
    padding: 0 0.35rem;
    color: GrayText;
  }
  .add {
    font: inherit;
    margin-left: 0.25rem;
  }
</style>
