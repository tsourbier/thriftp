# Desktop client

The desktop client is a Tauri 2 shell over `thriftp-core`, with a Svelte 5
dual-pane interface. The pinned baseline is Tauri 2.12.5, Rust 1.99.1, and
Node.js 26.13.0 LTS.

## Features

- Connection profiles for FTP, explicit FTPS, implicit FTPS, and SFTP.
- Password and SSH private-key authentication.
- Native credential storage with missing/rejected/unavailable prompts.
- Plain-FTP warning and explicit confirmation.
- Multiple simultaneous connections and reconnect.
- Dual-pane local/remote browser with sorting, filtering, hidden files, and
  virtualized listing.
- Streaming uploads/downloads with progress, speed, ETA, pause, resume, cancel,
  conflict handling, recursive directories, and symlink policy.
- Persistent queue with concurrency, bandwidth limit, priority, reorder, and
  cancel-all. Completed transfers leave the queue automatically; failed,
  cancelled, and interrupted transfers remain until dismissed.
- Tabs and directory favorites, restored on launch.
- File inspector with permissions, ownership, and supported `chmod`.
- Internal text/image preview with size limits; native open for other files.
- Startup recovery for interrupted transfers and partial files.

## IPC surface

The typed client is `apps/desktop/src/lib/ipc/client.ts`. Commands follow the
contract in `specs/001-desktop-file-transfer/contracts/desktop-ipc.md`.

Long-running operations stream events:

- `directory:batch` and `directory:error` for listings.
- `transfer:progress` and `transfer:state` for transfers.
- `queue:state` for queue changes.

The UI treats certificate, host-key, credential-store, missing-credential, and
rejected-credential errors as distinct states. Unknown and changed SSH host keys
open a verification prompt that displays the SHA-256 fingerprint and offers
Cancel or Trust and connect; changing an already trusted key requires an extra
acknowledgement.

## Keyboard and accessibility

- Arrow keys move the file selection; Enter opens; Backspace/Alt+Left goes up.
- F5 refreshes; Delete deletes; Ctrl/Cmd+H toggles hidden files.
- Modals have roles/labels, focusable controls, and a focus trap utility.
- The system/light/dark theme is applied through `data-theme` and `color-scheme`.

## Lifecycle

On normal quit the shell pauses the queue, gives active transfers a two-second
grace period to stop at a chunk boundary, then cancels anything still running.
Queue and transfer state are checkpointed through the JSON stores.

## Workspace refresh

The desktop shell is an always-visible dual-pane workspace: the local browser is
always on the left and the remote browser or disconnected connection hub is on
the right. A far-left sidebar groups saved servers, directory favorites, and
automation. The pane split is draggable and keyboard-adjustable; the split,
sidebar state, and last hub tab are persisted in `metadata.json`.

When no connection is active, the right pane shows Favorites and History tabs.
History records successful connections only, keeps one entry per saved server,
is capped at 50 entries, and is clearable without touching favorites or
credentials. Favorite server records may also store optional absolute local and
remote folders; reconnecting restores both, falling back to the nearest
accessible ancestor and then home or the server login folder.

Cmd/Ctrl+K opens the command palette for saved servers, directory favorites,
open tabs, and safe actions. Destructive and transfer-queue actions are
intentionally excluded. Both panes use breadcrumbs with an editable path mode
(Ctrl/Cmd+L), and long paths collapse middle segments into an overflow control.

SFTP key authentication discovers conventional private keys in the platform
SSH directory and exposes a key button beside the field for the native file
chooser. Discovery returns file names and paths only; key contents are never
read or displayed.

## Browser workflow polish

- File selection follows native desktop conventions: Shift selects a contiguous
  range and Cmd/Ctrl toggles individual items, including Shift/Cmd/Ctrl+Arrow
  from the keyboard. Selections survive refreshes and re-sorts while the items
  remain present.
- Name, Size, and Modified column headers sort their pane and toggle between
  ascending and descending order. Each pane keeps its own sort state, and the
  active tab restores its saved sort, hidden-file, and filter settings when
  selected again.
- The remote session line has a Link/Unlink control. When linked, every
  navigation action mirrors the same relative movement in the other pane; if the
  counterpart does not exist or cannot be read, the other pane stays unchanged.
- The queue shows per-connection completed-file counts, completed volume, and
  average speed for the current connection session. Reconnecting starts new
  statistics.
- The file inspector shows User/Group/World Read/Write/Execute permissions,
  octal and symbolic summaries, and a copy-path action. Remote permissions can
  be changed; local permissions are read-only. Directories offer a confirmed
  recursive **Apply to enclosed…** action that skips symbolic links and reports
  partial failures.
- The remote pane header owns the Disconnect action, and the pane toolbars no
  longer show back/forward buttons.
- Favorite dialogs support browsing for a local folder, keep their actions
  reachable in a scrollable body, and resolve active-tab working paths before
  saving so a remote path no longer blocks creation.
