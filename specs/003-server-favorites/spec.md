# Feature Specification: Server Favorites Manager

**Feature Branch**: `003-server-favorites`

**Created**: 2026-11-05

**Status**: Draft

**Input**: User description: "Introduce a server favorites feature that lets users save, organize, and quickly reconnect to frequently used FTP, FTPS, and SFTP servers while credentials remain protected by the operating system's secure credential store."

## Clarifications

### Session 2026-11-05

- Q: When a user deletes a server favorite, should the underlying saved connection also be deleted? → A: Delete the saved connection; active sessions remain and credentials are deleted only when no other favorite references them.
- Q: Should every saved server connection appear in the favorites list automatically, or only explicitly marked favorites? → A: Every saved connection appears automatically, with optional pinning to place favorites at the top.
- Q: When a saved credential is shared by multiple favorites, should updating it change the secret for all of them or create a separate credential? → A: Update all favorites that reference it, and list them before confirmation.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Save a server as a favorite (Priority: P1)

A user saves a server connection as a favorite so that they can reconnect later without re-entering the server's connection details.

**Why this priority**: Saving a reusable server is the foundation of the feature; without it, there is nothing to browse, organize, or reconnect to.

**Independent Test**: Create a favorite from scratch and from an existing active connection, restart the application, and confirm both appear in the favorites list with the correct connection settings.

**Acceptance Scenarios**:

1. **Given** no saved favorites, **when** the user creates a favorite with a display name and valid connection details, **then** the favorite is saved and appears in the favorites list.
2. **Given** an FTP, explicit FTPS, implicit FTPS, or SFTP server, **when** the user saves it as a favorite, **then** the favorite records the protocol, host, port, username, authentication method, and protocol-specific settings.
3. **Given** an active connection, **when** the user chooses to save it as a favorite, **then** the current connection settings are pre-filled and the user can confirm or adjust them before saving.
4. **Given** a favorite creation form, **when** the user chooses not to save credentials, **then** the favorite is saved without a stored secret and the user is told that credentials will be requested when connecting.
5. **Given** a favorite with saved credentials, **when** the user saves it, **then** the secret is stored only in the operating system credential store and the application configuration contains only a reference.
6. **Given** a favorite, **when** the application restarts, **then** the favorite and its non-secret settings remain available.

---

### User Story 2 - Browse and organize favorites (Priority: P1)

A user opens a dedicated favorites list, identifies servers by name and connection details, sorts the list predictably, and optionally organizes entries into simple groups.

**Why this priority**: A favorites feature is only useful if users can find the right server quickly as the list grows.

**Independent Test**: Create several favorites, sort them by each supported order, pin one, move one into a group, rename it, and delete another while confirming the remaining favorites and their credentials stay intact.

**Acceptance Scenarios**:

1. **Given** saved favorites, **when** the user opens the favorites view, **then** each favorite displays its display name, protocol, host, username, and an indication of whether credentials are saved.
2. **Given** the favorites list, **when** the user sorts by name, protocol, host, or last used, **then** the list reorders predictably with a stable tie-breaker.
3. **Given** a favorite, **when** the user renames it, **then** the new name is reflected in the list without changing its connection settings or credentials.
4. **Given** a favorite, **when** the user edits its connection settings, **then** the changes are saved and the list reflects them.
5. **Given** a favorite, **when** the user deletes it, **then** the favorite is removed after confirmation and other favorites and their credentials remain intact.
6. **Given** the user chooses to organize favorites, **when** the user assigns a group label and moves a favorite into it, **then** the favorite appears under that group.
7. **Given** a favorites list with some pinned entries, **when** the list is displayed, **then** pinned entries appear before unpinned entries while the selected sort order applies within each section.
8. **Given** an empty favorites list, **when** the user opens the view, **then** an empty state explains how to create the first favorite.

---

### User Story 3 - Connect quickly to a favorite (Priority: P1)

A user selects a favorite and connects with a single action, using saved credentials automatically when available and being prompted only when necessary.

**Why this priority**: Fast reconnection is the main user benefit of saving a server.

**Independent Test**: Connect from a favorite with saved credentials, then with missing credentials, then after a connection failure, and confirm each path gives clear feedback without exposing secrets.

**Acceptance Scenarios**:

1. **Given** a favorite with saved credentials, **when** the user selects the connect action, **then** the connection starts without asking for connection details or credentials.
2. **Given** a favorite without saved credentials, **when** the user selects the connect action, **then** the application requests the missing credentials before connecting.
3. **Given** a favorite whose stored credential is missing, rejected, or unavailable, **when** the user connects, **then** the application requests new credentials and does not reuse an incompatible secret.
4. **Given** a connection attempt, **when** it fails, **then** the application reports the failure clearly without exposing credentials.
5. **Given** an active connection to a favorite, **when** the user disconnects and reconnects, **then** the favorite remains available and unchanged.
6. **Given** an active connection, **when** the user edits or deletes the corresponding favorite, **then** the active connection is not interrupted.

---

### User Story 4 - Manage credentials securely (Priority: P1)

A user saves, updates, or removes the credentials associated with a favorite while the operating system protects the secret material.

**Why this priority**: Convenient reconnection must never come at the cost of exposing passwords or private keys.

**Independent Test**: Save, update, and remove a password and an SSH key reference, simulate an unavailable credential store, and confirm no plaintext secret appears in configuration, logs, errors, or diagnostics.

**Acceptance Scenarios**:

1. **Given** a favorite, **when** the user chooses to save a password, **then** the password is stored in the operating system credential store and the favorite stores only a reference.
2. **Given** an SFTP favorite using private-key authentication, **when** the user saves it, **then** the private key is not copied into ordinary application configuration and only a protected reference is retained.
3. **Given** a saved credential, **when** the user updates it, **then** the new secret replaces the old one for every favorite that references it.
4. **Given** a saved credential, **when** the user removes it, **then** the favorite remains usable but requests credentials on the next connection.
5. **Given** an unavailable operating-system credential store, **when** the user attempts to save or retrieve credentials, **then** the application explains the failure and does not fall back to plaintext storage.
6. **Given** any credential operation, **when** it fails, **then** no password, private key, passphrase, or other secret appears in logs, errors, diagnostics, crash reports, or telemetry.

---

### User Story 5 - Edit connection settings safely (Priority: P2)

A user updates a favorite when a server's connection details or authentication method change, without accidentally reusing credentials that belong to the old identity.

**Why this priority**: Server details change over time; editing must be safe and must not silently connect with stale credentials.

**Independent Test**: Change a favorite's username and authentication method, verify that old credentials are not reused, test the connection before saving, and confirm an existing active connection is unaffected.

**Acceptance Scenarios**:

1. **Given** a favorite, **when** the user edits its display name or connection settings, **then** the changes are saved without affecting other favorites.
2. **Given** a favorite using password authentication, **when** the user changes the authentication method to an SSH private key, **then** the application does not reuse the saved password.
3. **Given** a favorite, **when** the user changes the username, host, port, protocol, or authentication method, **then** the previous credential reference is not reused and the user is prompted to provide compatible credentials or clear the saved credential.
4. **Given** unsaved edits, **when** the user chooses to test the connection, **then** the application tests the pending settings without persisting credentials entered only for the test.
5. **Given** an active connection, **when** the user saves edits to its favorite, **then** the active connection continues with its original settings and new connections use the updated settings.

---

### Edge Cases

- The operating system credential store is locked, denied, or unavailable; the application must explain the failure and must not write secrets to a plaintext fallback.
- A saved credential is missing, rejected by the server, or removed outside the application; the application must request new credentials and must not silently retry with an incompatible secret.
- A favorite has no saved credentials; connecting must request the missing secret without altering the saved favorite unless the user chooses to save it.
- Two favorites have the same display name; the list must still distinguish them by protocol, host, and username.
- Two favorites reference the same credential; updating or deleting it must show which favorites are affected before proceeding.
- The user deletes a favorite whose credential is still referenced by another favorite; the shared credential must remain available.
- The user deletes the last favorite referencing a credential; the application must ask whether to delete or keep the credential.
- The user changes the host, port, protocol, username, or authentication method while editing a favorite; the old credential must not be reused silently.
- An SSH private-key file referenced by a favorite is moved or deleted; connecting must report the missing key and request a new reference.
- An FTPS favorite changes between explicit and implicit mode; the connection settings must reflect the change and credentials must remain bound to the correct identity.
- An active connection exists while its favorite is edited or deleted; the active connection must not be interrupted.
- The application terminates unexpectedly during a favorite save; the favorites list must not contain a partially written entry that points to a missing or corrupt credential reference.
- The favorites list is very large; browsing and sorting must remain responsive.
- A credential reference is corrupt or points to a missing entry; the favorite must remain visible with a clear repair or re-enter-credentials action.
- The user switches between keyboard-only navigation and pointer input; the favorites view must remain fully operable.
- A favorite group label is removed from the last favorite using it; the favorites must remain available in an ungrouped state.
- A connection profile referenced by directory favorites or open tabs is deleted as part of removing a server favorite; dependent references must be handled gracefully without deleting unrelated data or interrupting active connections.

## Requirements *(mandatory)*

### Terminology

- **Server Favorite**: Every saved server connection managed through the favorites view. It reuses the existing Connection Profile and Credential concepts and adds favorite-specific metadata such as group label, pinned state, sort order, last-used time, and credential policy.
- **Directory Favorite**: A saved remote directory associated with a connection profile, as defined by the desktop specification. Directory favorites are distinct from server favorites and are unchanged by this feature.
- **Active Connection**: A live session to a server. It is distinct from the saved server favorite that may have been used to create it.
- **Credential Reference**: A stable identifier that links a server favorite to secret material held in the operating system credential store. The reference is not itself a secret.
- **Favorite Group**: The optional single-level group label on a Connection Profile, presented as a group in the favorites view. This feature does not introduce a separate group record or empty groups.

### Functional Requirements

**Favorite Creation and Management**

- **FR-001**: Users MUST be able to create a server favorite by providing a display name and valid connection details.
- **FR-002**: Server favorites MUST support FTP, explicit FTPS, implicit FTPS, and SFTP.
- **FR-003**: A server favorite MUST record the protocol, host, port, username, authentication method, connection timeout, FTPS mode where applicable, and SSH key reference where applicable.
- **FR-004**: Users MUST be able to save an existing active connection as a server favorite with its current settings pre-filled.
- **FR-005**: Users MUST be able to edit a server favorite's display name, connection settings, and authentication method where the protocol supports it.
- **FR-006**: Users MUST be able to rename a server favorite without changing its connection settings or credentials.
- **FR-007**: Users MUST be able to delete a server favorite after explicit confirmation; deletion MUST remove the saved connection profile and its favorite metadata while leaving any active connection running.
- **FR-008**: Every saved connection profile MUST have exactly one server favorite entry; duplicate favorite entries for the same profile are prohibited.
- **FR-009**: Display names MAY repeat; the favorites list MUST also show protocol, host, and username so users can distinguish entries.
- **FR-010**: Server favorites MUST persist across application restarts.
- **FR-011**: Users MUST be able to assign server favorites to an optional single-level group label, rename the group label for all assigned favorites, and remove the label from favorites.
- **FR-012**: Users MUST be able to move favorites between group labels and back to an ungrouped state.
- **FR-013**: Removing a group label MUST NOT delete its favorites; affected favorites MUST remain available in an ungrouped state.
- **FR-014**: The favorites list MUST support sorting by display name, protocol, host, and last used (most recent first); pinned entries MUST appear before unpinned entries, with the selected sort order applied within each section.
- **FR-015**: The default order MUST place pinned entries first, then sort each section by display name ascending, case-insensitive, with protocol, host, and username as stable tie-breakers.

**Favorites List and User Experience**

- **FR-016**: The application MUST provide a dedicated favorites view or sidebar section that lists every saved server connection; there is no separate non-favorite saved-server state in this feature.
- **FR-017**: Each favorite entry MUST display its display name, protocol, host, username, and whether credentials are saved.
- **FR-018**: Each favorite MUST show a visible protocol indicator.
- **FR-019**: The interface MUST clearly distinguish saved server favorites from currently active connections.
- **FR-020**: Users MUST be able to connect, edit, and delete a favorite from the favorites list.
- **FR-021**: The favorites view MUST provide an empty state with a clear action to create the first favorite.
- **FR-022**: The favorites view MUST support keyboard navigation and accessible interaction elements.
- **FR-023**: When a connection requires credentials, the application MUST clearly indicate which favorite needs them and what kind of credential is required.
- **FR-024**: Connection and credential errors MUST be reported clearly and MUST NOT expose secrets.
- **FR-025**: Users MUST be able to test a favorite's connection settings before saving changes.
- **FR-026**: Testing a connection MUST NOT persist credentials entered solely for the test.

**Quick Connect and Active Connections**

- **FR-027**: Selecting the connect action for a favorite MUST initiate a connection using its saved settings with a single user action.
- **FR-028**: When saved credentials are available, the application MUST retrieve them automatically for the connection.
- **FR-029**: When credentials are missing, rejected, or unavailable, the application MUST request them before connecting.
- **FR-030**: Users MUST be able to disconnect and reconnect to a favorite without losing or modifying the favorite.
- **FR-031**: Editing a favorite MUST NOT interrupt an existing active connection that used it.
- **FR-032**: Deleting a favorite MUST NOT disconnect an existing active connection that used it.
- **FR-033**: New connections started after an edit MUST use the updated settings, while existing active connections retain their original settings.
- **FR-034**: Connection attempts from a favorite MUST use the same certificate and SSH host-key verification rules as any other connection.

**Credential Storage and Retrieval**

- **FR-035**: Users MUST be able to choose, per favorite, whether credentials are saved securely or requested at connection time.
- **FR-036**: Saved credentials MUST be stored in the operating system credential store: macOS Keychain, Windows Credential Manager, or Linux Secret Service or an equivalent supported mechanism.
- **FR-037**: Application configuration MUST store only a credential reference and MUST NOT store passwords, private keys, passphrases, or other secret material.
- **FR-038**: SSH private keys MUST NOT be copied into ordinary application configuration; the favorite MUST retain only a protected reference or the original protected location.
- **FR-039**: Server favorites MUST support password authentication and SSH private-key authentication for SFTP.
- **FR-040**: When connecting, the application MUST retrieve saved credentials from the operating system credential store using the favorite's credential reference.
- **FR-041**: When a credential is missing, rejected, or unavailable, the application MUST request new credentials and MUST NOT automatically reuse an incompatible credential.
- **FR-042**: Users MUST be able to update a saved credential.
- **FR-043**: Users MUST be able to remove a saved credential while keeping the favorite; when the credential is shared, the application MUST list the affected favorites before removal.
- **FR-044**: A credential reference MUST be bound to the protocol, host, port, username, and authentication method it was saved for.
- **FR-045**: Changing any identity-defining field of a favorite MUST invalidate the previous credential reference and MUST require new credentials or an explicit decision to clear the saved credential.
- **FR-046**: Each favorite with saved credentials MUST have its own credential reference by default; users MAY explicitly reuse an existing credential for a favorite only when the identity-defining fields match.
- **FR-047**: When a credential reference is shared, updating it MUST replace the secret for every favorite that references it, and the confirmation MUST list the affected favorites before the change is applied.
- **FR-048**: Deleting a favorite MUST remove its credential reference but MUST NOT delete a credential that is still referenced by another favorite.
- **FR-049**: When deleting the last favorite referencing a credential, the application MUST ask whether to delete or keep the credential; the confirmation MUST state the consequence and MUST NOT delete it silently.
- **FR-050**: If the operating system credential store is unavailable, the application MUST explain the failure and MUST NOT fall back to plaintext storage.
- **FR-051**: If the credential store is unavailable while saving a favorite, the application MUST save only the favorite's non-secret settings, MUST inform the user that credentials were not saved, and MUST NOT store the secret anywhere.
- **FR-052**: If the credential store is unavailable while connecting, the application MUST accept credentials for the current session only after a clear warning, MUST NOT persist them, and MUST discard them when the session ends.
- **FR-053**: Passwords, private keys, passphrases, and other secrets MUST NOT appear in application logs, error messages, diagnostic output, crash reports, or telemetry.
- **FR-054**: The feature MUST NOT introduce a Thriftp-specific master password or custom credential vault.

**Editing, Deletion, and Shared Credentials**

- **FR-055**: Editing a favorite's display name MUST NOT change its credential binding.
- **FR-056**: Changing a favorite's protocol, host, port, username, or authentication method MUST NOT silently reuse the previous credential.
- **FR-057**: Splitting a shared credential into a separate credential reference MUST require an explicit user action and MUST NOT happen as a side effect of editing a favorite.
- **FR-058**: Deleting or editing a favorite MUST NOT expose or destroy credentials belonging to another favorite.
- **FR-059**: The application MUST recover safely from a favorite record that references a missing or corrupt credential by keeping the favorite visible and offering repair or credential re-entry.

**Security and Privacy**

- **FR-060**: The feature MUST use the native secure credential facilities of each supported operating system.
- **FR-061**: The feature MUST NOT store secrets in plaintext configuration files, logs, errors, diagnostics, crash reports, or telemetry.
- **FR-062**: The feature MUST NOT silently downgrade to insecure credential storage when a secure backend is unavailable.
- **FR-063**: The feature MUST apply the principle of least privilege to credential access and favorite management operations.
- **FR-064**: The feature MUST NOT introduce cloud synchronization, credential sharing between devices, or a custom password vault.

**Persistence and Compatibility**

- **FR-065**: Favorites, group labels, pinned state, sort preferences, and last-used information MUST persist across restarts.
- **FR-066**: A server favorite MUST remain distinct from any active connection created from it.
- **FR-067**: The server favorite data model MUST be reusable by future CLI and MCP interfaces, even though managing server favorites through those interfaces is out of scope for this feature.
- **FR-068**: Managing server favorites through CLI or MCP MUST NOT be exposed in the initial release without a separate specification.
- **FR-069**: The feature MUST reuse the existing connection-profile and credential concepts rather than creating a parallel connection model.
- **FR-070**: The favorites list MUST remain responsive when it contains a large number of entries.

**Pinning**

- **FR-071**: Users MUST be able to pin and unpin a server favorite without changing its connection settings or credentials.
- **FR-072**: Pinned state MUST persist across application restarts.

### Key Entities *(include if feature involves data)*

- **Server Favorite**: The favorites-facing record for a saved server connection. Includes a display name, optional group label inherited from the Connection Profile, pinned state, sort position, last-used time (the most recent successful connection), credential policy, and a reference to the underlying Connection Profile. Every saved Connection Profile has exactly one Server Favorite.
- **Connection Profile**: The existing saved server configuration, reused by this feature. Holds protocol, host, port, username, authentication method, timeout, FTPS mode, SSH key reference, and connection options.
- **Credential Reference**: A stable identifier linking a Server Favorite to secret material in the operating system credential store. Bound to protocol, host, port, username, and authentication method; never contains the secret itself. A reference may be shared, and updating it affects every favorite that uses it.
- **Credential**: A password, private key, passphrase, or other authentication secret held by the operating system credential store. Not stored in application configuration.
- **Favorite Group Label**: The optional single-level group label on a Connection Profile, used to group entries in the favorites view. No separate group record or empty group is persisted.
- **Active Connection**: A live session created from a Server Favorite. It has its own lifecycle and is not modified by later favorite edits or deletion.
- **Favorite Preference**: A user setting for sorting, grouping visibility, or other favorites-view behavior.

### Out of Scope for the Initial Release

- Cloud synchronization of server favorites.
- Sharing server favorites between users, devices, or teams.
- Organization-wide server directories.
- Credential synchronization between operating systems or devices.
- A custom password vault or Thriftp-specific master password.
- Automatic discovery of servers on the network.
- Server administration features such as managing users, services, or remote configuration.
- Nested favorite groups, bulk import/export, or bulk editing.
- Managing server favorites through CLI or MCP in the initial release.
- Changes to directory favorites, which remain a separate desktop feature.

### Unresolved Decisions for Planning

- Whether to expose server favorite management through CLI or MCP later, and if so, what authorization model would apply.
- Whether future releases will support nested groups, bulk import/export, or bulk editing.
- The exact credential-store abstraction and recovery behavior when a platform backend is unavailable, subject to the security constraints in this specification.

### Proposed Constitutional Changes

None. The feature follows the existing secure-credential, privacy, and least-privilege principles. If server favorite management is later exposed through CLI or MCP, that work must follow the constitution's Automation & Integration principle rather than requiring an amendment now.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: A user can save a favorite with a name and connection details and reconnect later without re-entering the connection settings.
- **SC-002**: A favorite with saved credentials connects with one user action in 100% of tests on macOS, Linux, and Windows.
- **SC-003**: 100% of server favorites persist across application restarts, including their group label, pinned state, sort position, and credential policy.
- **SC-004**: 100% of missing, rejected, or unavailable credentials produce a clear request for new credentials rather than a silent connection attempt.
- **SC-005**: A security review finds zero plaintext passwords, private keys, passphrases, or other secrets in application configuration, logs, errors, diagnostics, crash reports, or telemetry.
- **SC-006**: When the operating system credential store is unavailable, 100% of credential save and retrieval attempts fail safely without plaintext fallback.
- **SC-007**: Editing or deleting one favorite leaves 100% of other favorites and their credentials intact.
- **SC-008**: Deleting a favorite whose credential is still referenced by another favorite preserves the credential in 100% of tests.
- **SC-009**: Deleting the last favorite referencing a credential prompts before deletion and does not delete the credential silently.
- **SC-010**: The favorites list sorts predictably by every supported sort field in 100% of tests, including stable tie-breaking.
- **SC-011**: A user can complete the full favorites workflow — create, find, connect, edit, and delete — using only the keyboard.
- **SC-012**: Testing a connection before saving does not persist credentials entered solely for the test in 100% of tests.
- **SC-013**: Editing or deleting a favorite does not interrupt an existing active connection in 100% of tests.
- **SC-014**: SSH private keys are never copied into ordinary application configuration in 100% of security tests.
- **SC-015**: The favorites view remains responsive with at least 500 saved entries on a typical professional workstation.
- **SC-016**: Assigning, renaming, and removing a group label leaves 100% of affected favorites usable and ungrouped when the label is removed.
- **SC-017**: Pinned favorites appear before unpinned favorites in 100% of tests and remain pinned across restarts.
- **SC-018**: Updating a shared credential updates all favorites that reference it and lists them before confirmation in 100% of tests.

## Assumptions

- A Server Favorite reuses the existing Connection Profile and Credential concepts; it is not a separate connection model.
- The existing desktop specification's "Favorite" entity refers to a saved remote directory. Server favorites are a distinct concept and do not change directory-favorite behavior.
- Credentials are opt-in per favorite and are not saved unless the user chooses to save them.
- Each favorite with saved credentials has its own credential reference by default; sharing is an explicit user choice and is allowed only when protocol, host, port, username, and authentication method match.
- "Expired" or "rejected" credentials mean the stored secret is no longer accepted by the server or has been removed from the credential store; the user is prompted for new credentials.
- Deleting a favorite removes the saved connection profile and its credential reference; an active connection created from it remains active until the user disconnects.
- All saved server connections appear in the favorites list automatically; there is no separate non-favorite saved-server state in this feature.
- Pinning is optional and affects display order only; it does not change connection settings, credentials, or enablement.
- Updating a shared credential replaces the secret for all favorites that reference it; the affected favorites are listed before confirmation.
- Last used means the most recent successful connection to the favorite; failed attempts do not update it.
- Favorite groups reuse the optional group label on the Connection Profile; no separate group entity or empty group is introduced.
- Favorite groups are single-level and optional; nested groups are out of scope.
- The favorites view is desktop-only in the initial release, consistent with the CLI/MCP specification; the data model must remain reusable for future interfaces.
- The implementation will reuse the existing connection-profile and credential abstractions; architecture and platform-backend design are deferred to planning.
- The feature does not require an account, subscription, cloud service, or remote authentication service.
- The operating system credential store may be locked or unavailable; the application must fail safely rather than degrade to plaintext.
- The initial favorites interface is presented in English, consistent with the desktop application.
