import { cleanup, fireEvent, render, screen } from "@testing-library/svelte";
import { afterEach, describe, expect, it, vi } from "vitest";

import AutomationPanel from "./AutomationPanel.svelte";
import ClientList from "./ClientList.svelte";
import PendingApprovalDialog from "./PendingApprovalDialog.svelte";
import type {
  AutomationStatus,
  EnrollmentRequestView,
  McpClientView
} from "../../lib/automation/types";

afterEach(cleanup);

function client(id: string): McpClientView {
  return {
    id,
    fingerprint: `${id}-fingerprint-0000000000000000`,
    displayName: `Agent ${id}`,
    state: "active",
    enrolledAt: new Date().toISOString(),
    lastSeenAt: null
  };
}

function request(): EnrollmentRequestView {
  return {
    id: "request-1",
    fingerprint: "abc123",
    displayName: "New agent",
    requestedAt: new Date().toISOString(),
    expiresAt: new Date().toISOString()
  };
}

function status(): AutomationStatus {
  return {
    enabled: true,
    mode: "embedded",
    hostRole: "host",
    clients: [client("a")],
    grants: [
      {
        grantId: "grant-1",
        revision: 1,
        epoch: 0,
        expiresAt: null,
        profiles: ["profile-1"],
        operations: ["browse.list"],
        disconnectPolicy: "continue",
        scopes: [
          {
            id: "scope-1",
            kind: "local",
            profileId: null,
            canonicalPath: "/tmp",
            recursive: true
          }
        ]
      }
    ],
    scopes: [
      {
        id: "scope-1",
        grantId: "grant-1",
        kind: "local",
        profileId: null,
        canonicalPath: "/tmp",
        recursive: true
      }
    ],
    pendingRequests: [request()],
    sessions: [],
    contractVersion: "1.0.0",
    protocolVersions: ["2026-07-28", "2025-11-25"]
  };
}
describe("automation ClientList", () => {
  it("lists clients and exposes revoke and disconnect actions", async () => {
    const onrevoke = vi.fn();
    const ondisconnect = vi.fn();
    render(ClientList, {
      props: { clients: [client("a")], sessions: [], onrevoke, ondisconnect }
    });
    expect(screen.getByText("Agent a")).toBeTruthy();
    await fireEvent.click(screen.getByLabelText("Revoke Agent a"));
    expect(onrevoke).toHaveBeenCalledWith("a-fingerprint-0000000000000000");
    await fireEvent.click(screen.getByLabelText("Disconnect Agent a"));
    expect(ondisconnect).toHaveBeenCalled();
  });
});

describe("pending approval dialog", () => {
  it("defaults the disconnect policy to continue", async () => {
    const onapprove = vi.fn();
    render(PendingApprovalDialog, {
      props: { request: request(), onapprove, ondeny: vi.fn() }
    });
    const selector = screen.getByLabelText("On disconnect") as HTMLSelectElement;
    expect(selector.value).toBe("continue");
    await fireEvent.change(selector, { target: { value: "cancel" } });
    await fireEvent.click(screen.getByText("Approve"));
    expect(onapprove).toHaveBeenCalledWith(
      expect.objectContaining({ disconnectPolicy: "cancel" })
    );
  });
});

describe("automation panel", () => {
  it("shows the host role and the audit integrity state", () => {
    render(AutomationPanel, {
      props: {
        status: status(),
        audit: [],
        auditValid: true,
        onenable: vi.fn(),
        ondisable: vi.fn(),
        onapprove: vi.fn(),
        ondeny: vi.fn(),
        onrevoke: vi.fn(),
        ondisconnect: vi.fn()
      }
    });
    expect(screen.getByText("host")).toBeTruthy();
    expect(screen.getByText("Chain verified")).toBeTruthy();
  });
});
