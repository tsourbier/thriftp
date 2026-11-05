use thriftp_core::model::{EntryType, ProtocolCapabilities};
use thriftp_protocols::parse_mlsd_line;

#[test]
fn parses_file_facts() {
    let entry = parse_mlsd_line(
        "type=file;size=12;modify=20260101120000;unix.mode=0644; /pub/file.txt",
        "/pub",
        ProtocolCapabilities::default(),
    )
    .unwrap();
    assert_eq!(entry.name, "file.txt");
    assert_eq!(entry.path, "/pub/file.txt");
    assert_eq!(entry.entry_type, EntryType::File);
    assert_eq!(entry.size, Some(12));
    assert_eq!(
        entry.modified_at.unwrap().to_rfc3339(),
        "2026-01-01T12:00:00+00:00"
    );
    assert_eq!(entry.permissions.as_deref(), Some("644"));
}

#[test]
fn parses_directory_and_hidden_entries() {
    let directory = parse_mlsd_line(
        "type=dir; /pub/folder",
        "/pub",
        ProtocolCapabilities::default(),
    )
    .unwrap();
    assert_eq!(directory.entry_type, EntryType::Directory);
    assert_eq!(directory.size, None);

    let hidden = parse_mlsd_line(
        "type=file;size=1; /pub/.hidden",
        "/pub",
        ProtocolCapabilities::default(),
    )
    .unwrap();
    assert!(hidden.hidden);
}

#[test]
fn rejects_malformed_lines() {
    assert!(parse_mlsd_line("not-a-fact-line", "/pub", ProtocolCapabilities::default()).is_err());
}
