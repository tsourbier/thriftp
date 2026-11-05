<script lang="ts">
  import { onMount } from "svelte";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import ConnectionEditor from "./components/connections/ConnectionEditor.svelte";
  import ConnectionStatus from "./components/connections/ConnectionStatus.svelte";
  import CredentialPrompt from "./components/connections/CredentialPrompt.svelte";
  import SecurityPrompt from "./components/connections/SecurityPrompt.svelte";
  import DualPane from "./components/browser/DualPane.svelte";
  import InternalPreview from "./components/previews/InternalPreview.svelte";
  import QueueManager from "./components/queue/QueueManager.svelte";
  import AutomationPanel from "./components/automation/AutomationPanel.svelte";
  import RecoveryPanel from "./components/transfers/RecoveryPanel.svelte";
  import TabBar from "./components/tabs/TabBar.svelte";
  import FavoritesSidebar from "./components/favorites/FavoritesSidebar.svelte";
  import SaveDirectoryFavoriteDialog from "./components/favorites/SaveDirectoryFavoriteDialog.svelte";
  import ServerFavoritesList from "./components/server-favorites/FavoritesList.svelte";
  import EditFavoriteDialog from "./components/server-favorites/EditFavoriteDialog.svelte";
  import SaveFavoriteDialog from "./components/server-favorites/SaveFavoriteDialog.svelte";
  import CredentialPromptDialog from "./components/server-favorites/CredentialPromptDialog.svelte";
  import CredentialUnavailableBanner from "./components/server-favorites/CredentialUnavailableBanner.svelte";
  import CredentialImpactDialog from "./components/server-favorites/CredentialImpactDialog.svelte";
  import DeleteCredentialDialog from "./components/server-favorites/DeleteCredentialDialog.svelte";
  import {
    applyActiveConnections,
    connectFavorite as connectServerFavoriteStore,
    favoriteConnectionStates,
    favoritePreferences,
    loadServerFavorites,
    removeCredential as removeServerCredential,
    reusableCredentials as loadReusableCredentials,
    serverFavorites,
    setPinned as setServerFavoritePinned,
    setPreferences as setServerFavoritePreferences
  } from "./lib/server-favorites/stores";
  import { prefillFavorite } from "./lib/server-favorites/api";
  import { setActiveConnections } from "./lib/ipc/stores";
  import Inspector from "./components/inspector/Inspector.svelte";
  import TextPrompt from "./components/common/TextPrompt.svelte";
  import { confirmAction } from "./lib/dialogs/confirm";
  import { newTab, restoreTabs } from "./lib/tabs/restore";
  import { prepareAndQuit } from "./lib/lifecycle/quit";
  import { subscribeTransferEvents } from "./lib/transfers/events";
  import {
    approveClient,
    automationStatus,
    denyClient,
    disableAutomation,
    disconnectClient,
    enableAutomation,
    loadAutomation,
    revokeClient
  } from "./lib/automation/stores";
  import { focusTrap } from "./lib/accessibility/focus";
  import { applyTheme } from "./lib/theme/apply";
  import { IpcError, ipc } from "./lib/ipc/client";
  import type {
    BrowsingSession,
    BrowserSource,
    ConnectionProfile,
    ConnectionView,
    DesktopError,
    DirectoryFavorite,
    CredentialImpact,
    CredentialPolicy,
    FavoritePreferences,
    FileEntryView,
    Preferences,
    Priority,
    QueueView,
    ReusableCredential,
    ServerFavoriteInput,
    ServerFavoriteView,
    TransferView
  } from "./lib/ipc/types";

  let profiles = $state<ConnectionProfile[]>([]);
  let active = $state<ConnectionView[]>([]);
  let editor = $state<{ profile: ConnectionProfile | null } | null>(null);
  let credentialRequest = $state<{
    profile: ConnectionProfile;
    code: string;
  } | null>(null);
  let securityProfile = $state<ConnectionProfile | null>(null);
  let notice = $state<DesktopError | null>(null);
  let transferNotice = $state<string | null>(null);
  let selectedConnectionId = $state<string | null>(null);
  let preview = $state<{
    source: BrowserSource;
    path: string;
    name: string;
  } | null>(null);
  let transfers = $state<TransferView[]>([]);
  let queue = $state<QueueView | null>(null);
  let tabs = $state<BrowsingSession[]>([]);
  let closedTabs = $state<BrowsingSession[]>([]);
  let activeTabId = $state<string | null>(null);
  let favorites = $state<DirectoryFavorite[]>([]);
  let favoritePath = $state<string | null>(null);
  let inspector = $state<{ source: BrowserSource; path: string } | null>(null);
  let quitPrompt = $state(false);
  let waitToQuit = $state(false);
  let serverFavoriteEditor = $state<ServerFavoriteView | null>(null);
  let serverFavoriteDraft = $state<{
    initial?: Partial<ServerFavoriteView>;
  } | null>(null);
  let serverFavoriteReusable = $state<ReusableCredential[]>([]);
  let serverCredentialRequest = $state<{
    favorite: ServerFavoriteView;
    code: string;
  } | null>(null);
  let serverCredentialBanner = $state<{
    code: string;
    favorite?: ServerFavoriteView;
  } | null>(null);
  let deleteCredentialRequest = $state<{
    credentialRefId: string;
    favoriteName: string;
  } | null>(null);
  let credentialImpactRequest = $state<{
    kind: "update" | "remove";
    favorite: ServerFavoriteView;
    impact: CredentialImpact;
    profile?: ServerFavoriteInput;
    policy?: CredentialPolicy;
    secret?: string;
    credentialRefId?: string;
  } | null>(null);
  let preferences = $state<Preferences | null>(null);
  let preferencesOpen = $state(false);
  let textPrompt = $state<{
    title: string;
    label: string;
    initial: string;
    submit: (value: string) => void;
  } | null>(null);

  onMount(() => {
    void refresh();
    void refreshTransfers();
    void loadServerFavorites();
    void loadAutomation();
    void loadPreferences().then((loaded) => {
      if (!loaded) {
        return;
      }
      void restoreTabs(loaded.restoreTabs).then((restored) => {
        tabs = restored;
        activeTabId = restored[0]?.id ?? null;
        const first = restored[0];
        if (first?.connectionProfileId) {
          const connection = active.find(
            (candidate) => candidate.profileId === first.connectionProfileId
          );
          selectedConnectionId = connection?.connectionId ?? null;
        }
      });
    });
    let dispose: (() => void) | undefined;
    void subscribeTransferEvents({
      onProgress: (event) => {
        transfers = transfers.map((transfer) =>
          transfer.id === event.transferId
            ? {
                ...transfer,
                transferredBytes: event.transferredBytes,
                totalBytes: event.totalBytes,
                speedBytesPerSecond: event.speedBytesPerSecond,
                estimatedRemainingSeconds: event.estimatedRemainingSeconds
              }
            : transfer
        );
      },
      onState: (event) => {
        transfers = transfers.map((transfer) =>
          transfer.id === event.transferId
            ? {
                ...transfer,
                state: event.state,
                error: event.error?.message ?? transfer.error
              }
            : transfer
        );
        if (event.state === "completed") {
          transferNotice = "Transfer completed.";
        } else if (event.state === "failed") {
          transferNotice = event.error?.message ?? "Transfer failed.";
        }
      },
      onQueue: (value) => (queue = value)
    }).then((stop) => {
      dispose = stop;
    });
    let unlistenClose: (() => void) | undefined;
    void getCurrentWindow()
      .onCloseRequested((event) => {
        const busy = transfers.some((transfer) =>
          ["pending", "active", "paused"].includes(transfer.state)
        );
        if (busy) {
          event.preventDefault();
          quitPrompt = true;
        }
      })
      .then((unlisten) => {
        unlistenClose = unlisten;
      })
      .catch(() => {
        // The window API is unavailable outside a Tauri runtime.
      });
    return () => {
      dispose?.();
      unlistenClose?.();
    };
  });

  $effect(() => {
    if (!waitToQuit) {
      return;
    }
    const busy = transfers.some((transfer) =>
      ["pending", "active", "paused"].includes(transfer.state)
    );
    if (!busy) {
      waitToQuit = false;
      void prepareAndQuit();
    }
  });

  async function quitAfterCancelling() {
    quitPrompt = false;
    try {
      await ipc.queueCancelAll(true);
    } finally {
      await prepareAndQuit();
    }
  }

  function waitForTransfers() {
    quitPrompt = false;
    waitToQuit = true;
  }

  const selectedConnection = $derived(
    active.find((connection) => connection.connectionId === selectedConnectionId) ?? null
  );

  $effect(() => {
    setActiveConnections(active);
    applyActiveConnections($serverFavorites, active);
  });

  const activeServerFavoriteId = $derived(
    $serverFavorites.find((favorite) =>
      active.some(
        (connection) =>
          connection.profileId === favorite.connectionProfileId &&
          connection.state !== "error"
      )
    )?.id ?? null
  );

  const activeTab = $derived(
    tabs.find((tab) => tab.id === activeTabId) ?? tabs[0] ?? null
  );

  $effect(() => {
    const profileId = selectedConnection?.profileId;
    if (!profileId) {
      favorites = [];
      return;
    }
    void loadFavorites(profileId);
  });

  async function refresh() {
    try {
      profiles = await ipc.listProfiles();
      notice = null;
    } catch (error) {
      notice = detailsOf(error);
    }
  }

  async function loadPreferences(): Promise<Preferences | null> {
    try {
      preferences = await ipc.preferencesGet();
      applyTheme(preferences.theme);
      return preferences;
    } catch (error) {
      notice = detailsOf(error);
      return null;
    }
  }

  async function savePreferences(next: Preferences) {
    try {
      preferences = await ipc.preferencesSet(next);
      applyTheme(preferences.theme);
      preferencesOpen = false;
    } catch (error) {
      notice = detailsOf(error);
    }
  }

  async function saveProfile(profile: ConnectionProfile, secret?: string) {
    try {
      const exists = profiles.some((candidate) => candidate.id === profile.id);
      if (exists) {
        await ipc.updateProfile(profile, secret);
      } else {
        await ipc.createProfile(profile, secret);
      }
      editor = null;
      notice = null;
      await Promise.all([refresh(), loadServerFavorites()]);
    } catch (error) {
      notice = detailsOf(error);
    }
  }

  async function requestConnect(profile: ConnectionProfile) {
    if (profile.protocol === "ftp") {
      securityProfile = profile;
      return;
    }
    await connectNow(profile);
  }

  async function connectNow(profile: ConnectionProfile, secret?: string) {
    try {
      const connection = await ipc.connect(profile.id, secret);
      active = [
        ...active.filter(
          (candidate) => candidate.connectionId !== connection.connectionId
        ),
        connection
      ];
      selectedConnectionId = connection.connectionId;
      ensureTab(connection);
      notice = null;
    } catch (error) {
      handleConnectionError(error, profile);
    }
  }

  function handleConnectionError(error: unknown, profile: ConnectionProfile) {
    const details = detailsOf(error);
    if (
      details.code === "credential_missing" ||
      details.code === "credential_store_unavailable" ||
      details.code === "credential_rejected"
    ) {
      credentialRequest = { profile, code: details.code };
      return;
    }
    notice = details;
  }

  async function confirmPlainFtp() {
    const profile = securityProfile;
    securityProfile = null;
    if (profile) {
      await connectNow(profile);
    }
  }

  async function submitCredential(secret: string) {
    const request = credentialRequest;
    credentialRequest = null;
    if (request) {
      await connectNow(request.profile, secret);
    }
  }

  async function reconnect(connection: ConnectionView) {
    const profile = profiles.find((candidate) => candidate.id === connection.profileId);
    if (!profile) {
      notice = {
        code: "not_found",
        message: "The connection profile for this session no longer exists."
      };
      return;
    }
    try {
      const replacement = await ipc.reconnect(connection.connectionId);
      active = active.map((candidate) =>
        candidate.connectionId === connection.connectionId ? replacement : candidate
      );
      selectedConnectionId = replacement.connectionId;
      ensureTab(replacement);
      notice = null;
    } catch (error) {
      handleConnectionError(error, profile);
    }
  }

  async function disconnect(connection: ConnectionView) {
    try {
      await ipc.disconnect(connection.connectionId);
      active = active.filter(
        (candidate) => candidate.connectionId !== connection.connectionId
      );
      if (selectedConnectionId === connection.connectionId) {
        selectedConnectionId = active[0]?.connectionId ?? null;
      }
    } catch (error) {
      notice = detailsOf(error);
    }
  }

  async function duplicateProfile(profile: ConnectionProfile) {
    const now = new Date().toISOString();
    const copy: ConnectionProfile = {
      ...profile,
      id: crypto.randomUUID(),
      displayName: `${profile.displayName} (copy)`,
      createdAt: now,
      updatedAt: now,
      lastUsedAt: null
    };
    try {
      await ipc.createProfile(copy);
      await Promise.all([refresh(), loadServerFavorites()]);
    } catch (error) {
      notice = detailsOf(error);
    }
  }

  async function deleteProfile(profile: ConnectionProfile) {
    if (
      !(await confirmAction(
        `Delete the saved connection "${profile.displayName}"?`,
        "Delete connection"
      ))
    ) {
      return;
    }
    try {
      const favorite = $serverFavorites.find(
        (candidate) => candidate.connectionProfileId === profile.id
      );
      if (favorite) {
        await deleteServerFavorite(favorite);
        return;
      }
      await ipc.deleteProfile(profile.id);
      await refresh();
      await loadServerFavorites();
    } catch (error) {
      notice = detailsOf(error);
    }
  }

  function detailsOf(error: unknown): DesktopError {
    if (error instanceof IpcError) {
      return error.details;
    }
    return {
      code: "internal",
      message: error instanceof Error ? error.message : String(error)
    };
  }

  function credentialMessage() {
    return credentialRequest?.code === "credential_rejected"
      ? "The server rejected that credential. Try again."
      : undefined;
  }

  function openPreview(entry: FileEntryView, source: BrowserSource) {
    preview = { source, path: entry.path, name: entry.name };
  }

  async function refreshTransfers() {
    try {
      transfers = await ipc.transfersList();
      queue = await ipc.queueGet();
    } catch (error) {
      notice = detailsOf(error);
    }
  }

  async function pauseTransfer(transfer: TransferView) {
    try {
      const updated = await ipc.transfersPause(transfer.id);
      replaceTransfer(updated);
    } catch (error) {
      notice = detailsOf(error);
    }
  }

  async function resumeTransfer(transfer: TransferView) {
    try {
      const updated = await ipc.transfersResume(transfer.id);
      replaceTransfer(updated);
    } catch (error) {
      notice = detailsOf(error);
    }
  }

  async function restartTransfer(transfer: TransferView) {
    try {
      replaceTransfer(await ipc.transfersRestart(transfer.id));
    } catch (error) {
      notice = detailsOf(error);
    }
  }

  async function discardPartial(transfer: TransferView) {
    try {
      replaceTransfer(await ipc.transfersDiscardPartial(transfer.id));
    } catch (error) {
      notice = detailsOf(error);
    }
  }

  async function cancelTransfer(transfer: TransferView) {
    try {
      const updated = await ipc.transfersCancel(transfer.id);
      replaceTransfer(updated);
    } catch (error) {
      notice = detailsOf(error);
    }
  }

  function replaceTransfer(updated: TransferView) {
    transfers = transfers.map((transfer) =>
      transfer.id === updated.id ? updated : transfer
    );
  }

  async function pauseQueue() {
    queue = await ipc.queuePause();
  }

  async function resumeQueue() {
    queue = await ipc.queueResume();
  }

  async function setConcurrency(limit: number) {
    queue = await ipc.queueSetConcurrency(limit);
  }

  async function setBandwidth(bytesPerSecond: number | null) {
    queue = await ipc.queueSetBandwidth(bytesPerSecond);
  }

  async function reorderTransfers(ids: string[]) {
    queue = await ipc.queueReorder(ids);
  }

  async function setPriority(transferId: string, priority: Priority) {
    queue = await ipc.queueSetPriority(transferId, priority);
  }

  async function cancelAll(confirmed: boolean) {
    if (!confirmed) {
      return;
    }
    queue = await ipc.queueCancelAll(true);
    await refreshTransfers();
  }

  async function clearHistory() {
    try {
      await ipc.transfersClearHistory();
      await refreshTransfers();
    } catch (error) {
      notice = detailsOf(error);
    }
  }

  async function dismissTransfer(transfer: TransferView) {
    try {
      queue = await ipc.transfersDismiss(transfer.id);
      transfers = transfers.filter((candidate) => candidate.id !== transfer.id);
    } catch (error) {
      notice = detailsOf(error);
    }
  }

  async function loadFavorites(profileId: string) {
    try {
      favorites = await ipc.favoritesList(profileId);
    } catch (error) {
      notice = detailsOf(error);
    }
  }

  async function saveFavorite(label: string, path: string) {
    const profileId = selectedConnection?.profileId;
    if (!profileId) {
      return;
    }
    try {
      await ipc.favoritesCreate(profileId, path, label);
      favoritePath = null;
      await loadFavorites(profileId);
    } catch (error) {
      notice = detailsOf(error);
    }
  }

  async function deleteFavorite(favorite: DirectoryFavorite) {
    try {
      await ipc.favoritesDelete(favorite.id);
      await loadFavorites(favorite.connectionProfileId);
    } catch (error) {
      notice = detailsOf(error);
    }
  }

  async function moveFavorite(favorite: DirectoryFavorite, delta: number) {
    const order = favorites
      .filter(
        (candidate) => candidate.connectionProfileId === favorite.connectionProfileId
      )
      .map((candidate) => candidate.id);
    const index = order.indexOf(favorite.id);
    const target = index + delta;
    if (index < 0 || target < 0 || target >= order.length) {
      return;
    }
    [order[index], order[target]] = [order[target], order[index]];
    try {
      await ipc.favoritesReorder(favorite.connectionProfileId, order);
      await loadFavorites(favorite.connectionProfileId);
    } catch (error) {
      notice = detailsOf(error);
    }
  }

  function editFavorite(favorite: DirectoryFavorite) {
    textPrompt = {
      title: "Favorite label",
      label: "Label",
      initial: favorite.label,
      submit: (label) => {
        if (label === favorite.label) {
          return;
        }
        void (async () => {
          try {
            await ipc.favoritesUpdate(favorite.id, label);
            await loadFavorites(favorite.connectionProfileId);
          } catch (error) {
            notice = detailsOf(error);
          }
        })();
      }
    };
  }

  function openFavorite(favorite: DirectoryFavorite) {
    const connection = active.find(
      (candidate) => candidate.profileId === favorite.connectionProfileId
    );
    if (connection) {
      selectedConnectionId = connection.connectionId;
    }
    updateActiveTabPath(favorite.path);
  }

  async function saveTabs() {
    try {
      await ipc.tabsSave(tabs);
    } catch {
      // Tab persistence is best-effort; browsing must never be blocked by it.
    }
  }

  function ensureTab(connection: ConnectionView) {
    const existing = tabs.find((tab) => tab.connectionProfileId === connection.profileId);
    if (existing) {
      activeTabId = existing.id;
      return;
    }
    const tab = newTab(connection.profileId, ".");
    tabs = [...tabs, tab];
    activeTabId = tab.id;
    void saveTabs();
  }

  function selectTab(tab: BrowsingSession) {
    activeTabId = tab.id;
    const connection = active.find(
      (candidate) => candidate.profileId === tab.connectionProfileId
    );
    selectedConnectionId = connection?.connectionId ?? null;
  }

  function closeTab(tab: BrowsingSession) {
    closedTabs = [...closedTabs, tab];
    tabs = tabs.filter((candidate) => candidate.id !== tab.id);
    if (activeTabId === tab.id) {
      activeTabId = tabs[0]?.id ?? null;
    }
    void saveTabs();
  }

  function reopenTab() {
    const tab = closedTabs[closedTabs.length - 1];
    if (!tab) {
      return;
    }
    closedTabs = closedTabs.slice(0, -1);
    tabs = [...tabs, tab];
    activeTabId = tab.id;
    void saveTabs();
  }

  async function retryTab(tab: BrowsingSession) {
    if (!tab.connectionProfileId) {
      return;
    }
    const profile = profiles.find(
      (candidate) => candidate.id === tab.connectionProfileId
    );
    if (profile) {
      await connectNow(profile);
    }
  }

  function addTab() {
    const tab = newTab(selectedConnection?.profileId ?? null, ".");
    tabs = [...tabs, tab];
    activeTabId = tab.id;
    void saveTabs();
  }

  function updateActiveTabPath(path: string) {
    if (!activeTab) {
      return;
    }
    const history = activeTab.history.slice(0, activeTab.historyIndex + 1);
    if (history[history.length - 1] !== path) {
      history.push(path);
    }
    tabs = tabs.map((tab) =>
      tab.id === activeTab.id
        ? {
            ...tab,
            currentPath: path,
            history,
            historyIndex: history.length - 1,
            restored: false
          }
        : tab
    );
    void saveTabs();
  }

  function updateActiveTabSelection(source: BrowserSource, paths: string[]) {
    if (!activeTab) {
      return;
    }
    tabs = tabs.map((tab) =>
      tab.id === activeTab.id
        ? source === "local"
          ? { ...tab, localSelectedItems: paths }
          : { ...tab, selectedItems: paths }
        : tab
    );
    void saveTabs();
  }

  function updateActiveTabLocalPath(path: string) {
    if (!activeTab) {
      return;
    }
    tabs = tabs.map((tab) =>
      tab.id === activeTab.id ? { ...tab, localPath: path } : tab
    );
    void saveTabs();
  }

  function updateActiveTabView(settings: {
    sortKey: "name" | "size" | "date" | "type";
    showHidden: boolean;
    filter: string;
  }) {
    if (!activeTab) {
      return;
    }
    tabs = tabs.map((tab) =>
      tab.id === activeTab.id
        ? {
            ...tab,
            viewSettings: {
              sortKey: settings.sortKey,
              showHidden: settings.showHidden,
              filter: settings.filter
            }
          }
        : tab
    );
    void saveTabs();
  }

  function tabLabel(tab: BrowsingSession) {
    const profile = profiles.find(
      (candidate) => candidate.id === tab.connectionProfileId
    );
    return profile?.displayName ?? tab.currentPath;
  }

  function selectedConnectionLabel() {
    const profile = profiles.find(
      (candidate) => candidate.id === selectedConnection?.profileId
    );
    return profile?.displayName ?? "";
  }

  function serverFavoriteInput(favorite: ServerFavoriteView): ServerFavoriteInput {
    return {
      displayName: favorite.displayName,
      groupLabel: favorite.groupLabel,
      protocol: favorite.protocol,
      host: favorite.host,
      port: favorite.port,
      username: favorite.username,
      authMethod: favorite.authMethod,
      timeoutSeconds: favorite.timeoutSeconds,
      sshKeyRef: favorite.sshKeyRef,
      pinned: favorite.pinned
    };
  }

  function favoriteSecretKind(favorite: ServerFavoriteView): "password" | "passphrase" {
    return favorite.authMethod === "sshPrivateKey" ? "passphrase" : "password";
  }

  function handleCredentialWarning(view: ServerFavoriteView) {
    if (view.credentialWarning) {
      serverCredentialBanner = {
        code: view.credentialWarning.code,
        favorite: view
      };
    } else {
      serverCredentialBanner = null;
    }
  }

  async function loadReusableForDialog() {
    try {
      serverFavoriteReusable = await loadReusableCredentials();
    } catch {
      serverFavoriteReusable = [];
    }
  }

  async function openServerFavoriteDraft(initial?: Partial<ServerFavoriteView>) {
    await loadReusableForDialog();
    serverFavoriteDraft = { initial };
  }

  async function openServerFavoriteEditor(favorite: ServerFavoriteView) {
    await loadReusableForDialog();
    serverFavoriteEditor = favorite;
  }

  async function saveActiveAsFavorite() {
    if (!selectedConnection) {
      return;
    }
    try {
      const prefilled = await prefillFavorite(selectedConnection.connectionId);
      if (prefilled.favoriteId) {
        const existing = $serverFavorites.find(
          (favorite) => favorite.id === prefilled.favoriteId
        );
        if (existing) {
          await openServerFavoriteEditor(existing);
          return;
        }
      }
      await openServerFavoriteDraft(prefilled.initial);
    } catch (error) {
      notice = detailsOf(error);
    }
  }

  function credentialError(error: unknown): boolean {
    const code = detailsOf(error).code;
    return [
      "credential_missing",
      "credential_rejected",
      "credential_store_unavailable"
    ].includes(code);
  }

  async function createServerFavorite(
    profile: ServerFavoriteInput,
    policy: CredentialPolicy,
    secret?: string,
    credentialRefId?: string
  ) {
    try {
      const view = await ipc.serverFavoritesCreate(
        profile,
        policy,
        credentialRefId,
        secret
      );
      serverFavoriteDraft = null;
      handleCredentialWarning(view);
      await Promise.all([loadServerFavorites(), refresh()]);
    } catch (error) {
      notice = detailsOf(error);
    }
  }

  async function applyServerFavoriteUpdate(
    favorite: ServerFavoriteView,
    profile: ServerFavoriteInput,
    policy: CredentialPolicy,
    secret?: string,
    credentialRefId?: string
  ) {
    const view = await ipc.serverFavoritesUpdate(
      favorite.id,
      profile,
      policy,
      credentialRefId,
      secret
    );
    serverFavoriteEditor = null;
    handleCredentialWarning(view);
    await Promise.all([loadServerFavorites(), refresh()]);
  }

  async function saveServerFavoriteEdit(
    profile: ServerFavoriteInput,
    policy: CredentialPolicy,
    secret?: string,
    credentialRefId?: string
  ) {
    if (!serverFavoriteEditor) return;
    const favorite = serverFavoriteEditor;
    try {
      if (favorite.hasSavedCredential) {
        const impact = await ipc.credentialsImpact(favorite.id).catch(() => null);
        if (impact?.isShared) {
          credentialImpactRequest = {
            kind: "update",
            favorite,
            impact,
            profile,
            policy,
            secret,
            credentialRefId
          };
          return;
        }
      }
      await applyServerFavoriteUpdate(favorite, profile, policy, secret, credentialRefId);
    } catch (error) {
      notice = detailsOf(error);
    }
  }

  async function requestRemoveFavoriteCredential(favorite: ServerFavoriteView) {
    try {
      const impact = await ipc.credentialsImpact(favorite.id);
      credentialImpactRequest = { kind: "remove", favorite, impact };
    } catch (error) {
      notice = detailsOf(error);
    }
  }

  async function confirmCredentialImpact() {
    const request = credentialImpactRequest;
    credentialImpactRequest = null;
    if (!request) {
      return;
    }
    try {
      if (request.kind === "update" && request.profile && request.policy) {
        await applyServerFavoriteUpdate(
          request.favorite,
          request.profile,
          request.policy,
          request.secret,
          request.credentialRefId
        );
      } else {
        await removeServerCredential(request.favorite.id, true);
        serverFavoriteEditor = null;
      }
    } catch (error) {
      notice = detailsOf(error);
    }
  }

  async function connectServerFavorite(favorite: ServerFavoriteView) {
    try {
      const connection = await connectServerFavoriteStore(
        favorite.id,
        undefined,
        favoriteSecretKind(favorite)
      );
      active = [
        ...active.filter(
          (candidate) => candidate.connectionId !== connection.connectionId
        ),
        connection
      ];
      selectedConnectionId = connection.connectionId;
      ensureTab(connection);
      await loadServerFavorites();
    } catch (error) {
      const details = detailsOf(error);
      if (credentialError(error)) {
        if (details.code === "credential_store_unavailable") {
          serverCredentialBanner = { code: details.code, favorite };
        }
        serverCredentialRequest = { favorite, code: details.code };
      } else {
        notice = details;
      }
    }
  }

  async function submitServerCredential(secret: string, save: boolean) {
    const request = serverCredentialRequest;
    serverCredentialRequest = null;
    if (!request) {
      return;
    }
    try {
      if (save) {
        const view = await ipc.serverFavoritesUpdate(
          request.favorite.id,
          serverFavoriteInput(request.favorite),
          "save",
          undefined,
          secret
        );
        handleCredentialWarning(view);
        await loadServerFavorites();
      }
      const connection = await connectServerFavoriteStore(
        request.favorite.id,
        secret,
        favoriteSecretKind(request.favorite)
      );
      active = [
        ...active.filter(
          (candidate) => candidate.connectionId !== connection.connectionId
        ),
        connection
      ];
      selectedConnectionId = connection.connectionId;
      ensureTab(connection);
      await loadServerFavorites();
    } catch (error) {
      notice = detailsOf(error);
    }
  }

  async function pinServerFavorite(favorite: ServerFavoriteView, pinned: boolean) {
    try {
      await setServerFavoritePinned(favorite.id, pinned);
    } catch (error) {
      notice = detailsOf(error);
    }
  }

  async function deleteServerFavorite(favorite: ServerFavoriteView) {
    try {
      const result = await ipc.serverFavoritesDelete(favorite.id);
      await Promise.all([loadServerFavorites(), refresh()]);
      if (result.orphanedCredentialRefId) {
        deleteCredentialRequest = {
          credentialRefId: result.orphanedCredentialRefId,
          favoriteName: favorite.displayName
        };
      }
    } catch (error) {
      notice = detailsOf(error);
    }
  }

  async function resolveOrphanedCredential(deleteIt: boolean) {
    const request = deleteCredentialRequest;
    deleteCredentialRequest = null;
    if (!request) {
      return;
    }
    try {
      if (deleteIt) {
        await ipc.credentialsDelete(request.credentialRefId, true);
      }
      await loadServerFavorites();
    } catch (error) {
      notice = detailsOf(error);
    }
  }

  async function sortServerFavorites(sortKey: FavoritePreferences["sortKey"]) {
    try {
      await setServerFavoritePreferences({ ...$favoritePreferences, sortKey });
    } catch (error) {
      notice = detailsOf(error);
    }
  }

  async function groupServerFavorite(favorite: ServerFavoriteView, label: string) {
    try {
      await ipc.serverFavoritesSetGroup(favorite.id, label);
      await loadServerFavorites();
    } catch (error) {
      notice = detailsOf(error);
    }
  }

  async function ungroupServerFavorite(favorite: ServerFavoriteView) {
    try {
      await ipc.serverFavoritesSetGroup(favorite.id, null);
      await loadServerFavorites();
    } catch (error) {
      notice = detailsOf(error);
    }
  }

  async function renameServerFavoriteGroup(oldLabel: string, newLabel: string) {
    try {
      await ipc.serverFavoritesRenameGroup(oldLabel, newLabel);
      await loadServerFavorites();
    } catch (error) {
      notice = detailsOf(error);
    }
  }

  async function testServerFavorite(
    profile: Parameters<typeof ipc.serverFavoritesTestConnection>[0],
    secret?: string
  ) {
    try {
      return await ipc.serverFavoritesTestConnection(profile, secret);
    } catch (error) {
      return { ok: false, error: detailsOf(error) };
    }
  }
</script>

<main class="shell">
  <header class="titlebar">
    <h1>Thriftp</h1>
    <div class="title-actions">
      <button type="button" onclick={() => (preferencesOpen = true)}>Preferences</button>
      <button type="button" onclick={() => void openServerFavoriteDraft()}>
        New favorite
      </button>
      <button onclick={() => (editor = { profile: null })}>New connection</button>
    </div>
  </header>

  {#if notice}
    <div class="error" role="alert">
      <span>{notice.message}</span>
      <button type="button" onclick={() => (notice = null)}>Dismiss</button>
    </div>
  {/if}
  {#if transferNotice}
    <div class="transfer-notice" role="status">
      <span>{transferNotice}</span>
      <button type="button" onclick={() => (transferNotice = null)}>Dismiss</button>
    </div>
  {/if}

  <div class="workspace">
    <ConnectionStatus
      {profiles}
      {active}
      onconnect={requestConnect}
      ondisconnect={disconnect}
      onreconnect={reconnect}
      onedit={(profile) => (editor = { profile })}
      onduplicate={duplicateProfile}
      ondelete={deleteProfile}
    />

    <section class="server-favorites-panel" aria-label="Saved servers">
      {#if serverCredentialBanner}
        <CredentialUnavailableBanner
          code={serverCredentialBanner.code}
          onretry={() => void loadServerFavorites()}
          onrepair={() => {
            if (serverCredentialBanner?.favorite) {
              void openServerFavoriteEditor(serverCredentialBanner.favorite);
            }
            serverCredentialBanner = null;
          }}
        />
      {/if}
      <ServerFavoritesList
        favorites={$serverFavorites}
        preferences={$favoritePreferences}
        activeFavoriteId={activeServerFavoriteId}
        connectionStates={$favoriteConnectionStates}
        onconnect={connectServerFavorite}
        onedit={(favorite) => void openServerFavoriteEditor(favorite)}
        ondelete={deleteServerFavorite}
        onpin={pinServerFavorite}
        onsort={sortServerFavorites}
        ongroup={groupServerFavorite}
        onrenamegroup={renameServerFavoriteGroup}
        onungroup={ungroupServerFavorite}
        oncreate={() => void openServerFavoriteDraft()}
      />
    </section>

    <div class="browser-area">
      <TabBar
        {tabs}
        activeId={activeTab?.id ?? null}
        labelFor={tabLabel}
        onselect={selectTab}
        onclose={closeTab}
        onadd={addTab}
        connectedProfileIds={active.map((connection) => connection.profileId)}
        onretry={retryTab}
        onreopen={reopenTab}
        canReopen={closedTabs.length > 0}
      />
      <RecoveryPanel
        {transfers}
        onresume={resumeTransfer}
        onrestart={restartTransfer}
        oncancel={cancelTransfer}
        ondiscard={discardPartial}
      />
      {#if active.length > 0}
        <div class="connection-picker">
          <label>
            Remote session
            <select bind:value={selectedConnectionId} aria-label="Active connection">
              {#each active as connection (connection.connectionId)}
                {@const profile = profiles.find(
                  (candidate) => candidate.id === connection.profileId
                )}
                <option value={connection.connectionId}>
                  {profile?.displayName ?? connection.profileId} · {connection.state}
                </option>
              {/each}
            </select>
          </label>
          <button type="button" onclick={() => void saveActiveAsFavorite()}>
            Save as favorite
          </button>
        </div>
        {#if selectedConnection?.transportWarning}
          <p class="transport-warning" role="status">
            {selectedConnection.transportWarning.message}
          </p>
        {/if}
      {/if}
      <div class="browser-body">
        {#if selectedConnection}
          <FavoritesSidebar
            {favorites}
            canSave={Boolean(activeTab)}
            onopen={openFavorite}
            onsave={() => {
              if (activeTab) {
                favoritePath = activeTab.currentPath;
              }
            }}
            ondelete={deleteFavorite}
            onedit={editFavorite}
            onmove={moveFavorite}
          />
        {/if}
        <DualPane
          connectionId={selectedConnection?.connectionId ?? null}
          connectionLabel={selectedConnectionLabel()}
          profileId={selectedConnection?.profileId ?? null}
          remotePath={activeTab?.currentPath ?? null}
          localPath={activeTab?.localPath ?? "."}
          localSelectedItems={activeTab?.localSelectedItems ?? []}
          remoteSelectedItems={activeTab?.selectedItems ?? []}
          onlocalpathchange={updateActiveTabLocalPath}
          onselectionchange={updateActiveTabSelection}
          defaultShowHidden={activeTab?.viewSettings.showHidden ??
            preferences?.showHiddenFiles ??
            false}
          defaultSortKey={activeTab?.viewSettings.sortKey ??
            preferences?.sortKey ??
            "name"}
          onviewchange={updateActiveTabView}
          onpreview={openPreview}
          oninspect={(entry, source) => (inspector = { source, path: entry.path })}
          ontransferstarted={refreshTransfers}
          onremotepathchange={updateActiveTabPath}
        />
      </div>
    </div>
    <QueueManager
      {transfers}
      {queue}
      onpausequeue={pauseQueue}
      onresumequeue={resumeQueue}
      onsetconcurrency={setConcurrency}
      onsetbandwidth={setBandwidth}
      onreorder={reorderTransfers}
      onsetpriority={setPriority}
      onpausetransfer={pauseTransfer}
      onresumetransfer={resumeTransfer}
      oncanceltransfer={cancelTransfer}
      oncancelall={() => cancelAll(true)}
      onclearhistory={clearHistory}
      ondismiss={dismissTransfer}
    />
  </div>
</main>

<section class="automation-surface" aria-label="MCP automation controls">
  <AutomationPanel
    status={$automationStatus}
    onenable={(mode) => void enableAutomation(mode)}
    ondisable={() => void disableAutomation()}
    onapprove={(input) => void approveClient(input)}
    ondeny={(requestId) => void denyClient(requestId)}
    onrevoke={(fingerprint) => void revokeClient(fingerprint)}
    ondisconnect={(fingerprint) => void disconnectClient(fingerprint)}
  />
</section>

{#if editor}
  <div
    class="modal"
    role="dialog"
    aria-modal="true"
    use:focusTrap
    aria-label="Connection editor"
  >
    <ConnectionEditor
      profile={editor.profile}
      onsave={saveProfile}
      oncancel={() => (editor = null)}
    />
  </div>
{/if}

{#if credentialRequest}
  <div
    class="modal"
    role="dialog"
    aria-modal="true"
    use:focusTrap
    aria-label="Credentials required"
  >
    <CredentialPrompt
      profileName={credentialRequest.profile.displayName}
      message={credentialMessage()}
      onsubmit={submitCredential}
      oncancel={() => (credentialRequest = null)}
    />
  </div>
{/if}

{#if inspector}
  <div
    class="modal"
    role="dialog"
    aria-modal="true"
    use:focusTrap
    aria-label="File information"
  >
    <Inspector
      source={inspector.source}
      connectionId={selectedConnection?.connectionId ?? null}
      path={inspector.path}
      onclose={() => (inspector = null)}
    />
  </div>
{/if}

{#if serverFavoriteEditor}
  <div
    class="modal"
    role="dialog"
    aria-modal="true"
    use:focusTrap
    aria-label="Edit server favorite"
  >
    <EditFavoriteDialog
      favorite={serverFavoriteEditor}
      reusable={serverFavoriteReusable}
      onsave={saveServerFavoriteEdit}
      ontest={testServerFavorite}
      onremovecredential={() => {
        if (serverFavoriteEditor) {
          void requestRemoveFavoriteCredential(serverFavoriteEditor);
        }
      }}
      oncancel={() => (serverFavoriteEditor = null)}
    />
  </div>
{/if}

{#if serverFavoriteDraft}
  <div
    class="modal"
    role="dialog"
    aria-modal="true"
    use:focusTrap
    aria-label="Save server favorite"
  >
    <SaveFavoriteDialog
      initial={serverFavoriteDraft.initial}
      reusable={serverFavoriteReusable}
      onsave={createServerFavorite}
      oncancel={() => (serverFavoriteDraft = null)}
    />
  </div>
{/if}

{#if credentialImpactRequest}
  <div
    class="modal"
    role="dialog"
    aria-modal="true"
    use:focusTrap
    aria-label="Credential impact"
  >
    <CredentialImpactDialog
      impact={credentialImpactRequest.impact}
      action={credentialImpactRequest.kind === "remove" ? "remove" : "update"}
      onconfirm={() => void confirmCredentialImpact()}
      oncancel={() => (credentialImpactRequest = null)}
    />
  </div>
{/if}

{#if deleteCredentialRequest}
  <div
    class="modal"
    role="dialog"
    aria-modal="true"
    use:focusTrap
    aria-label="Credential deletion"
  >
    <DeleteCredentialDialog
      onkeep={() => void resolveOrphanedCredential(false)}
      ondelete={() => void resolveOrphanedCredential(true)}
      oncancel={() => (deleteCredentialRequest = null)}
    />
  </div>
{/if}

{#if serverCredentialRequest}
  <div
    class="modal"
    role="dialog"
    aria-modal="true"
    use:focusTrap
    aria-label="Server favorite credential required"
  >
    <CredentialPromptDialog
      code={serverCredentialRequest.code}
      favoriteName={serverCredentialRequest.favorite.displayName}
      secretKind={favoriteSecretKind(serverCredentialRequest.favorite)}
      onsubmit={submitServerCredential}
      oncancel={() => (serverCredentialRequest = null)}
    />
  </div>
{/if}

{#if favoritePath}
  <div
    class="modal"
    role="dialog"
    aria-modal="true"
    use:focusTrap
    aria-label="Save directory favorite"
  >
    <SaveDirectoryFavoriteDialog
      path={favoritePath}
      onsave={saveFavorite}
      oncancel={() => (favoritePath = null)}
    />
  </div>
{/if}

{#if preview}
  <div
    class="modal"
    role="dialog"
    aria-modal="true"
    use:focusTrap
    aria-label="File preview"
  >
    <InternalPreview
      source={preview.source}
      connectionId={selectedConnection?.connectionId ?? null}
      path={preview.path}
      name={preview.name}
      onclose={() => (preview = null)}
    />
  </div>
{/if}

{#if quitPrompt}
  <div
    class="modal"
    role="dialog"
    aria-modal="true"
    use:focusTrap
    aria-label="Transfers in progress"
  >
    <div class="quit-dialog">
      <h2>Transfers are still running</h2>
      <p>
        Wait for active transfers to finish, or cancel them before quitting. Thriftp does
        not quit silently while transfers remain.
      </p>
      <div class="actions">
        <button type="button" onclick={waitForTransfers}>Wait for completion</button>
        <button type="button" class="danger" onclick={quitAfterCancelling}>
          Cancel transfers and quit
        </button>
        <button type="button" onclick={() => (quitPrompt = false)}>Stay</button>
      </div>
    </div>
  </div>
{/if}

{#if textPrompt}
  <div
    class="modal"
    role="dialog"
    aria-modal="true"
    use:focusTrap
    aria-label={textPrompt.title}
  >
    <TextPrompt
      title={textPrompt.title}
      label={textPrompt.label}
      initial={textPrompt.initial}
      onsubmit={(value) => {
        const submit = textPrompt?.submit;
        textPrompt = null;
        submit?.(value);
      }}
      oncancel={() => (textPrompt = null)}
    />
  </div>
{/if}

{#if preferencesOpen && preferences}
  <div
    class="modal"
    role="dialog"
    aria-modal="true"
    use:focusTrap
    aria-label="Preferences"
  >
    <form
      class="preferences"
      onsubmit={(event) => {
        event.preventDefault();
        void savePreferences(preferences);
      }}
    >
      <h2>Preferences</h2>
      <label>
        Theme
        <select bind:value={preferences.theme}>
          <option value="system">System</option>
          <option value="light">Light</option>
          <option value="dark">Dark</option>
        </select>
      </label>
      <label class="inline">
        <input type="checkbox" bind:checked={preferences.restoreTabs} />
        Restore open tabs on launch
      </label>
      <label class="inline">
        <input type="checkbox" bind:checked={preferences.showHiddenFiles} />
        Show hidden files by default
      </label>
      <label>
        Default concurrency
        <input
          type="number"
          min="1"
          max="16"
          bind:value={preferences.defaultConcurrency}
        />
      </label>
      <label>
        Preview mode
        <select bind:value={preferences.previewMode}>
          <option value="auto">Automatic</option>
          <option value="native">Native when available</option>
          <option value="internal">Internal</option>
        </select>
      </label>
      <div class="actions">
        <button type="button" onclick={() => (preferencesOpen = false)}>Cancel</button>
        <button type="submit">Save</button>
      </div>
    </form>
  </div>
{/if}

{#if securityProfile}
  <div
    class="modal"
    role="dialog"
    aria-modal="true"
    use:focusTrap
    aria-label="Unencrypted FTP warning"
  >
    <SecurityPrompt
      title="Unencrypted FTP connection"
      message="Plain FTP does not encrypt your credentials or file data. Continue only if you trust this network and server."
      confirmLabel="Connect anyway"
      onconfirm={confirmPlainFtp}
      oncancel={() => (securityProfile = null)}
    />
  </div>
{/if}

<style>
  .shell {
    display: flex;
    flex-direction: column;
    min-height: 100vh;
    font-family: system-ui, sans-serif;
  }
  .titlebar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0.75rem 1rem;
    border-bottom: 1px solid #d0d0d0;
  }
  .titlebar h1 {
    font-size: 1rem;
    margin: 0;
  }
  .title-actions {
    display: flex;
    gap: 0.5rem;
  }
  .preferences {
    display: grid;
    gap: 0.65rem;
    min-width: 22rem;
  }
  .preferences h2 {
    margin: 0;
    font-size: 1rem;
  }
  .preferences label {
    display: grid;
    gap: 0.25rem;
    font-size: 0.85rem;
  }
  .preferences label.inline {
    display: flex;
    align-items: center;
    gap: 0.4rem;
  }
  .preferences .actions {
    display: flex;
    justify-content: flex-end;
    gap: 0.5rem;
  }
  .workspace {
    display: flex;
    flex: 1;
    min-height: 0;
  }
  .browser-area {
    display: flex;
    flex-direction: column;
    flex: 1;
    min-width: 0;
    min-height: 0;
  }
  .browser-body {
    display: flex;
    flex: 1;
    min-height: 0;
    min-width: 0;
  }
  .connection-picker {
    padding: 0.5rem 0.75rem;
    border-bottom: 1px solid #d0d0d0;
    font-size: 0.8rem;
  }
  .connection-picker label {
    display: inline-flex;
    align-items: center;
    gap: 0.4rem;
  }
  .transport-warning {
    margin: 0;
    padding: 0.4rem 0.75rem;
    background: #fff3cd;
    color: #6b4b00;
    font-size: 0.8rem;
    border-bottom: 1px solid #e0c26a;
  }
  .transfer-notice {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 1rem;
    padding: 0.45rem 1rem;
    background: #e7f5e7;
    color: #1a5c1a;
    font-size: 0.85rem;
  }
  .error {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 1rem;
    margin: 0;
    padding: 0.5rem 1rem;
    background: #ffe5e5;
    color: #8a0000;
  }
  .modal {
    position: fixed;
    inset: 0;
    display: grid;
    place-items: center;
    padding: 1rem;
    background: rgb(0 0 0 / 0.25);
  }
  .modal > :global(*) {
    background: Canvas;
    border-radius: 0.5rem;
    box-shadow: 0 10px 30px rgb(0 0 0 / 0.2);
    padding: 1rem;
  }
  .quit-dialog {
    display: grid;
    gap: 0.75rem;
    max-width: 34rem;
  }
  .quit-dialog h2 {
    margin: 0;
    font-size: 1rem;
  }
  .quit-dialog p {
    margin: 0;
    font-size: 0.85rem;
  }
  .quit-dialog .actions {
    display: flex;
    flex-wrap: wrap;
    justify-content: flex-end;
    gap: 0.5rem;
  }
  .quit-dialog .danger {
    color: #8a0000;
  }
</style>
