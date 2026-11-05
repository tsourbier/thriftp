# Protocol integration harness

The container harness is only used by tests. Thriftp never requires Docker at runtime.

```sh
scripts/test-integration.sh
```

Endpoints:

| Protocol | Host | Port |
|----------|------|------|
| FTP | 127.0.0.1 | 2121 |
| FTPS explicit | 127.0.0.1 | 2122 |
| FTPS implicit | 127.0.0.1 | 2990 |
| SFTP | 127.0.0.1 | 2222 |

Credentials: `tester` / `testpass`. TLS uses `certs/server.crt`; SFTP uses the host key in
`../sftp/keys/` and the client key `../sftp/keys/client_ed25519` (passphrase `testpass`).
