<script lang="ts">
  import type {
    CredentialPolicy,
    ReusableCredential,
    ServerFavoriteInput,
    ServerFavoriteView
  } from "../../lib/ipc/types";
  import SaveFavoriteDialog from "./SaveFavoriteDialog.svelte";

  let {
    favorite,
    reusable = [],
    onsave,
    ontest,
    onremovecredential,
    oncancel
  }: {
    favorite: ServerFavoriteView;
    reusable?: ReusableCredential[];
    onsave: (
      profile: ServerFavoriteInput,
      policy: CredentialPolicy,
      secret?: string,
      credentialRefId?: string
    ) => void;
    onremovecredential?: () => void;
    ontest?: (
      profile: ServerFavoriteInput,
      secret?: string
    ) => Promise<{ ok: boolean; error?: { message?: string } | unknown }>;
    oncancel: () => void;
  } = $props();

  let dirty = $state(false);
  let confirmDiscard = $state(false);

  function requestCancel() {
    if (dirty) {
      confirmDiscard = true;
      return;
    }
    oncancel();
  }
</script>

<div class="edit-favorite">
  <p class="warning" role="note">
    Editing connection identity fields invalidates the current credential. Active
    connections keep their original settings.
  </p>
  <SaveFavoriteDialog
    initial={favorite}
    initialPolicy={favorite.credentialPolicy}
    title="Edit server favorite"
    {reusable}
    {onsave}
    {ontest}
    onchange={() => (dirty = true)}
    oncancel={requestCancel}
  />
  {#if favorite.hasSavedCredential && onremovecredential}
    <button type="button" class="remove-credential" onclick={onremovecredential}>
      Remove saved credential
    </button>
  {/if}
  {#if confirmDiscard}
    <div class="discard" role="alertdialog" aria-label="Discard unsaved changes">
      <p>Discard unsaved changes?</p>
      <div class="actions">
        <button type="button" onclick={oncancel}>Discard changes</button>
        <button type="button" onclick={() => (confirmDiscard = false)}>
          Keep editing
        </button>
      </div>
    </div>
  {/if}
</div>

<style>
  .edit-favorite {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    max-height: 85vh;
    min-height: 0;
  }
  .edit-favorite > :global(.save-favorite) {
    min-height: 0;
  }
  .warning {
    color: var(--server-favorites-warning, #9a6700);
  }
  .discard {
    border: 1px solid #d0a000;
    border-radius: 0.4rem;
    padding: 0.75rem;
    background: #fff8e1;
  }
  .discard p {
    margin: 0 0 0.5rem;
  }
  .actions {
    display: flex;
    gap: 0.5rem;
  }
  .remove-credential {
    justify-self: start;
    color: var(--server-favorites-error, #8a0000);
  }
</style>
