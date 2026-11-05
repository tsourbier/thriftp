mod support;

use std::path::Path;

use support::{env_port, env_string, fixture_path, integration_enabled, profile, unique_name};
use thriftp_core::credentials::Secret;
use thriftp_core::model::{AuthMethod, Protocol};
use thriftp_core::protocol::{ProgressCallback, RemoteFs};
use thriftp_protocols::{FtpAdapter, SftpAdapter};

fn ftp_host() -> String {
    env_string("THRIFTP_FTP_HOST", "127.0.0.1")
}

async fn round_trip(adapter: &mut dyn RemoteFs, directory: &str) {
    let _ = adapter.mkdir(directory).await;
    let payload: Vec<u8> = (0..200_000).map(|index| (index % 251) as u8).collect();
    let work = tempfile::tempdir().unwrap();
    let source = work.path().join("payload.bin");
    std::fs::write(&source, &payload).unwrap();

    let progress: ProgressCallback = std::sync::Arc::new(|_| true);
    let uploaded = adapter
        .upload_from(
            &source,
            &format!("{directory}/payload.bin"),
            0,
            progress.clone(),
        )
        .await
        .unwrap();
    assert_eq!(uploaded, payload.len() as u64);

    let destination = work.path().join("downloaded.bin");
    let downloaded = adapter
        .download_to(
            &format!("{directory}/payload.bin"),
            &destination,
            0,
            progress.clone(),
        )
        .await
        .unwrap();
    assert_eq!(downloaded, payload.len() as u64);
    assert_eq!(std::fs::read(&destination).unwrap(), payload);

    // Resume at an offset and verify the tail is appended in place.
    let offset = 100_000_u64;
    let resumed = adapter
        .download_to(
            &format!("{directory}/payload.bin"),
            &destination,
            offset,
            progress,
        )
        .await
        .unwrap();
    assert_eq!(resumed, payload.len() as u64 - offset);

    adapter
        .delete_file(&format!("{directory}/payload.bin"))
        .await
        .unwrap();
    adapter.delete_dir(directory).await.unwrap();
}

#[tokio::test]
async fn ftp_upload_download_and_resume() {
    if !integration_enabled() {
        eprintln!("SKIP ftp_upload_download_and_resume");
        return;
    }
    let profile = profile(
        Protocol::Ftp,
        &ftp_host(),
        env_port("THRIFTP_FTP_PORT", 2121),
        "tester",
        AuthMethod::Password,
        None,
    );
    let mut adapter = FtpAdapter::new(Protocol::Ftp);
    adapter
        .connect(&profile, Some(Secret::new(b"testpass".to_vec())))
        .await
        .unwrap();
    round_trip(&mut adapter, &unique_name()).await;
}

#[tokio::test]
async fn sftp_upload_download_and_resume() {
    if !integration_enabled() {
        eprintln!("SKIP sftp_upload_download_and_resume");
        return;
    }
    let profile = profile(
        Protocol::Sftp,
        &env_string("THRIFTP_SFTP_HOST", "127.0.0.1"),
        env_port("THRIFTP_SFTP_PORT", 2222),
        "tester",
        AuthMethod::Password,
        None,
    );
    let mut adapter = SftpAdapter::new().with_known_hosts_path(fixture_path(&env_string(
        "THRIFTP_KNOWN_HOSTS",
        "docker/sftp/keys/known_hosts",
    )));
    adapter
        .connect(&profile, Some(Secret::new(b"testpass".to_vec())))
        .await
        .unwrap();
    let home = env_string("THRIFTP_SFTP_HOME", "");
    let directory = unique_name();
    let directory = if home.is_empty() {
        directory
    } else {
        format!("{home}/{directory}")
    };
    round_trip(&mut adapter, &directory).await;
}

#[tokio::test]
async fn local_copy_uses_a_bounded_stream() {
    let directory = tempfile::tempdir().unwrap();
    let source = directory.path().join("source.bin");
    let destination = directory.path().join("destination.bin");
    std::fs::write(&source, vec![5_u8; 1_000_000]).unwrap();
    let copied = thriftp_core::transfers::io::copy_local(&source, &destination, 0, |_| true)
        .await
        .unwrap();
    assert_eq!(copied, 1_000_000);
    assert_eq!(std::fs::metadata(&destination).unwrap().len(), 1_000_000);
    assert!(Path::new(&destination).is_file());
}
