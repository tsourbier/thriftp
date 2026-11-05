//! Path validation and accessible-ancestor fallback rules.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// How a requested path was resolved.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PathFallback {
    /// The requested path was usable.
    None,
    /// The nearest accessible ancestor was used.
    Ancestor,
    /// The local home directory was used.
    Home,
    /// The remote login directory was used.
    Login,
}

/// A redacted notice explaining a path fallback.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PathNotice {
    /// Stable notice code.
    pub code: String,
    /// User-facing message.
    pub message: String,
}

/// Result of resolving a saved path.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PathResolution {
    /// Requested path.
    pub original_path: String,
    /// Path that should be opened.
    pub path: String,
    /// Fallback applied.
    pub fallback: PathFallback,
    /// At most one redacted notice.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notice: Option<PathNotice>,
}

impl PathResolution {
    /// Build a resolution with an optional notice.
    #[must_use]
    pub fn new(
        original_path: impl Into<String>,
        path: impl Into<String>,
        fallback: PathFallback,
        message: Option<String>,
    ) -> Self {
        let original_path = original_path.into();
        let path = path.into();
        let notice = message.map(|message| PathNotice {
            code: match fallback {
                PathFallback::None => "path_restored".to_owned(),
                PathFallback::Ancestor => "path_ancestor_fallback".to_owned(),
                PathFallback::Home => "path_home_fallback".to_owned(),
                PathFallback::Login => "path_login_fallback".to_owned(),
            },
            message,
        });
        Self {
            original_path,
            path,
            fallback,
            notice,
        }
    }
}

/// Return candidate ancestors from nearest to root for a POSIX-style path.
#[must_use]
pub fn posix_ancestor_candidates(path: &str) -> Vec<String> {
    let trimmed = path.trim_end_matches('/');
    if trimmed.is_empty() {
        return Vec::new();
    }
    let mut candidates = Vec::new();
    let mut current = trimmed;
    while let Some(index) = current.rfind('/') {
        if index == 0 {
            candidates.push("/".to_owned());
            break;
        }
        current = &current[..index];
        candidates.push(current.to_owned());
    }
    candidates
}

/// Return candidate ancestors from nearest to root for a local path.
#[must_use]
pub fn local_ancestor_candidates(path: &Path) -> Vec<PathBuf> {
    path.ancestors().skip(1).map(Path::to_path_buf).collect()
}

/// Resolve a local path to itself, its nearest readable ancestor, or home.
#[must_use]
pub fn resolve_local(path: &str) -> PathResolution {
    let requested = Path::new(path);
    if is_readable_directory(requested) {
        return PathResolution::new(path, path, PathFallback::None, None);
    }
    for ancestor in local_ancestor_candidates(requested) {
        if is_readable_directory(&ancestor) {
            return PathResolution::new(
                path,
                ancestor.display().to_string(),
                PathFallback::Ancestor,
                Some(
                    "The saved local folder could not be opened. Using the nearest available folder instead."
                        .to_owned(),
                ),
            );
        }
    }
    if let Some(home) = local_home()
        && is_readable_directory(&home)
    {
        return PathResolution::new(
            path,
            home.display().to_string(),
            PathFallback::Home,
            Some(
                "The saved local folder could not be opened. Using your home folder instead."
                    .to_owned(),
            ),
        );
    }
    PathResolution::new(
        path,
        path,
        PathFallback::None,
        Some("The saved local folder could not be opened.".to_owned()),
    )
}

/// Resolve a remote path using a predicate that checks accessibility.
#[must_use]
pub fn resolve_remote_with<F>(path: &str, mut accessible: F) -> PathResolution
where
    F: FnMut(&str) -> bool,
{
    if accessible(path) {
        return PathResolution::new(path, path, PathFallback::None, None);
    }
    for candidate in posix_ancestor_candidates(path) {
        if accessible(&candidate) {
            return PathResolution::new(
                path,
                candidate,
                PathFallback::Ancestor,
                Some(
                    "The saved remote folder could not be opened. Using the nearest available folder instead."
                        .to_owned(),
                ),
            );
        }
    }
    PathResolution::new(
        path,
        ".",
        PathFallback::Login,
        Some(
            "The saved remote folder could not be opened. Using the server login folder instead."
                .to_owned(),
        ),
    )
}

fn is_readable_directory(path: &Path) -> bool {
    path.is_dir() && std::fs::read_dir(path).is_ok()
}

fn local_home() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
}

#[cfg(test)]
mod tests {
    use super::{PathFallback, posix_ancestor_candidates, resolve_local, resolve_remote_with};

    #[test]
    fn posix_candidates_walk_to_root() {
        assert_eq!(
            posix_ancestor_candidates("/srv/www/app"),
            vec!["/srv/www", "/srv", "/"]
        );
        assert_eq!(posix_ancestor_candidates("relative/path"), vec!["relative"]);
    }

    #[test]
    fn remote_resolver_uses_ancestor_then_login() {
        let resolved = resolve_remote_with("/srv/www/missing", |path| path == "/srv");
        assert_eq!(resolved.path, "/srv");
        assert_eq!(resolved.fallback, PathFallback::Ancestor);
        assert!(resolved.notice.is_some());

        let resolved = resolve_remote_with("/missing", |_| false);
        assert_eq!(resolved.path, ".");
        assert_eq!(resolved.fallback, PathFallback::Login);
    }

    #[test]
    fn local_resolver_keeps_existing_directory() {
        let temp = std::env::temp_dir();
        let resolved = resolve_local(&temp.display().to_string());
        assert_eq!(resolved.fallback, PathFallback::None);
        assert_eq!(resolved.path, temp.display().to_string());
    }

    #[test]
    fn local_resolver_falls_back_for_missing_path() {
        let missing = std::env::temp_dir().join("thriftp-does-not-exist-004");
        let resolved = resolve_local(&missing.display().to_string());
        assert_ne!(resolved.fallback, PathFallback::None);
        assert!(resolved.notice.is_some());
    }
}
