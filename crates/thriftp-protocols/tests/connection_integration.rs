mod support;

use std::path::PathBuf;

use support::{env_port, env_string, fixture_path, integration_enabled, profile, unique_name};
use thriftp_core::credentials::Secret;
use thriftp_core::error::CoreError;
use thriftp_core::model::{AuthMethod, Protocol};
use thriftp_core::protocol::RemoteFs;
use thriftp_protocols::{FtpAdapter, SftpAdapter};

const USERNAME: &str = "tester";
const PASSWORD: &str = "testpass";

fn skip(test: &str) {
    eprintln!("SKIP {test}: set THRIFTP_INTEGRATION=1 with the protocol containers running");
}

fn ftp_host() -> String {
    env_string("THRIFTP_FTP_HOST", "127.0.0.1")
}

async fn exercise(adapter: &mut dyn RemoteFs) {
    let directory = unique_name();
    adapter.list_dir(".").await.unwrap();

    adapter.mkdir(&directory).await.unwrap();
    let renamed = format!("{directory}-renamed");
    adapter.rename(&directory, &renamed).await.unwrap();
    adapter.delete_dir(&renamed).await.unwrap();
}

#[tokio::test]
async fn ftp_round_trip() {
    if !integration_enabled() {
        return skip("ftp_round_trip");
    }
    let profile = profile(
        Protocol::Ftp,
        &ftp_host(),
        env_port("THRIFTP_FTP_PORT", 2121),
        USERNAME,
        AuthMethod::Password,
        None,
    );
    let mut adapter = FtpAdapter::new(Protocol::Ftp);
    adapter
        .connect(&profile, Some(Secret::new(PASSWORD.as_bytes().to_vec())))
        .await
        .unwrap();
    exercise(&mut adapter).await;
}

#[tokio::test]
async fn ftps_explicit_round_trip() {
    if !integration_enabled() {
        return skip("ftps_explicit_round_trip");
    }
    let profile = profile(
        Protocol::FtpsExplicit,
        &ftp_host(),
        env_port("THRIFTP_FTPS_EXPLICIT_PORT", 2122),
        USERNAME,
        AuthMethod::Password,
        None,
    );
    let mut adapter = FtpAdapter::new(Protocol::FtpsExplicit).with_tls_ca(fixture_path(
        &env_string("THRIFTP_TLS_CA", "docker/protocol/certs/server.crt"),
    ));
    adapter
        .connect(&profile, Some(Secret::new(PASSWORD.as_bytes().to_vec())))
        .await
        .unwrap();
    exercise(&mut adapter).await;
}

#[tokio::test]
async fn ftps_implicit_round_trip() {
    if !integration_enabled() {
        return skip("ftps_implicit_round_trip");
    }
    let profile = profile(
        Protocol::FtpsImplicit,
        &ftp_host(),
        env_port("THRIFTP_FTPS_IMPLICIT_PORT", 2990),
        USERNAME,
        AuthMethod::Password,
        None,
    );
    let mut adapter = FtpAdapter::new(Protocol::FtpsImplicit).with_tls_ca(fixture_path(
        &env_string("THRIFTP_TLS_CA", "docker/protocol/certs/server.crt"),
    ));
    adapter
        .connect(&profile, Some(Secret::new(PASSWORD.as_bytes().to_vec())))
        .await
        .unwrap();
    exercise(&mut adapter).await;
}

#[tokio::test]
async fn ftp_rejects_bad_credentials() {
    if !integration_enabled() {
        return skip("ftp_rejects_bad_credentials");
    }
    let profile = profile(
        Protocol::Ftp,
        &ftp_host(),
        env_port("THRIFTP_FTP_PORT", 2121),
        USERNAME,
        AuthMethod::Password,
        None,
    );
    let mut adapter = FtpAdapter::new(Protocol::Ftp);
    let error = adapter
        .connect(&profile, Some(Secret::new(b"wrong".to_vec())))
        .await
        .unwrap_err();
    assert_eq!(error, CoreError::CredentialRejected);
}

#[tokio::test]
async fn explicit_tls_failure_is_distinct() {
    if !integration_enabled() {
        return skip("explicit_tls_failure_is_distinct");
    }
    // Connect without trusting the harness CA: the certificate must be rejected.
    let profile = profile(
        Protocol::FtpsExplicit,
        &ftp_host(),
        env_port("THRIFTP_FTPS_EXPLICIT_PORT", 2122),
        USERNAME,
        AuthMethod::Password,
        None,
    );
    let mut adapter = FtpAdapter::new(Protocol::FtpsExplicit);
    let error = adapter
        .connect(&profile, Some(Secret::new(PASSWORD.as_bytes().to_vec())))
        .await
        .unwrap_err();
    assert_eq!(error.code(), "certificate_invalid");
}

#[tokio::test]
async fn sftp_password_round_trip() {
    if !integration_enabled() {
        return skip("sftp_password_round_trip");
    }
    let profile = profile(
        Protocol::Sftp,
        &env_string("THRIFTP_SFTP_HOST", "127.0.0.1"),
        env_port("THRIFTP_SFTP_PORT", 2222),
        USERNAME,
        AuthMethod::Password,
        None,
    );
    let mut adapter = SftpAdapter::new().with_known_hosts_path(fixture_path(&env_string(
        "THRIFTP_KNOWN_HOSTS",
        "docker/sftp/keys/known_hosts",
    )));
    adapter
        .connect(&profile, Some(Secret::new(PASSWORD.as_bytes().to_vec())))
        .await
        .unwrap();
    exercise(&mut adapter).await;
}

#[tokio::test]
async fn sftp_private_key_auth() {
    if !integration_enabled() {
        return skip("sftp_private_key_auth");
    }
    let key_path = fixture_path(&env_string(
        "THRIFTP_SFTP_KEY_PATH",
        "docker/sftp/keys/client_ed25519",
    ));
    let passphrase = env_string("THRIFTP_SFTP_KEY_PASSPHRASE", "testpass");
    let profile = profile(
        Protocol::Sftp,
        &env_string("THRIFTP_SFTP_HOST", "127.0.0.1"),
        env_port("THRIFTP_SFTP_PORT", 2222),
        &env_string("THRIFTP_SFTP_KEY_USER", USERNAME),
        AuthMethod::SshPrivateKey,
        Some(key_path.to_str().unwrap()),
    );
    let mut adapter = SftpAdapter::new().with_known_hosts_path(fixture_path(&env_string(
        "THRIFTP_KNOWN_HOSTS",
        "docker/sftp/keys/known_hosts",
    )));
    adapter
        .connect(&profile, Some(Secret::new(passphrase.into_bytes())))
        .await
        .unwrap();
    exercise(&mut adapter).await;
}

#[tokio::test]
async fn sftp_rejects_unknown_host_keys() {
    if !integration_enabled() {
        return skip("sftp_rejects_unknown_host_keys");
    }
    let directory = tempfile::tempdir().unwrap();
    let known_hosts: PathBuf = directory.path().join("known_hosts");
    std::fs::write(&known_hosts, b"").unwrap();

    let profile = profile(
        Protocol::Sftp,
        &env_string("THRIFTP_SFTP_HOST", "127.0.0.1"),
        env_port("THRIFTP_SFTP_PORT", 2222),
        USERNAME,
        AuthMethod::Password,
        None,
    );
    let mut adapter = SftpAdapter::new().with_known_hosts_path(known_hosts);
    let error = adapter
        .connect(&profile, Some(Secret::new(PASSWORD.as_bytes().to_vec())))
        .await
        .unwrap_err();
    assert!(matches!(
        error,
        CoreError::HostKeyUnknown { fingerprint, .. } if !fingerprint.is_empty()
    ));
}

#[tokio::test]
async fn sftp_rejects_bad_credentials() {
    if !integration_enabled() {
        return skip("sftp_rejects_bad_credentials");
    }
    // Accept any well-formed key first so authentication, not host-key policy,
    // is exercised. The container key is learned from the mounted fixture.
    let profile = profile(
        Protocol::Sftp,
        &env_string("THRIFTP_SFTP_HOST", "127.0.0.1"),
        env_port("THRIFTP_SFTP_PORT", 2222),
        USERNAME,
        AuthMethod::Password,
        None,
    );
    let mut adapter = SftpAdapter::new().with_known_hosts_path(fixture_path(&env_string(
        "THRIFTP_KNOWN_HOSTS",
        "docker/sftp/keys/known_hosts",
    )));
    let error = adapter
        .connect(&profile, Some(Secret::new(b"wrong".to_vec())))
        .await
        .unwrap_err();
    assert_eq!(error, CoreError::CredentialRejected);
}
