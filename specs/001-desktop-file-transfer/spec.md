# Feature Specification: Thriftp Desktop File Transfer Client

**Feature Branch**: `001-desktop-file-transfer`

**Created**: 2026-11-05

**Status**: Draft

**Input**: User description: "Thriftp product specification for the initial release: a free, open-source, cross-platform desktop file transfer client with FTP, FTPS, and SFTP support, dual-pane local/remote browsing, a transfer queue, secure credential storage, and native desktop conventions."

## Clarifications

### Session 2026-11-05

- Q: When a transfer would overwrite an existing destination file, what should happen by default? → A: Ask per conflict, offering overwrite, skip, rename, and apply-to-all remaining.
- Q: During a recursive directory transfer, how should symbolic links be handled? → A: Preserve links when supported, otherwise skip and report; never follow them.
- Q: What should happen when the user quits while transfers are active? → A: Warn and require the user to wait for completion or cancel the transfers before quitting.
- Q: How should destructive delete operations behave? → A: Confirm deletes; use the operating system trash locally when available, and warn that remote deletes are permanent.
- Q: Should open tabs and their current locations be restored on restart? → A: Restore them by default, with a preference to start with a clean workspace.

### Refinement Decisions (2026-11-05)

- Q: Is pausing the entire transfer queue required in addition to pausing individual transfers? → A: Yes; users can pause and resume individual transfers and the entire queue.
- Q: Is the bandwidth limit global or per transfer? → A: One configurable global limit with an unlimited option; per-transfer limits are out of scope for the initial release.
- Q: How can users prioritize queued transfers? → A: Reorder queued transfers and assign priority levels that determine start order.
- Q: What does "compare directory contents" mean? → A: Viewing two directories side by side; no automated diff or comparison function in the initial release.
- Q: What happens to partially transferred files? → A: Local partial files are retained in an identifiable incomplete state for possible resume and can be discarded by the user; remote partial files are cleaned up when the protocol supports it, otherwise reported.
- Q: Are the transfer queue and transfer history persisted? → A: Yes; both persist across restarts, and interrupted transfers are restored as interrupted.
- Q: What does "organization details" on a connection profile mean? → A: An optional user-defined group label used to organize saved connections.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Connect securely to a remote server (Priority: P1)

A developer, system administrator, or webmaster creates a connection profile for an FTP, FTPS, or SFTP server, authenticates with a password or an SSH private key, and opens a session. The profile and its non-secret settings persist across application restarts, while the secret is kept in the operating system's credential store.

**Why this priority**: No file transfer can happen without a connection; secure, reliable connection management is the foundation of every other capability.

**Independent Test**: Create one profile per supported protocol, connect to a test server with each authentication method, restart the application, and confirm the profile remains usable without re-entering the secret.

**Acceptance Scenarios**:

1. **Given** no saved connections, **when** the user creates an FTP, FTPS, or SFTP profile with host, port, username, and password, **then** the application connects and shows the remote home location.
2. **Given** an SFTP profile, **when** the user selects private-key authentication and provides a valid key, **then** the connection succeeds without requiring a password.
3. **Given** a saved profile, **when** the application restarts, **then** the profile remains listed and connects using the credential retrieved from the operating system credential store.
4. **Given** invalid credentials or an unreachable host, **when** the user attempts to connect, **then** the application shows an actionable error and remains usable.
5. **Given** an FTPS server that supports explicit or implicit TLS, **when** the user selects the corresponding mode, **then** the connection succeeds.
6. **Given** several saved profiles, **when** the user assigns an optional group label, **then** the profiles can be organized and found under that label.

---

### User Story 2 - Browse local and remote files in a dual-pane workspace (Priority: P1)

The user opens a local filesystem panel and a remote filesystem panel side by side, navigates each independently, inspects file metadata and previews, and performs common file and directory operations without leaving the application.

**Why this priority**: Browsing is the primary interaction model; users must be able to find and manage files before transferring them.

**Independent Test**: Open both panels, navigate to nested directories, sort and refresh listings, toggle hidden files, preview a file, and create, rename, and delete an item on each side.

**Acceptance Scenarios**:

1. **Given** a connected session, **when** the user opens the dual-pane workspace, **then** a local panel and a remote panel are shown and can be navigated independently.
2. **Given** a directory listing, **when** the user sorts by name, size, date, or type, **then** the listing reorders accordingly.
3. **Given** a remote directory, **when** the user creates, renames, or deletes a file or directory, **then** the change is reflected after the operation completes.
4. **Given** hidden files exist, **when** the user enables the hidden-files view, **then** those entries become visible and can be hidden again.
5. **Given** the application window is resized, **when** it reaches a smaller size, **then** both panels remain usable and core actions remain reachable.
6. **Given** a file or directory, **when** the user requests deletion, **then** the application asks for confirmation; local items are moved to the operating system trash when available, and remote deletes warn that they are permanent.
7. **Given** a selected file, **when** the user requests a preview, **then** the native operating system preview opens where available, otherwise an internal preview fallback is shown without leaving the application.
8. **Given** a directory with many entries, **when** the listing is opened, **then** the first entries appear before the complete listing finishes and the panel remains responsive while enumeration continues.

---

### User Story 3 - Transfer files and directories with clear progress and control (Priority: P1)

The user uploads and downloads individual files, multiple selected files, and entire directories, using buttons or drag and drop, while monitoring progress, speed, and remaining time, pausing or cancelling work, and receiving a clear result when each transfer finishes or fails.

**Why this priority**: Reliable transfer is the product's core value proposition.

**Independent Test**: Upload and download a single file, a multi-file selection, and a nested directory; verify progress, pause, resume, cancellation, completion notification, and the final state on both sides.

**Acceptance Scenarios**:

1. **Given** files selected in one panel, **when** the user starts a transfer to the other panel, **then** the files are transferred and per-transfer progress is shown.
2. **Given** a directory selected for transfer, **when** the transfer starts, **then** its contents are transferred recursively while preserving the directory structure.
3. **Given** an active transfer, **when** the user cancels it, **then** the transfer stops promptly and is marked cancelled.
4. **Given** a completed or failed transfer, **when** it reaches its final state, **then** the user receives a clear notification and the state is distinguishable as successful, failed, cancelled, paused, or interrupted.
5. **Given** a file larger than available memory, **when** it is transferred, **then** the application remains responsive and completes the transfer without exhausting memory.
6. **Given** a network interruption during a transfer that the server can resume, **when** connectivity returns, **then** the transfer resumes from the last confirmed position rather than restarting from the beginning.
7. **Given** a destination file already exists, **when** the user starts a transfer that would overwrite it, **then** the application asks how to resolve the conflict, offers overwrite, skip, rename, and apply-to-all remaining, and does not block unrelated browsing or other transfers.
8. **Given** a directory containing symbolic links, **when** it is transferred recursively, **then** links are recreated where the protocol and destination support it, skipped with a visible report otherwise, and never followed.
9. **Given** an active transfer, **when** the user pauses it, **then** it stops consuming transfer bandwidth, its progress is preserved, and its state becomes paused.
10. **Given** a paused transfer, **when** the user resumes it, **then** it continues from its last confirmed position rather than restarting.

---

### User Story 4 - Manage and control all transfers from a dedicated queue (Priority: P2)

The user opens a transfer manager that lists pending, active, paused, completed, failed, cancelled, and interrupted transfers, shows individual and global progress, allows cancelling individual transfers or all pending transfers, lets the user pause or resume the entire queue, applies a global bandwidth limit, and lets the user reorder or prioritize queued work while continuing to browse.

**Why this priority**: Professional users routinely run many transfers and need visibility and control without losing their browsing context.

**Independent Test**: Queue several transfers, change the concurrency limit, pause and resume the queue, apply a bandwidth limit, reorder queued transfers, cancel one transfer and then all pending ones, and verify each status and progress display while navigating other folders.

**Acceptance Scenarios**:

1. **Given** multiple queued transfers, **when** the user opens the transfer manager, **then** each transfer appears with its status and progress.
2. **Given** an active queue, **when** the user changes the concurrency limit, **then** the number of simultaneously active transfers adjusts.
3. **Given** pending transfers, **when** the user cancels all pending transfers, **then** they stop and are marked cancelled.
4. **Given** an active queue, **when** the user navigates other directories or switches tabs, **then** the transfer manager remains accessible and transfers continue.
5. **Given** active transfers, **when** the user quits the application, **then** the application warns and requires the user to wait for completion or cancel the transfers before quitting.
6. **Given** an active queue, **when** the user pauses the queue, **then** active transfers pause and no pending transfer starts until the queue is resumed.
7. **Given** a global bandwidth limit, **when** transfers are active, **then** aggregate transfer throughput remains at or below the configured limit.
8. **Given** queued transfers, **when** the user reorders them or assigns a higher priority, **then** the start order reflects the new order or priority without interrupting active transfers.

---

### User Story 5 - Organize work with tabs, favorites, and navigation history (Priority: P2)

The user opens multiple remote locations in tabs, switches between connections while preserving each tab's navigation state, saves frequently used remote directories as favorites associated with a profile, and moves back and forward through visited locations.

**Why this priority**: Repeated professional workflows depend on fast context switching and quick access to familiar locations.

**Independent Test**: Open several tabs, navigate each to a different directory, save favorites, restart the application, and confirm tabs and favorites restore as expected.

**Acceptance Scenarios**:

1. **Given** a connected session, **when** the user opens a new tab, **then** the new tab has an independent navigation state.
2. **Given** two tabs on different connections, **when** the user switches between them, **then** each retains its own path and view settings.
3. **Given** a remote directory, **when** the user saves it as a favorite, **then** it appears under the associated connection profile and persists across restarts.
4. **Given** a navigation history, **when** the user moves back and then forward, **then** the previously visited locations are restored in order.
5. **Given** active transfers, **when** the user switches tabs or closes a browsing tab, **then** the transfers continue.
6. **Given** one or more open tabs, **when** the application restarts, **then** the tabs and their locations are restored unless the user has chosen to start with a clean workspace.

---

### User Story 6 - Inspect and manage file metadata and permissions (Priority: P3)

The user opens an optional inspector for a selected file or directory to review its type, size, dates, permissions, ownership, and remote path, and modifies Unix permissions where the protocol and server support it.

**Why this priority**: Metadata and permission management are important for technical users but are not required for the basic transfer loop.

**Independent Test**: Select files on local and remote panels, open the inspector, modify permissions on a supporting server, and verify that unsupported metadata is presented gracefully.

**Acceptance Scenarios**:

1. **Given** a selected file or directory, **when** the user opens the inspector, **then** available metadata is displayed and unavailable fields are omitted or marked unavailable.
2. **Given** a remote filesystem that supports Unix permissions, **when** the user changes permissions, **then** the new permissions are applied and reflected in the listing.
3. **Given** a server that does not support permission or ownership changes, **when** the user attempts the operation, **then** the application explains the limitation without failing or blocking navigation.

---

### User Story 7 - Recover interrupted transfers after restart (Priority: P2)

The user restarts the application after an unexpected exit or a network interruption and finds the transfer queue and history intact, with interrupted transfers clearly identified and resumable without silently trusting stale partial data.

**Why this priority**: Resumability is only trustworthy when users can see what happened and when the application verifies that resuming is safe.

**Independent Test**: Interrupt a transfer, restart the application, confirm the queue and history are restored, resume the transfer, and verify that a changed source file is detected and not resumed silently.

**Acceptance Scenarios**:

1. **Given** an active transfer when the application terminates unexpectedly, **when** the application starts again, **then** the transfer appears as interrupted with its last confirmed progress and a resume action.
2. **Given** pending queued transfers at exit, **when** the application starts again, **then** the queue and history are restored and the pending transfers remain pending rather than starting automatically.
3. **Given** an interrupted transfer, **when** the user resumes it, **then** the application verifies that the source and destination have not changed since the recorded position before continuing.
4. **Given** an interrupted transfer whose source size or modification date changed, **when** the user attempts to resume, **then** the application refuses to resume silently and offers to restart or cancel the transfer.
5. **Given** a cancelled or failed transfer, **when** it stops, **then** its last confirmed progress is preserved and its state is reported accurately.
6. **Given** a partial local file from an interrupted or failed transfer, **when** the user views it, **then** it is clearly identified as incomplete and can be discarded or retained for a later resume.

---

### Edge Cases

- A transfer is interrupted by network loss, sleep, or application termination; the application must distinguish interrupted from failed and resume only when the protocol and server support it.
- A resume is attempted after the source file changed size or modification date; the application must refuse a silent resume and offer restart or cancel.
- A partial local file exists from a prior transfer; it must be identifiable as incomplete and must never be presented as a completed file.
- A remote partial file cannot be cleaned up because the protocol does not support deletion; the application must report the leftover file.
- An SSH host key changes unexpectedly; the user must be warned and must explicitly decide whether to continue.
- A TLS certificate is invalid, expired, or self-signed; verification must fail visibly and must not be bypassed silently.
- A local directory contains 10,000 entries; first entries must appear promptly and the interface must remain responsive while the complete listing loads.
- A remote directory contains 10,000 entries over a slow link; first entries must appear as soon as the server returns them and the interface must remain responsive while the listing completes.
- A file name contains characters that are invalid or differently encoded on the local filesystem; the user must receive a clear error or a safe rename option.
- A recursive transfer encounters a name collision; the user must be able to resolve it with overwrite, skip, rename, or apply-to-all remaining without an intrusive blocking loop.
- A directory contains symbolic links that point outside the selected tree or form a loop; the application must never follow them and must report any skipped links.
- The local disk is full or a local permission is denied mid-transfer; the failure must be reported and partial output must be identifiable.
- A remote item is renamed or deleted by another user during browsing or transfer; the application must report the conflict and remain stable.
- The user quits with active transfers; the application must warn and require the user to wait for completion or cancel the transfers first.
- A server reports capabilities that differ from its protocol defaults; the interface must degrade gracefully rather than assume unsupported behavior.
- Multiple tabs connect to the same server with different credentials; sessions must remain independent.
- Clock or timezone differences make remote modification dates ambiguous; dates must be displayed consistently and without implying false precision.
- The user deletes a non-empty directory or permanently deletes an item; confirmation must state the scope and irreversibility before the action proceeds.
- A restored tab cannot reconnect on launch; it must show a clear disconnected state with a retry action and must not block other tabs.
- The platform has no native file preview capability; the application must use its internal preview fallback.
- A remote file selected for preview is too large to download within available memory; the application must not download it without explicit user action and must offer metadata or a bounded preview.
- The user sets a bandwidth limit below the active transfers' current aggregate throughput; the application must reduce throughput to the limit without corrupting transfers.
- The user pauses the queue while a file is being written; the transfer must stop after the current block completes and before the next block is written, preserving its progress.

## Requirements *(mandatory)*

### Functional Requirements

**Connection Management**

- **FR-001**: Users MUST be able to create, edit, duplicate, and delete connection profiles.
- **FR-002**: Users MUST be able to configure protocol, host, port, username, authentication method, and connection timeout for each profile.
- **FR-003**: Users MUST be able to authenticate with a password.
- **FR-004**: Users MUST be able to authenticate to SFTP with an SSH private key.
- **FR-005**: Users MUST be able to save and organize frequently used connections, including assigning an optional group label.
- **FR-006**: Connection profiles MUST persist between application sessions.
- **FR-007**: Users MUST be able to open multiple simultaneous connections.
- **FR-008**: Users MUST be able to reconnect to previously used servers.
- **FR-009**: The application MUST support protocol-specific connection parameters for FTP, FTPS, and SFTP.
- **FR-010**: The application MUST present actionable errors when a connection fails and MUST remain usable.
- **FR-011**: FTPS connections MUST support explicit and implicit TLS modes where the server supports them.

**Local and Remote Browsing**

- **FR-012**: Users MUST be able to navigate remote directory structures, including moving to a parent directory.
- **FR-013**: Users MUST be able to navigate to a remote location using a path or location bar.
- **FR-014**: Remote listings MUST display file name, size, modification date, and permissions where the protocol and server provide them.
- **FR-015**: Users MUST be able to sort directory contents by name, size, date, and type.
- **FR-016**: Users MUST be able to create, rename, and delete remote files and directories; deletes MUST require confirmation and MUST warn that remote deletion is permanent.
- **FR-017**: Users MUST be able to show and hide hidden files.
- **FR-018**: Users MUST be able to refresh remote directory contents.
- **FR-019**: The application MUST handle differences in capabilities between supported protocols and remote filesystems without assuming unavailable behavior.
- **FR-020**: The application MUST provide an integrated local filesystem browser with directory navigation, metadata display, sorting, and filtering.
- **FR-021**: Users MUST be able to create, rename, and delete local files and directories from within the application; deletes MUST require confirmation and MUST use the operating system trash when available.
- **FR-022**: Local filesystem access MUST respect operating system permissions and security restrictions.
- **FR-023**: Directory listings MUST display the first available entries before the complete listing finishes and MUST keep the interface responsive while enumeration continues.
- **FR-024**: Local and remote directory performance MUST be specified and evaluated separately, accounting for server and network behavior on remote listings.

**Dual-Pane Workspace**

- **FR-025**: The main interface MUST provide two independent file browsing panels.
- **FR-026**: The default layout MUST present a local filesystem panel and a remote filesystem panel.
- **FR-027**: Users MUST be able to switch the active panel, navigate each panel independently, and open different locations in each panel.
- **FR-028**: Users MUST be able to transfer files between panels and view both directories side by side; automated directory comparison or diffing is not part of the initial release.
- **FR-029**: Users MUST be able to drag and drop files between panels.
- **FR-030**: Users MUST be able to perform common navigation and file operations with keyboard shortcuts.
- **FR-031**: The dual-pane interface MUST remain usable across supported window sizes.

**File Transfers**

- **FR-032**: Users MUST be able to upload and download individual files.
- **FR-033**: Users MUST be able to upload and download directories recursively; symbolic links MUST be recreated where the protocol and destination support it, MUST be skipped with a visible report otherwise, and MUST NOT be followed.
- **FR-034**: Users MUST be able to transfer multiple selected files in one operation.
- **FR-035**: Users MUST be able to initiate transfers by drag and drop.
- **FR-036**: All transfer operations MUST run asynchronously and MUST NOT block the user interface.
- **FR-037**: Users MUST be able to monitor per-transfer progress.
- **FR-038**: Users MUST be able to cancel ongoing transfers.
- **FR-039**: The application MUST display transfer speed and, when the total size is known, an estimated remaining time.
- **FR-040**: The application MUST notify users when transfers complete or fail.
- **FR-041**: The application MUST clearly distinguish pending, active, paused, successful, failed, cancelled, and interrupted transfers.
- **FR-042**: Large files MUST be transferred without loading the entire file into memory.
- **FR-043**: The application MUST handle network interruptions gracefully and report the resulting transfer state.
- **FR-044**: When a transfer would overwrite an existing destination file, the application MUST ask the user how to resolve the conflict and MUST offer overwrite, skip, rename, and apply-to-all remaining as choices; the prompt MUST NOT block unrelated browsing or transfer queue activity.
- **FR-045**: Transfer errors MUST be explicit, actionable, and attributable to the affected file or directory.

**Transfer Controls**

- **FR-046**: Users MUST be able to pause an individual active transfer; a paused transfer MUST stop consuming transfer bandwidth and preserve its progress.
- **FR-047**: Users MUST be able to resume an individual paused or interrupted transfer from its last confirmed position.
- **FR-048**: Users MUST be able to pause and resume the entire transfer queue; pausing the queue MUST pause active transfers and prevent pending transfers from starting.
- **FR-049**: Users MUST be able to set a global transfer bandwidth limit, including an unlimited option; the aggregate throughput of active transfers MUST NOT exceed the configured limit.
- **FR-050**: Users MUST be able to reorder queued transfers and assign priority levels that determine start order without interrupting active transfers.
- **FR-051**: Users MUST be able to configure the transfer concurrency limit.

**Transfer Lifecycle and Recovery**

- **FR-052**: The transfer queue and transfer history MUST persist across application restarts.
- **FR-053**: On startup, transfers that were active when the application terminated unexpectedly MUST be restored as interrupted with their last confirmed progress and a resume action.
- **FR-054**: Queued transfers that were pending at exit MUST be restored as pending and MUST NOT start automatically without user action.
- **FR-055**: On normal quit, the application MUST require the user to wait for completion or cancel the transfers; cancelled transfers MUST be reported as cancelled.
- **FR-056**: Transfers active during an unexpected termination MUST become interrupted rather than failed or completed.
- **FR-057**: Partial local files MUST be retained in an identifiable incomplete state for possible resume and MUST NOT be presented as complete; users MUST be able to discard them.
- **FR-058**: Partial remote files MUST be cleaned up when the protocol and server support it; otherwise the leftover remote file MUST be reported and retained for possible resume.
- **FR-059**: Before resuming, the application MUST verify that the source has not changed since the recorded position, using size and modification time where available, and that the destination partial data matches the recorded offset; if verification fails, the application MUST NOT resume silently and MUST offer restart or cancel.
- **FR-060**: Pausing MUST preserve progress and resuming MUST continue from the last confirmed position rather than restarting.
- **FR-061**: Cancellation and failure MUST preserve the last confirmed progress and MUST report the correct final state.
- **FR-062**: Completed transfers MUST report complete progress; interrupted and failed transfers MUST report their last confirmed progress and MUST NOT be shown as complete.

**Transfer Queue**

- **FR-063**: The application MUST provide a dedicated transfer manager listing pending, active, paused, completed, failed, cancelled, and interrupted transfers.
- **FR-064**: The transfer manager MUST show individual and global progress.
- **FR-065**: The transfer manager MUST show transfer status and error information.
- **FR-066**: Users MUST be able to cancel individual transfers and all pending transfers.
- **FR-067**: The transfer manager MUST remain accessible while users browse files or manage connections.
- **FR-068**: Completed and failed operations MUST remain clearly visible until the user dismisses them.

**Permissions and File Information**

- **FR-069**: Users MUST be able to view Unix file permissions for local and remote entries where available.
- **FR-070**: Users MUST be able to modify file and directory permissions where the protocol and server support it.
- **FR-071**: Users MUST be able to view ownership information where available.
- **FR-072**: Unsupported permission or ownership operations MUST be handled gracefully with a clear explanation.
- **FR-073**: The application MUST provide an optional file information inspector for selected files and directories.
- **FR-074**: The inspector MUST display name, type, size, modification date, permissions, ownership where available, remote path, and relevant protocol-specific metadata.
- **FR-075**: The inspector MUST adapt to the capabilities of the local or remote filesystem.

**File Previews**

- **FR-076**: Users MUST be able to preview a selected local or remote file using the native operating system preview capability where available.
- **FR-077**: When the native preview capability is unavailable on a supported platform, the application MUST provide an internal preview fallback without leaving the application.
- **FR-078**: Opening or closing a preview MUST NOT block browsing, transfers, or other file operations; previewing a remote file MUST NOT require downloading the entire file without explicit user action.

**Favorites, Tabs, and Navigation**

- **FR-079**: Users MUST be able to save frequently accessed remote directories as favorites.
- **FR-080**: Users MUST be able to organize favorite locations.
- **FR-081**: Users MUST be able to quickly access saved connections.
- **FR-082**: Users MUST be able to navigate backward and forward through previously visited locations.
- **FR-083**: Favorites MUST persist across sessions and be associated with their relevant connection profiles.
- **FR-084**: Users MUST be able to open multiple browsing tabs.
- **FR-085**: Users MUST be able to maintain independent navigation state in each tab.
- **FR-086**: Users MUST be able to close and reopen tabs.
- **FR-087**: Open tabs and their locations MUST be restored on launch by default, and a preference MUST allow starting with a clean workspace.
- **FR-088**: Transfer operations MUST continue while the user navigates between tabs.

**User Experience**

- **FR-089**: The application MUST follow each supported platform's interaction conventions for window controls, menus, keyboard shortcuts, file dialogs, and standard desktop gestures.
- **FR-090**: The interface MUST use consistent terminology and layout across views and MUST provide immediate visible feedback for user actions.
- **FR-091**: The application MUST make functionality discoverable and support efficient keyboard navigation.
- **FR-092**: The interface MUST clearly distinguish local resources from remote resources.
- **FR-093**: Transfer states MUST be represented consistently across browsing views, notifications, and the transfer manager.
- **FR-094**: The interface MUST provide accessible typography, contrast, and interaction elements.
- **FR-095**: The application MUST support light and dark appearance where the platform provides it.
- **FR-096**: Common file operations MUST NOT require users to understand protocol-specific technical details.
- **FR-097**: The application MUST provide a consistent experience across platforms while respecting each operating system's visual and interaction language.

**Security and Privacy**

- **FR-098**: The application MUST NOT store credentials in plaintext.
- **FR-099**: Persistent credentials MUST be stored using the native credential management facilities of the operating system.
- **FR-100**: The application MUST NOT expose passwords, private keys, or other secrets in logs, telemetry, crash reports, error messages, or source control.
- **FR-101**: TLS certificates MUST be validated by default.
- **FR-102**: SSH server host keys MUST be verified.
- **FR-103**: The application MUST detect unexpected SSH host key changes and warn the user.
- **FR-104**: Certificate and host key verification MUST NOT be bypassed silently; any override MUST require explicit user action and remain visible for the session.
- **FR-105**: The application MUST clearly indicate when an unencrypted FTP connection is in use, before and during the connection.
- **FR-106**: The application MUST respect operating system filesystem permissions.
- **FR-107**: The application MUST avoid unnecessary collection or transmission of user information.
- **FR-108**: The application MUST NOT require an account, subscription, cloud service, or remote authentication service to function.
- **FR-109**: Telemetry MUST NOT be mandatory.

**Reliability and Performance**

- **FR-110**: The application MUST support asynchronous network operations.
- **FR-111**: The application MUST provide clear, actionable error messages that identify the failed operation and its target.
- **FR-112**: The application MUST handle network interruptions without crashing or losing browsing state.
- **FR-113**: The application MUST support multiple concurrent transfers within configured limits.
- **FR-114**: The application MUST avoid unnecessary resource consumption during browsing and transfers.
- **FR-115**: The application MUST remain responsive during intensive transfer operations.
- **FR-116**: The application MUST account for differences in server capabilities and network behavior.
- **FR-117**: When the user quits with pending, active, or paused transfers, the application MUST warn and require an explicit choice to wait for completion or cancel the transfers before quitting; it MUST NOT quit silently while transfers remain.
- **FR-118**: Directory enumeration MUST NOT block user interaction; scrolling, selection, and navigation MUST remain available while a listing loads.
- **FR-119**: Local and remote directory listings MUST be measured against separate performance targets that account for round-trip latency, server throughput, and network variability.
- **FR-120**: While a directory listing is incomplete, the interface MUST indicate that more entries are still loading.

**Open Source and Distribution**

- **FR-121**: The project MUST be hosted in a public source repository.
- **FR-122**: The project MUST use a permissive open-source license, initially MIT.
- **FR-123**: The project MUST provide installation and build instructions.
- **FR-124**: The project MUST provide contribution guidelines.
- **FR-125**: The project MUST document architectural decisions.
- **FR-126**: The project MUST include a security reporting policy.
- **FR-127**: Third-party dependencies and their licenses MUST remain transparent.
- **FR-128**: The project MUST support reproducible development and build workflows.
- **FR-129**: The project MUST NOT depend on proprietary remote services for core functionality.
- **FR-130**: The application MUST build and run on macOS, Linux, and Windows.
- **FR-131**: The initial release MUST include automated tests, cross-platform build validation, and user documentation.

### Key Entities *(include if feature involves data)*

- **Connection Profile**: A saved configuration for reaching a server. Holds a display name, optional group label, protocol, host, port, username, authentication method, timeout, and references to stored credentials. A profile can own favorites and be referenced by tabs.
- **Credential**: A secret used to authenticate. Referenced by a connection profile and held in the operating system credential store; never stored in plaintext with the profile.
- **Browsing Session / Tab**: An active connection context with its own current location, navigation history, view settings, and selected items. Multiple sessions may exist simultaneously, including multiple sessions to the same server.
- **File Entry**: A file or directory shown in a panel. Includes name, type, size, modification date, permissions, ownership where available, hidden status, path, and protocol-specific metadata.
- **Transfer**: A single upload or download operation. Includes direction, source, destination, total size, transferred amount, state (pending, active, paused, completed, failed, cancelled, interrupted), speed, estimated remaining time, resumability, resume verification data, partial-file location, and error details.
- **Transfer Queue**: The ordered collection of transfers, their priority levels, the configured concurrency limit, the global bandwidth limit, and aggregated progress. The queue persists across restarts.
- **Transfer History**: The persisted record of completed, failed, cancelled, and interrupted transfers, including their final states and error details.
- **Partial Transfer Data**: Local or remote data retained from an incomplete transfer, identified as incomplete and associated with the transfer needed to resume or discard it.
- **Favorite**: A saved remote location associated with a connection profile, with a label and ordering used for quick access.
- **Preference**: A user setting such as theme, hidden-file visibility, default concurrency limit, bandwidth limit, preview behavior, connection timeout, and default local location.

### Out of Scope for the Initial Release

- WebDAV and cloud storage protocols, including Amazon S3.
- Directory synchronization and automated scheduled transfers.
- Automated directory comparison or diffing between panels.
- Per-transfer bandwidth limits; the initial release provides one global limit.
- Remote file editing and an integrated terminal.
- Mobile applications.
- Built-in remote server management.
- Mandatory cloud synchronization of connection profiles.
- Built-in telemetry that users cannot disable.

### Open Product Decisions

- **Accessibility conformance target**: The specification requires accessible typography, contrast, and interaction elements but does not adopt a formal accessibility standard or conformance level. A target must be chosen before release criteria are finalized.
- **Transfer history retention**: Transfer history persists across restarts, but the retention period or size limit is not specified. A default must be chosen and documented.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: A new user can install and launch the application on macOS, Linux, and Windows using the published instructions, with 100% of supported platforms producing a working build.
- **SC-002**: A user can create a connection profile and reach a remote home directory in under 60 seconds per protocol when valid credentials are available.
- **SC-003**: For a local directory containing 10,000 entries, the first entries are visible within 1 second and the complete listing is available within 5 seconds on a typical professional workstation.
- **SC-004**: For a remote directory containing 10,000 entries on a controlled test server with at most 50 ms round-trip latency, the first entries are visible within 2 seconds of the first server response and the complete listing is available within 30 seconds.
- **SC-005**: The interface remains interactive and does not exhaust memory while transferring a file larger than the machine's available memory.
- **SC-006**: The application supports at least four concurrent transfers by default, with a user-configurable limit, without blocking browsing or navigation.
- **SC-007**: A cancelled transfer stops within 2 seconds of the user's action and is reported as cancelled.
- **SC-008**: A paused transfer consumes no measurable transfer bandwidth, preserves its progress, and resumes from its last confirmed position.
- **SC-009**: With a global limit of 1 MB/s, observed aggregate transfer throughput does not exceed 1 MB/s over a 60-second measurement.
- **SC-010**: When queued transfers are reordered or reprioritized, the next available transfer slot goes to the highest-priority eligible transfer in 100% of tests.
- **SC-011**: Pausing the queue stops all active transfers within 2 seconds and prevents pending transfers from starting until the queue is resumed.
- **SC-012**: For protocols and servers that support resumption, an interrupted transfer resumes from the last confirmed position rather than restarting from zero.
- **SC-013**: A resume attempt after the source size or modification date changes is refused in 100% of tests, with restart and cancel offered.
- **SC-014**: After a restart, the transfer queue and transfer history are present, and 100% of previously active transfers appear as interrupted with their last confirmed progress.
- **SC-015**: 100% of transfer records expose exactly one unambiguous state from pending, active, paused, completed, failed, cancelled, or interrupted.
- **SC-016**: A security review finds no plaintext credentials in profile storage, logs, telemetry, or error output.
- **SC-017**: 100% of supported-platform builds pass the automated test suite in continuous integration before a release is published.
- **SC-018**: A user can complete the core workflow — connect, browse both panels, upload a file, download a file — without consulting protocol-specific documentation.
- **SC-019**: A new contributor can follow the published build instructions and produce a running development build on each supported platform without maintainer assistance.
- **SC-020**: Users can perform all core file operations with no account, subscription, or remote service, including when working entirely with local files.
- **SC-021**: A selected file opens in the native operating system preview on macOS, and the internal preview fallback is available on every supported platform where the native capability is absent.

## Assumptions

- Users have network access to the servers they connect to and hold valid credentials or keys.
- Servers generally conform to their protocol standards; deviations are handled gracefully rather than assumed away.
- macOS is the primary design reference, with platform-specific adaptations for Linux and Windows considered from the beginning.
- The default transfer concurrency is four, configurable by the user within a safe range.
- The global bandwidth limit is configurable and includes an unlimited option; per-transfer limits are out of scope for the initial release.
- Transfer prioritization is expressed by reordering queued transfers and assigning priority levels.
- Hidden files are hidden by default and can be revealed per panel.
- Resumability depends on protocol and server support; the application exposes it when available rather than promising it universally.
- The transfer queue and transfer history are stored locally and persist across restarts.
- Partial local files are retained for possible resume and can be discarded by the user; remote cleanup depends on protocol support.
- Connection profiles, favorites, and preferences are stored locally for a single desktop user; no multi-user or shared-profile model is required.
- The initial user interface is presented in English.
- The initial release targets desktop use only; mobile and web clients are out of scope.
- Native file preview integration is available on macOS; other platforms use the internal preview fallback when the native capability is absent.
- Technology and architecture choices are deferred to the planning phase; this specification defines observable behavior only.
- Users are responsible for the servers they connect to and for complying with those servers' policies.
