# Desktop client

The desktop client is a Tauri 2 shell over `thriftp-core`, with a Svelte 5
dual-pane interface.

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
  cancel-all.
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
rejected-credential errors as distinct states.

## Keyboard and accessibility

- Arrow keys move the file selection; Enter opens; Backspace/Alt+Left goes up.
- F5 refreshes; Delete deletes; Ctrl/Cmd+H toggles hidden files.
- Modals have roles/labels, focusable controls, and a focus trap utility.
- The system/light/dark theme is applied through `data-theme` and `color-scheme`.

## Lifecycle

On normal quit the shell pauses the queue, gives active transfers a two-second
grace period to stop at a chunk boundary, then cancels anything still running.
Queue and transfer state are checkpointed through the JSON stores.
