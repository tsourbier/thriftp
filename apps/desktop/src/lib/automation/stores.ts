import { writable } from "svelte/store";
import { automationApi } from "./api";
import type {
  ApprovalInput,
  AuditEventView,
  AutomationStatus,
  RetentionReport
} from "./types";

export const automationStatus = writable<AutomationStatus | null>(null);
export const automationAudit = writable<AuditEventView[]>([]);
export const automationAuditValid = writable<boolean | null>(null);
export const automationRetention = writable<RetentionReport | null>(null);
export const automationError = writable<string | null>(null);

export async function loadAutomation(): Promise<void> {
  try {
    automationStatus.set(await automationApi.status());
    automationError.set(null);
  } catch (error) {
    automationError.set(
      error instanceof Error ? error.message : "Could not load automation state."
    );
  }
}

export async function enableAutomation(mode: string): Promise<void> {
  automationStatus.set(await automationApi.enable(mode));
}

export async function disableAutomation(): Promise<void> {
  automationStatus.set(await automationApi.disable());
}

export async function approveClient(input: ApprovalInput): Promise<void> {
  await automationApi.approve(input);
  await loadAutomation();
}

export async function denyClient(requestId: string): Promise<void> {
  await automationApi.deny(requestId);
  await loadAutomation();
}

export async function revokeClient(fingerprint: string): Promise<void> {
  await automationApi.revoke(fingerprint);
  await loadAutomation();
}

export async function disconnectClient(fingerprint: string): Promise<void> {
  await automationApi.disconnect(fingerprint);
  await loadAutomation();
}

export async function loadAudit(): Promise<void> {
  automationAudit.set(await automationApi.audit());
  automationAuditValid.set((await automationApi.verifyAudit()).valid);
}
