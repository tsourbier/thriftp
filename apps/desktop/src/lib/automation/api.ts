import { invoke } from "@tauri-apps/api/core";
import type {
  ApprovalInput,
  AuditEventView,
  AutomationStatus,
  RetentionReport
} from "./types";

export const automationApi = {
  status: () => invoke<AutomationStatus>("automation_status"),
  enable: (mode: string) => invoke<AutomationStatus>("automation_enable", { mode }),
  disable: () => invoke<AutomationStatus>("automation_disable"),
  pendingRequests: () =>
    invoke<AutomationStatus["pendingRequests"]>("automation_pending_requests"),
  approve: (input: ApprovalInput) =>
    invoke<unknown>("automation_approve", {
      requestId: input.requestId,
      profileIds: input.profileIds,
      directories: input.directories,
      localDirectories: input.localDirectories,
      operations: input.operations,
      expiresAt: input.expiresAt ?? null,
      disconnectPolicy: input.disconnectPolicy,
      preApprove: input.preApprove
    }),
  deny: (requestId: string) => invoke<unknown>("automation_deny", { requestId }),
  revoke: (fingerprint: string) => invoke<unknown>("automation_revoke", { fingerprint }),
  disconnect: (fingerprint: string) =>
    invoke<unknown>("automation_disconnect", { fingerprint }),
  issueConfirmation: (
    fingerprint: string,
    operation: string,
    target?: string | null,
    ttl = 120
  ) =>
    invoke<{ confirmationId: string; expiresAt: string }>(
      "automation_issue_confirmation",
      { fingerprint, operation, target: target ?? null, ttl }
    ),
  audit: (limit = 200) => invoke<AuditEventView[]>("automation_audit_list", { limit }),
  verifyAudit: () => invoke<{ valid: boolean }>("automation_audit_verify"),
  retention: (days: number) => invoke<unknown>("automation_retention", { days }),
  limit: (bytes: number | null) => invoke<unknown>("automation_limit", { bytes }),
  applyRetention: () => invoke<RetentionReport>("automation_apply_retention"),
  hostStatus: () => invoke<unknown>("automation_host_status")
};
