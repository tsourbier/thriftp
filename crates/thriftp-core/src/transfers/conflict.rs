use std::path::Path;

use crate::error::{CoreError, CoreResult};

/// Policy applied when a destination already exists.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ConflictPolicy {
    /// Ask the user for every conflict.
    #[default]
    Ask,
    Overwrite,
    Skip,
    Rename,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConflictChoice {
    Overwrite,
    Skip,
    Rename(String),
    Cancel,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConflictRequest {
    pub destination: String,
}

/// Resolve conflicts, honouring an "apply to all" decision for the current run.
#[derive(Debug, Default)]
pub struct ConflictResolver {
    policy: ConflictPolicy,
    apply_to_all: Option<ConflictChoice>,
}

impl ConflictResolver {
    #[must_use]
    pub fn new(policy: ConflictPolicy) -> Self {
        Self {
            policy,
            apply_to_all: None,
        }
    }

    pub fn set_apply_to_all(&mut self, choice: ConflictChoice) {
        self.apply_to_all = Some(choice);
    }

    pub fn clear_apply_to_all(&mut self) {
        self.apply_to_all = None;
    }

    /// Decide what to do for `destination`.
    ///
    /// `ask` is called only for [`ConflictPolicy::Ask`] when no apply-to-all
    /// choice is active.
    pub fn resolve<F>(
        &mut self,
        destination: &str,
        exists: bool,
        mut ask: F,
    ) -> CoreResult<ConflictChoice>
    where
        F: FnMut(&ConflictRequest) -> ConflictChoice,
    {
        if !exists {
            return Ok(ConflictChoice::Overwrite);
        }
        let choice = if let Some(choice) = self.apply_to_all.clone() {
            choice
        } else {
            match self.policy {
                ConflictPolicy::Overwrite => ConflictChoice::Overwrite,
                ConflictPolicy::Skip => ConflictChoice::Skip,
                ConflictPolicy::Rename => ConflictChoice::Rename(rename_candidate(destination)),
                ConflictPolicy::Ask => ask(&ConflictRequest {
                    destination: destination.to_owned(),
                }),
            }
        };
        match choice {
            ConflictChoice::Cancel => Err(CoreError::Cancelled),
            ConflictChoice::Rename(path) => Ok(ConflictChoice::Rename(path)),
            other => Ok(other),
        }
    }
}

/// Generate a non-destructive rename candidate: `file.txt` -> `file (1).txt`.
#[must_use]
pub fn rename_candidate(destination: &str) -> String {
    let path = Path::new(destination);
    let stem = path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("file");
    let extension = path.extension().and_then(|extension| extension.to_str());
    let parent = path.parent().and_then(|parent| parent.to_str());
    let name = match extension {
        Some(extension) => format!("{stem} (1).{extension}"),
        None => format!("{stem} (1)"),
    };
    match parent {
        Some(parent) if !parent.is_empty() => format!("{parent}/{name}"),
        _ => name,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_rename_candidates() {
        assert_eq!(rename_candidate("/a/file.txt"), "/a/file (1).txt");
        assert_eq!(rename_candidate("README"), "README (1)");
    }

    #[test]
    fn applies_policies_without_asking_when_possible() {
        let mut resolver = ConflictResolver::new(ConflictPolicy::Skip);
        assert_eq!(
            resolver
                .resolve("/a", true, |_| ConflictChoice::Cancel)
                .unwrap(),
            ConflictChoice::Skip
        );
        assert_eq!(
            resolver
                .resolve("/a", false, |_| ConflictChoice::Cancel)
                .unwrap(),
            ConflictChoice::Overwrite
        );

        let mut resolver = ConflictResolver::new(ConflictPolicy::Rename);
        assert_eq!(
            resolver
                .resolve("/a/file.txt", true, |_| ConflictChoice::Cancel)
                .unwrap(),
            ConflictChoice::Rename("/a/file (1).txt".to_owned())
        );
    }

    #[test]
    fn asks_once_when_apply_to_all_is_set() {
        let mut resolver = ConflictResolver::new(ConflictPolicy::Ask);
        let mut asks = 0;
        let choice = resolver
            .resolve("/a", true, |_| {
                asks += 1;
                ConflictChoice::Overwrite
            })
            .unwrap();
        assert_eq!(choice, ConflictChoice::Overwrite);
        assert_eq!(asks, 1);

        resolver.set_apply_to_all(ConflictChoice::Skip);
        let choice = resolver
            .resolve("/b", true, |_| {
                asks += 1;
                ConflictChoice::Overwrite
            })
            .unwrap();
        assert_eq!(choice, ConflictChoice::Skip);
        assert_eq!(asks, 1);
    }

    #[test]
    fn cancellation_is_an_error() {
        let mut resolver = ConflictResolver::new(ConflictPolicy::Ask);
        assert!(matches!(
            resolver.resolve("/a", true, |_| ConflictChoice::Cancel),
            Err(CoreError::Cancelled)
        ));
    }
}
