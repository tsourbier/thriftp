use std::path::Path;

use crate::error::{CoreError, CoreResult};

/// Largest file rendered inside the application.
pub const MAX_INLINE_BYTES: u64 = 5 * 1024 * 1024;

/// Largest remote file the user may consent to download for preview.
pub const MAX_REMOTE_BYTES: u64 = 25 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PreviewMode {
    /// Open with the operating system's default application.
    Native,
    /// Render inside the application.
    Internal,
    /// Show metadata only; do not read file contents.
    Metadata,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PreviewContent {
    Text,
    Image,
    Binary,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreviewPlan {
    pub mode: PreviewMode,
    pub content: Option<PreviewContent>,
    pub reason: Option<String>,
}

impl PreviewPlan {
    fn metadata(reason: impl Into<String>) -> Self {
        Self {
            mode: PreviewMode::Metadata,
            content: None,
            reason: Some(reason.into()),
        }
    }
}

#[must_use]
pub fn classify(name: &str) -> PreviewContent {
    let extension = Path::new(name)
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    match extension.as_str() {
        "png" | "jpg" | "jpeg" | "gif" | "webp" | "bmp" | "svg" | "ico" => PreviewContent::Image,
        "txt" | "md" | "json" | "toml" | "yaml" | "yml" | "rs" | "ts" | "js" | "css" | "html"
        | "xml" | "csv" | "log" | "sh" | "py" | "c" | "h" | "cpp" | "go" | "java" | "sql" => {
            PreviewContent::Text
        }
        _ => PreviewContent::Binary,
    }
}

/// Plan a preview for a local file.
pub fn plan_local(path: &Path) -> CoreResult<PreviewPlan> {
    let metadata = std::fs::symlink_metadata(path)
        .map_err(|error| CoreError::NotFound(format!("{}: {error}", path.display())))?;
    if metadata.file_type().is_dir() {
        return Err(CoreError::UnsupportedCapability(
            "directories cannot be previewed".to_owned(),
        ));
    }
    if metadata.len() > MAX_INLINE_BYTES {
        return Ok(PreviewPlan::metadata(
            "the file is too large to preview inside the application",
        ));
    }
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("");
    let content = classify(name);
    let mode = match content {
        PreviewContent::Binary => PreviewMode::Native,
        _ => PreviewMode::Internal,
    };
    Ok(PreviewPlan {
        mode,
        content: (mode == PreviewMode::Internal).then_some(content),
        reason: None,
    })
}

/// Plan a preview for a remote file.
///
/// Downloading a remote file for preview requires explicit consent and is
/// bounded by [`MAX_REMOTE_BYTES`]. Without consent only metadata is shown.
pub fn plan_remote(size: Option<u64>, name: &str, allow_download: bool) -> PreviewPlan {
    let Some(size) = size else {
        return PreviewPlan::metadata("the remote file size is unknown");
    };
    if size > MAX_REMOTE_BYTES {
        return PreviewPlan::metadata("the remote file exceeds the preview limit");
    }
    if !allow_download {
        return PreviewPlan::metadata("downloading the remote file requires consent");
    }
    let content = classify(name);
    if content == PreviewContent::Binary {
        return PreviewPlan::metadata("binary files are not previewed inside the application");
    }
    PreviewPlan {
        mode: PreviewMode::Internal,
        content: Some(content),
        reason: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_common_types() {
        assert_eq!(classify("photo.PNG"), PreviewContent::Image);
        assert_eq!(classify("notes.md"), PreviewContent::Text);
        assert_eq!(classify("archive.zip"), PreviewContent::Binary);
        assert_eq!(classify("README"), PreviewContent::Binary);
    }

    #[test]
    fn small_local_text_is_previewed_internally() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("notes.txt");
        std::fs::write(&path, b"hello").unwrap();
        let plan = plan_local(&path).unwrap();
        assert_eq!(plan.mode, PreviewMode::Internal);
        assert_eq!(plan.content, Some(PreviewContent::Text));
    }

    #[test]
    fn large_local_file_falls_back_to_metadata() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("large.txt");
        let file = std::fs::File::create(&path).unwrap();
        file.set_len(MAX_INLINE_BYTES + 1).unwrap();
        let plan = plan_local(&path).unwrap();
        assert_eq!(plan.mode, PreviewMode::Metadata);
    }

    #[test]
    fn remote_preview_requires_consent_and_a_bound() {
        assert_eq!(
            plan_remote(Some(10), "a.txt", false).mode,
            PreviewMode::Metadata
        );
        assert_eq!(
            plan_remote(Some(MAX_REMOTE_BYTES + 1), "a.txt", true).mode,
            PreviewMode::Metadata
        );
        assert_eq!(
            plan_remote(Some(10), "a.txt", true).mode,
            PreviewMode::Internal
        );
        assert_eq!(plan_remote(None, "a.txt", true).mode, PreviewMode::Metadata);
    }

    #[test]
    fn directories_are_rejected() {
        let directory = tempfile::tempdir().unwrap();
        assert!(plan_local(directory.path()).is_err());
    }
}
