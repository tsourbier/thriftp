<script lang="ts">
  import type { ConnectionProfile, ConnectionView } from "../../lib/ipc/types";

  let {
    profiles,
    active,
    onconnect,
    ondisconnect,
    onreconnect,
    onedit,
    onduplicate,
    ondelete
  }: {
    profiles: ConnectionProfile[];
    active: ConnectionView[];
    onconnect: (profile: ConnectionProfile) => void;
    ondisconnect: (connection: ConnectionView) => void;
    onreconnect: (connection: ConnectionView) => void;
    onedit: (profile: ConnectionProfile) => void;
    onduplicate: (profile: ConnectionProfile) => void;
    ondelete: (profile: ConnectionProfile) => void;
  } = $props();

  function activeFor(profile: ConnectionProfile) {
    return active.filter((connection) => connection.profileId === profile.id);
  }

  function protocolLabel(protocol: ConnectionProfile["protocol"]) {
    switch (protocol) {
      case "ftp":
        return "FTP";
      case "ftpsExplicit":
        return "FTPS (explicit)";
      case "ftpsImplicit":
        return "FTPS (implicit)";
      case "sftp":
        return "SFTP";
    }
  }
</script>

<section class="connections" aria-label="Saved connections">
  <h2>Connections</h2>
  {#if profiles.length === 0}
    <p class="empty">No saved connections yet.</p>
  {:else}
    <ul>
      {#each profiles as profile (profile.id)}
        {@const connections = activeFor(profile)}
        <li>
          <div class="details">
            <strong>{profile.displayName}</strong>
            <span
              >{protocolLabel(profile.protocol)} · {profile.host}:{profile.port} · {profile.username}</span
            >
            {#if connections.length === 0}
              <span class="state disconnected">Disconnected</span>
            {/if}
            {#each connections as connection (connection.connectionId)}
              <span class="state {connection.state}">
                {connection.state}
                {#if connection.error}
                  — {connection.error.message}
                {/if}
              </span>
              {#if connection.transportWarning}
                <span class="warning" role="status"
                  >{connection.transportWarning.message}</span
                >
              {/if}
            {/each}
          </div>
          <div class="actions">
            <button onclick={() => onconnect(profile)}>Connect</button>
            {#each connections as connection (connection.connectionId)}
              {#if connection.state === "error"}
                <button onclick={() => onreconnect(connection)}>Retry</button>
              {:else}
                <button onclick={() => ondisconnect(connection)}>Disconnect</button>
              {/if}
            {/each}
            <button onclick={() => onedit(profile)}>Edit</button>
            <button onclick={() => onduplicate(profile)}>Duplicate</button>
            <button class="danger" onclick={() => ondelete(profile)}>Delete</button>
          </div>
        </li>
      {/each}
    </ul>
  {/if}
</section>

<style>
  .connections {
    padding: 1rem;
    min-width: 18rem;
    border-right: 1px solid #d0d0d0;
    overflow: auto;
  }
  h2 {
    font-size: 0.9rem;
    margin: 0 0 0.75rem;
  }
  ul {
    list-style: none;
    padding: 0;
    margin: 0;
    display: grid;
    gap: 0.5rem;
  }
  li {
    display: flex;
    justify-content: space-between;
    gap: 0.75rem;
    padding: 0.5rem;
    border: 1px solid #e0e0e0;
    border-radius: 0.4rem;
  }
  .details {
    display: grid;
    gap: 0.15rem;
    min-width: 0;
  }
  .details span {
    font-size: 0.75rem;
    color: GrayText;
  }
  .state {
    font-weight: 600;
    color: #1a5c1a;
  }
  .state.disconnected {
    color: GrayText;
  }
  .state.error {
    color: #8a0000;
  }
  .warning {
    color: #8a5a00;
    font-size: 0.7rem;
    font-weight: 600;
  }
  .actions {
    display: flex;
    flex-wrap: wrap;
    gap: 0.35rem;
    align-items: start;
    justify-content: flex-end;
  }
  button {
    font: inherit;
  }
  .danger {
    color: #8a0000;
  }
  .empty {
    font-size: 0.85rem;
    color: GrayText;
  }
</style>
