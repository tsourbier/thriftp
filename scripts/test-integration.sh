#!/usr/bin/env bash
set -euo pipefail

COMPOSE_FILE="${COMPOSE_FILE:-docker/protocol/docker-compose.yml}"

if ! command -v docker >/dev/null 2>&1; then
  echo "SKIP: no Docker-compatible container runtime found; unit tests remain available." >&2
  exit 0
fi

if ! docker info >/dev/null 2>&1; then
  echo "SKIP: Docker is installed but the daemon is unavailable; unit tests remain available." >&2
  exit 0
fi

cleanup() {
  docker compose -f "$COMPOSE_FILE" down -v >/dev/null 2>&1 || true
}
trap cleanup EXIT

compose() {
  docker compose -f "$COMPOSE_FILE" "$@"
}

restart_service() {
  # vsftpd can crash on TLS shutdown in some container builds. Tests are
  # grouped so a fresh server is available for every TLS scenario.
  compose up -d --force-recreate --wait "$1"
}

run() {
  cargo test -p thriftp-protocols "$@"
}

compose up -d --build --wait

export THRIFTP_INTEGRATION=1
export THRIFTP_FTP_HOST="${THRIFTP_FTP_HOST:-127.0.0.1}"
export THRIFTP_FTP_PORT="${THRIFTP_FTP_PORT:-2121}"
export THRIFTP_FTPS_EXPLICIT_PORT="${THRIFTP_FTPS_EXPLICIT_PORT:-2122}"
export THRIFTP_FTPS_IMPLICIT_PORT="${THRIFTP_FTPS_IMPLICIT_PORT:-2990}"
export THRIFTP_SFTP_HOST="${THRIFTP_SFTP_HOST:-127.0.0.1}"
export THRIFTP_SFTP_PORT="${THRIFTP_SFTP_PORT:-2222}"
export THRIFTP_TLS_CA="${THRIFTP_TLS_CA:-docker/protocol/certs/server.crt}"
export THRIFTP_KNOWN_HOSTS="${THRIFTP_KNOWN_HOSTS:-docker/sftp/keys/known_hosts}"
export THRIFTP_SFTP_KEY_PATH="${THRIFTP_SFTP_KEY_PATH:-docker/sftp/keys/client_ed25519}"
export THRIFTP_SFTP_KEY_PASSPHRASE="${THRIFTP_SFTP_KEY_PASSPHRASE:-testpass}"

run --test ftp_parse
run --test connection_integration ftp_ -- --test-threads=1
run --test connection_integration sftp_ -- --test-threads=1
run --test transfer_integration local_copy -- --test-threads=1
restart_service ftp
run --test transfer_integration ftp_ -- --test-threads=1
run --test transfer_integration sftp_ -- --test-threads=1

restart_service ftps-explicit
run --test connection_integration explicit_tls_failure -- --test-threads=1
restart_service ftps-explicit
run --test connection_integration ftps_explicit -- --test-threads=1

restart_service ftps-implicit
run --test connection_integration ftps_implicit -- --test-threads=1
