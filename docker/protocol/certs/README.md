# Test-only TLS fixtures

`server.crt` and `server.key` are self-signed certificates used exclusively by
the containerized FTPS integration tests. They are public test fixtures and must
never be used for a real server.

The certificate is valid for `localhost` and `127.0.0.1` for 365 days and
includes the server-auth extended key usage required by modern TLS stacks.
