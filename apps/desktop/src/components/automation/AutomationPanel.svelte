<script lang="ts">
  import type {
    ApprovalInput,
    AuditEventView,
    AutomationStatus,
    EnrollmentRequestView
  } from "../../lib/automation/types";
  import AuditView from "./AuditView.svelte";
  import ClientList from "./ClientList.svelte";
  import PendingApprovalDialog from "./PendingApprovalDialog.svelte";

  let {
    status,
    audit = [],
    auditValid = null,
    onenable,
    ondisable,
    onapprove,
    ondeny,
    onrevoke,
    ondisconnect
  }: {
    status: AutomationStatus | null;
    audit?: AuditEventView[];
    auditValid?: boolean | null;
    onenable: (mode: string) => void;
    ondisable: () => void;
    onapprove: (input: ApprovalInput) => void;
    ondeny: (requestId: string) => void;
    onrevoke: (fingerprint: string) => void;
    ondisconnect: (fingerprint: string) => void;
  } = $props();

  let pending = $derived<EnrollmentRequestView[]>(status?.pendingRequests ?? []);
</script>

<section class="automation-panel" aria-label="MCP automation">
  <header>
    <h2>MCP automation</h2>
    {#if status?.enabled}
      <button type="button" onclick={ondisable}>Disable MCP</button>
    {:else}
      <button type="button" onclick={() => onenable("embedded")}>Enable MCP</button>
    {/if}
  </header>

  {#if status}
    <dl class="summary">
      <dt>Mode</dt>
      <dd>{status.mode}</dd>
      <dt>Host role</dt>
      <dd>{status.hostRole}</dd>
      <dt>Active grants</dt>
      <dd>{status.grants.length}</dd>
    </dl>

    {#if status.grants.length > 0}
      <h3>Grants</h3>
      <ul class="grants">
        {#each status.grants as grant (grant.grantId)}
          <li>
            <strong>{grant.grantId.slice(0, 8)}…</strong>
            <span>{grant.operations.join(", ")}</span>
            <span class="policy">disconnect: {grant.disconnectPolicy}</span>
            <ul>
              {#each grant.scopes as scope (scope.id)}
                <li>{scope.kind}: {scope.canonicalPath}</li>
              {/each}
            </ul>
          </li>
        {/each}
      </ul>
    {/if}

    {#if pending.length > 0}
      <h3>Pending requests</h3>
      {#each pending as request (request.id)}
        <PendingApprovalDialog {request} {onapprove} {ondeny} />
      {/each}
    {/if}

    <ClientList
      clients={status.clients}
      sessions={status.sessions}
      {onrevoke}
      {ondisconnect}
    />
  {/if}

  <AuditView events={audit} valid={auditValid} />
</section>
