<script lang="ts">
  import { onMount } from "svelte";
  import { ipc } from "../../lib/ipc/client";
  import type { SshKeyCandidate, SshKeyValidation } from "../../lib/ipc/types";

  let {
    value = $bindable(""),
    id = "ssh-key",
    label = "SSH key reference",
    onchange
  }: {
    value?: string;
    id?: string;
    label?: string;
    onchange?: (value: string) => void;
  } = $props();

  let candidates = $state<SshKeyCandidate[]>([]);
  let directory = $state("~/.ssh");
  let validation = $state<SshKeyValidation | null>(null);
  let browseError = $state<string | null>(null);

  onMount(() => {
    void loadCandidates();
  });

  async function loadCandidates() {
    try {
      const listing = await ipc.sshKeysList();
      candidates = listing.candidates;
      directory = listing.directory;
      if (!value && candidates.length === 1) {
        setValue(candidates[0].path);
      }
    } catch {
      candidates = [];
    }
  }

  function setValue(next: string) {
    value = next;
    onchange?.(next);
    void validate();
  }

  async function validate() {
    if (!value.trim()) {
      validation = null;
      return;
    }
    try {
      validation = await ipc.sshKeysValidate(value.trim());
    } catch (error) {
      validation = {
        valid: false,
        code: "unsupported",
        message: error instanceof Error ? error.message : String(error)
      };
    }
  }

  async function browse() {
    browseError = null;
    try {
      const { open } = await import("@tauri-apps/plugin-dialog");
      const selected = await open({
        directory: false,
        multiple: false,
        title: "Select SSH private key",
        defaultPath: directory
      });
      if (typeof selected === "string") {
        setValue(selected);
      }
    } catch {
      browseError =
        "The native file chooser is unavailable. Enter the key path manually.";
    }
  }
</script>

<div class="ssh-key-field">
  <label for={id}>{label}</label>
  <div class="row">
    <input
      {id}
      bind:value
      onblur={validate}
      oninput={() => (validation = null)}
      autocomplete="off"
      spellcheck="false"
      placeholder="~/.ssh/id_ed25519"
    />
    <button
      type="button"
      class="key-button"
      onclick={browse}
      title="Browse for a private key"
      aria-label="Browse for a private key"
    >
      🔑
    </button>
  </div>
  {#if candidates.length > 0}
    <div class="suggestions" aria-label="Discovered SSH keys">
      {#each candidates as candidate (candidate.path)}
        <button type="button" class="suggestion" onclick={() => setValue(candidate.path)}>
          {candidate.fileName}
        </button>
      {/each}
    </div>
  {/if}
  {#if validation && !validation.valid}
    <p class="validation" role="alert">{validation.message}</p>
  {/if}
  {#if browseError}
    <p class="validation" role="alert">{browseError}</p>
  {/if}
</div>

<style>
  .ssh-key-field {
    display: grid;
    gap: 0.3rem;
  }
  label {
    font-size: 0.85rem;
  }
  .row {
    display: flex;
    gap: 0.35rem;
  }
  input {
    flex: 1;
    min-width: 0;
    padding: 0.35rem;
    font: inherit;
  }
  .key-button {
    padding: 0.25rem 0.5rem;
    font: inherit;
    cursor: pointer;
  }
  .suggestions {
    display: flex;
    flex-wrap: wrap;
    gap: 0.3rem;
  }
  .suggestion {
    font-size: 0.78rem;
    padding: 0.15rem 0.4rem;
    cursor: pointer;
  }
  .validation {
    margin: 0;
    color: var(--danger-text);
    font-size: 0.8rem;
  }
</style>
