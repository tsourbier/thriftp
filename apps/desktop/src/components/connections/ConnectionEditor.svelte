<script lang="ts">
  import { untrack } from "svelte";
  import SshKeyField from "./SshKeyField.svelte";
  import type {
    AuthMethod,
    ConnectionProfile,
    ConnectionProfileInput,
    Protocol
  } from "../../lib/ipc/types";

  let {
    profile,
    onsave,
    oncancel
  }: {
    profile?: ConnectionProfile | null;
    onsave: (profile: ConnectionProfile, secret?: string) => void;
    oncancel: () => void;
  } = $props();

  const defaultPorts: Record<Protocol, number> = {
    ftp: 21,
    ftpsExplicit: 21,
    ftpsImplicit: 990,
    sftp: 22
  };

  let displayName = $state(untrack(() => profile?.displayName ?? ""));
  let groupLabel = $state(untrack(() => profile?.groupLabel ?? ""));
  let protocol = $state<Protocol>(untrack(() => profile?.protocol ?? "sftp"));
  let host = $state(untrack(() => profile?.host ?? ""));
  let port = $state(untrack(() => profile?.port ?? defaultPorts[protocol]));
  let username = $state(untrack(() => profile?.username ?? ""));
  let authMethod = $state<AuthMethod>(untrack(() => profile?.authMethod ?? "password"));
  let timeoutSeconds = $state(untrack(() => profile?.timeoutSeconds ?? 30));
  let sshKeyRef = $state(untrack(() => profile?.sshKeyRef ?? ""));
  let secret = $state("");
  let previousProtocol = untrack(() => protocol);

  function changeProtocol(value: Protocol) {
    if (port === defaultPorts[previousProtocol]) {
      port = defaultPorts[value];
    }
    previousProtocol = value;
    if (value !== "sftp" && authMethod === "sshPrivateKey") {
      authMethod = "password";
    }
  }

  function submit(event: SubmitEvent) {
    event.preventDefault();
    const now = new Date().toISOString();
    const input: ConnectionProfileInput = {
      displayName: displayName.trim(),
      groupLabel: groupLabel.trim() || null,
      protocol,
      host: host.trim(),
      port,
      username: username.trim(),
      authMethod,
      timeoutSeconds,
      sshKeyRef: authMethod === "sshPrivateKey" ? sshKeyRef.trim() : null
    };
    onsave(
      {
        ...input,
        id: profile?.id ?? crypto.randomUUID(),
        createdAt: profile?.createdAt ?? now,
        updatedAt: now,
        lastUsedAt: profile?.lastUsedAt ?? null
      },
      secret || undefined
    );
  }

  const canSubmit = $derived(
    displayName.trim().length > 0 &&
      host.trim().length > 0 &&
      username.trim().length > 0 &&
      (authMethod !== "sshPrivateKey" || sshKeyRef.trim().length > 0)
  );
</script>

<form class="editor" onsubmit={submit}>
  <h2>{profile ? "Edit connection" : "New connection"}</h2>

  <label>
    Name
    <input bind:value={displayName} required maxlength="200" />
  </label>

  <label>
    Group
    <input bind:value={groupLabel} maxlength="100" placeholder="Optional" />
  </label>

  <label>
    Protocol
    <select
      bind:value={protocol}
      onchange={(event) => changeProtocol(event.currentTarget.value as Protocol)}
    >
      <option value="ftp">FTP</option>
      <option value="ftpsExplicit">FTPS (explicit)</option>
      <option value="ftpsImplicit">FTPS (implicit)</option>
      <option value="sftp">SFTP</option>
    </select>
  </label>

  <label>
    Host
    <input bind:value={host} required maxlength="255" />
  </label>

  <label>
    Port
    <input type="number" bind:value={port} min="1" max="65535" required />
  </label>

  <label>
    Username
    <input bind:value={username} required maxlength="255" />
  </label>

  <label>
    Authentication
    <select bind:value={authMethod}>
      <option value="password">Password</option>
      {#if protocol === "sftp"}
        <option value="sshPrivateKey">SSH private key</option>
      {/if}
    </select>
  </label>

  {#if authMethod === "sshPrivateKey"}
    <SshKeyField bind:value={sshKeyRef} />
  {/if}

  <label>
    {authMethod === "sshPrivateKey" ? "Key passphrase" : "Password"}
    <input
      type="password"
      bind:value={secret}
      autocomplete="new-password"
      placeholder={profile
        ? "Leave blank to keep the saved credential"
        : "Optional; saved in the OS keychain"}
    />
  </label>

  <label>
    Timeout (seconds)
    <input type="number" bind:value={timeoutSeconds} min="1" max="300" required />
  </label>

  <div class="actions">
    <button type="button" onclick={oncancel}>Cancel</button>
    <button type="submit" disabled={!canSubmit}>{profile ? "Save" : "Create"}</button>
  </div>
</form>

<style>
  .editor {
    display: grid;
    gap: 0.6rem;
    padding: 1rem;
    border: 1px solid #d0d0d0;
    border-radius: 0.5rem;
    background: Canvas;
    max-height: 90vh;
    overflow: auto;
  }
  h2 {
    font-size: 1rem;
    margin: 0;
  }
  label {
    display: grid;
    gap: 0.25rem;
    font-size: 0.85rem;
  }
  input,
  select {
    padding: 0.35rem;
    font: inherit;
  }
  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 0.5rem;
  }
</style>
