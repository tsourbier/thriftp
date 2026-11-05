<script lang="ts">
  import type {
    AuthMethod,
    CredentialPolicy,
    Protocol,
    ReusableCredential,
    ServerFavoriteInput,
    ServerFavoriteView
  } from "../../lib/ipc/types";
  import ReuseCredentialDialog from "./ReuseCredentialDialog.svelte";

  let {
    initial,
    initialPolicy = "ask",
    initialCredentialRefId,
    reusable = [],
    title = "Save server favorite",
    onsave,
    oncancel,
    onchange,
    ontest
  }: {
    initial?: Partial<ServerFavoriteView>;
    initialPolicy?: CredentialPolicy;
    initialCredentialRefId?: string;
    reusable?: ReusableCredential[];
    title?: string;
    onsave: (
      profile: ServerFavoriteInput,
      policy: CredentialPolicy,
      secret?: string,
      credentialRefId?: string
    ) => void;
    oncancel: () => void;
    onchange?: (dirty: boolean) => void;
    ontest?: (
      profile: ServerFavoriteInput,
      secret?: string
    ) => Promise<{ ok: boolean; error?: { message?: string } | unknown }>;
  } = $props();

  /* eslint-disable svelte/valid-compile */
  let displayName = $state(initial?.displayName ?? "");
  let protocol = $state<Protocol>(initial?.protocol ?? "sftp");
  let host = $state(initial?.host ?? "");
  let port = $state(initial?.port ?? 22);
  let username = $state(initial?.username ?? "");
  let groupLabel = $state(initial?.groupLabel ?? "");
  let authMethod = $state<AuthMethod>(initial?.authMethod ?? "password");
  let timeoutSeconds = $state(initial?.timeoutSeconds ?? 30);
  let sshKeyRef = $state(initial?.sshKeyRef ?? "");
  let pinned = $state(initial?.pinned ?? false);
  let policy = $state<CredentialPolicy>(initialPolicy);
  let credentialRefId = $state(initialCredentialRefId ?? "");
  /* eslint-enable svelte/valid-compile */
  let secret = $state("");
  let testSecret = $state("");
  let showReuseDialog = $state(false);
  let validationError = $state<string | null>(null);
  let testResult = $state<{ ok: boolean; message: string } | null>(null);
  let testing = $state(false);

  const passwordOnly = $derived(
    protocol === "ftp" || protocol === "ftpsExplicit" || protocol === "ftpsImplicit"
  );
  const usesPrivateKey = $derived(!passwordOnly && authMethod === "sshPrivateKey");
  const hasSavedCredential = $derived(Boolean(initial?.hasSavedCredential));
  const compatibleCredentials = $derived(
    reusable.filter(
      (credential) =>
        credential.protocol === protocol &&
        credential.host === host &&
        credential.port === port &&
        credential.username === username &&
        credential.authMethod === (passwordOnly ? "password" : authMethod)
    )
  );

  function markChanged() {
    onchange?.(true);
    validationError = null;
    testResult = null;
  }

  function identityChanged(): boolean {
    if (!initial) {
      return false;
    }
    return (
      initial.protocol !== protocol ||
      initial.host !== host ||
      initial.port !== port ||
      initial.username !== username ||
      initial.authMethod !== authMethod
    );
  }

  function currentProfile(): ServerFavoriteInput {
    return {
      displayName,
      groupLabel: groupLabel || null,
      protocol,
      host,
      port,
      username,
      authMethod: passwordOnly ? "password" : authMethod,
      timeoutSeconds,
      sshKeyRef: usesPrivateKey ? sshKeyRef || null : null,
      pinned
    };
  }

  function validate(): string | null {
    if (!displayName.trim()) {
      return "Name is required.";
    }
    if (!host.trim()) {
      return "Host is required.";
    }
    if (!username.trim()) {
      return "Username is required.";
    }
    if (usesPrivateKey && !sshKeyRef.trim()) {
      return "A private-key reference is required for SSH key authentication.";
    }
    if (
      initial &&
      hasSavedCredential &&
      identityChanged() &&
      policy === "save" &&
      !secret &&
      !credentialRefId
    ) {
      return "Connection identity changed. Enter a new credential or choose “Ask when connecting” to clear the saved credential.";
    }
    return null;
  }

  function submit() {
    const error = validate();
    if (error) {
      validationError = error;
      return;
    }
    onsave(
      currentProfile(),
      policy,
      policy === "save" && !credentialRefId && secret ? secret : undefined,
      policy === "save" ? credentialRefId || undefined : undefined
    );
  }

  async function runTest() {
    if (!ontest) {
      return;
    }
    const error = validate();
    if (error) {
      validationError = error;
      return;
    }
    testing = true;
    testResult = null;
    try {
      const result = await ontest(currentProfile(), secret || testSecret || undefined);
      if (result.ok) {
        testResult = { ok: true, message: "Connection test succeeded." };
      } else {
        const failure = result.error as { message?: string } | undefined;
        testResult = {
          ok: false,
          message: failure?.message ?? "Connection test failed."
        };
      }
    } catch (error) {
      testResult = {
        ok: false,
        message: error instanceof Error ? error.message : "Connection test failed."
      };
    } finally {
      testing = false;
    }
  }
</script>

<form
  class="save-favorite"
  oninput={markChanged}
  onchange={markChanged}
  onsubmit={(event) => {
    event.preventDefault();
    submit();
  }}
>
  <h2>{title}</h2>
  <label>Name <input bind:value={displayName} required maxlength="200" /></label>
  <label
    >Protocol
    <select bind:value={protocol}>
      <option value="ftp">FTP</option>
      <option value="ftpsExplicit">FTPS explicit</option>
      <option value="ftpsImplicit">FTPS implicit</option>
      <option value="sftp">SFTP</option>
    </select>
  </label>
  <label>Host <input bind:value={host} required maxlength="255" /></label>
  <label>Port <input type="number" bind:value={port} min="1" max="65535" /></label>
  <label>Username <input bind:value={username} required /></label>
  <label>Group <input bind:value={groupLabel} maxlength="100" /></label>
  <label>
    Authentication
    <select bind:value={authMethod} disabled={passwordOnly}>
      <option value="password">Password</option>
      {#if !passwordOnly}
        <option value="sshPrivateKey">SSH private key</option>
      {/if}
    </select>
  </label>
  {#if usesPrivateKey}
    <label>
      SSH key reference
      <input bind:value={sshKeyRef} placeholder="~/.ssh/id_ed25519" />
    </label>
  {/if}
  <label
    >Timeout <input type="number" bind:value={timeoutSeconds} min="1" max="300" /></label
  >
  <label class="checkbox">
    <input type="checkbox" bind:checked={pinned} />
    Pin to the top
  </label>
  <label
    >Credential policy
    <select bind:value={policy}>
      <option value="ask">Ask when connecting</option>
      <option value="save">Save securely</option>
    </select>
  </label>
  {#if policy === "save" && compatibleCredentials.length > 0}
    <label>
      Reuse saved credential
      <select bind:value={credentialRefId}>
        <option value="">Use a new credential</option>
        {#each compatibleCredentials as credential (credential.credentialRefId)}
          <option value={credential.credentialRefId}>
            {credential.username}@{credential.host}:{credential.port}
            ({credential.affectedFavoriteCount} favorites)
          </option>
        {/each}
      </select>
    </label>
    <button type="button" onclick={() => (showReuseDialog = true)}>
      Browse reusable credentials
    </button>
  {/if}
  {#if showReuseDialog}
    <div class="nested-dialog">
      <ReuseCredentialDialog
        credentials={compatibleCredentials}
        onselect={(credentialRef) => {
          credentialRefId = credentialRef;
          showReuseDialog = false;
          markChanged();
        }}
        oncancel={() => (showReuseDialog = false)}
      />
    </div>
  {/if}
  {#if policy === "save" && !credentialRefId}
    <label>
      Password
      <input
        type="password"
        bind:value={secret}
        placeholder={hasSavedCredential ? "Leave blank to keep the saved credential" : ""}
      />
    </label>
  {/if}
  {#if ontest}
    <label>
      Test credential
      <input
        type="password"
        bind:value={testSecret}
        placeholder="Used only for the test and never saved"
      />
    </label>
  {/if}
  {#if validationError}
    <p class="error" role="alert">{validationError}</p>
  {/if}
  {#if testResult}
    <p class:error={!testResult.ok} role="status">{testResult.message}</p>
  {/if}
  <div class="actions">
    {#if ontest}
      <button type="button" disabled={testing} onclick={runTest}>
        {testing ? "Testing…" : "Test connection"}
      </button>
    {/if}
    <button type="submit">Save</button>
    <button type="button" onclick={oncancel}>Cancel</button>
  </div>
</form>

<style>
  .save-favorite {
    display: grid;
    gap: 0.5rem;
    padding: 1rem;
  }
  label {
    display: grid;
    gap: 0.2rem;
  }
  .checkbox {
    display: flex;
    gap: 0.4rem;
    align-items: center;
  }
  .actions {
    display: flex;
    flex-wrap: wrap;
    gap: 0.5rem;
  }
  .error {
    color: var(--server-favorites-error, #8a0000);
  }
  .nested-dialog {
    border: 1px solid var(--server-favorites-border, #d0d0d0);
    border-radius: 0.4rem;
    padding: 0.5rem;
  }
</style>
