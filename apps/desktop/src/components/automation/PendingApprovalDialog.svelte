<script lang="ts">
  import type {
    ApprovalInput,
    DisconnectPolicy,
    EnrollmentRequestView
  } from "../../lib/automation/types";

  let {
    request,
    onapprove,
    ondeny
  }: {
    request: EnrollmentRequestView;
    onapprove: (input: ApprovalInput) => void;
    ondeny: (requestId: string) => void;
  } = $props();

  let disconnectPolicy = $state<DisconnectPolicy>("continue");
  let directories = $state("");
  let localDirectories = $state("");
  let operations = $state("browse.list,transfers.list");

  function submit() {
    onapprove({
      requestId: request.id,
      profileIds: [],
      directories: directories
        .split(",")
        .map((value) => value.trim())
        .filter(Boolean),
      localDirectories: localDirectories
        .split(",")
        .map((value) => value.trim())
        .filter(Boolean),
      operations: operations
        .split(",")
        .map((value) => value.trim())
        .filter(Boolean),
      disconnectPolicy,
      preApprove: []
    });
  }
</script>

<form
  class="approval-dialog"
  onsubmit={(event) => {
    event.preventDefault();
    submit();
  }}
>
  <h3>Approve {request.displayName}</h3>
  <p class="fingerprint">{request.fingerprint}</p>

  <label>
    Remote directories (profile:path, comma separated)
    <input bind:value={directories} />
  </label>
  <label>
    Local directories (comma separated)
    <input bind:value={localDirectories} />
  </label>
  <label>
    Operations
    <input bind:value={operations} />
  </label>
  <label>
    On disconnect
    <select bind:value={disconnectPolicy}>
      <option value="continue">Continue transfers</option>
      <option value="cancel">Cancel transfers</option>
    </select>
  </label>

  <div class="actions">
    <button type="submit">Approve</button>
    <button type="button" onclick={() => ondeny(request.id)}>Deny</button>
  </div>
</form>

<style>
  .approval-dialog {
    display: grid;
    gap: 0.75rem;
    padding: 1rem;
    border: 1px solid var(--border, #ddd);
    border-radius: 0.5rem;
  }

  label {
    display: grid;
    gap: 0.25rem;
  }

  .fingerprint {
    font-family: monospace;
    color: var(--muted, #666);
    overflow-wrap: anywhere;
  }

  .actions {
    display: flex;
    gap: 0.5rem;
  }
</style>
