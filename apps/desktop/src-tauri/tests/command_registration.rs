use std::fs;
use std::path::Path;

/// Every `#[tauri::command]` must appear in `generate_handler!`, otherwise the
/// frontend fails at runtime with "Command <name> not found".
#[test]
fn every_tauri_command_is_registered() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let lib = fs::read_to_string(root.join("src/lib.rs")).unwrap();
    let commands = fs::read_dir(root.join("src/commands")).unwrap();

    for entry in commands {
        let path = entry.unwrap().path();
        if path.extension().and_then(|extension| extension.to_str()) != Some("rs")
            || path.file_name().and_then(|name| name.to_str()) == Some("mod.rs")
        {
            continue;
        }
        let module = path.file_stem().unwrap().to_str().unwrap();
        let source = fs::read_to_string(&path).unwrap();
        let lines: Vec<_> = source.lines().collect();

        for (index, line) in lines.iter().enumerate() {
            if line.trim() != "#[tauri::command]" {
                continue;
            }
            let name = lines[index + 1..]
                .iter()
                .find_map(|candidate| command_name(candidate.trim()))
                .unwrap_or_else(|| panic!("command in {module} has no function signature"));
            let registration = format!("commands::{module}::{name}");
            assert!(
                lib.contains(&registration),
                "{registration} is missing from generate_handler! in src/lib.rs"
            );
        }
    }
}

fn command_name(line: &str) -> Option<String> {
    for prefix in ["pub async fn ", "pub fn ", "async fn ", "fn "] {
        if let Some(rest) = line.strip_prefix(prefix) {
            let name: String = rest
                .chars()
                .take_while(|character| character.is_ascii_alphanumeric() || *character == '_')
                .collect();
            if !name.is_empty() {
                return Some(name);
            }
        }
    }
    None
}
