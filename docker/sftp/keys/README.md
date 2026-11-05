# Test-only SSH fixtures

These keys are used only by the containerized SFTP integration tests:

- `ssh_host_ed25519_key`: host key mounted into the test container.
- `known_hosts`: client-side trust entry for `[127.0.0.1]:2222`.
- `client_ed25519` / `.pub`: key-auth fixture; passphrase `testpass`.

They are public test fixtures and must never be used for a real server or
account.
