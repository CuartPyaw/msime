//! The four runtime roots a session reads and writes. Paths are captured at construction, so changing the process environment cannot redirect a live session. The working dictionaries under `dictionaries` are durable user data, never a disposable cache.
//!
//! `prepare_runtime_paths`, which stages a generation directory, lives in `user_dictionary::generation` because it replays the journal.

use std::path::{Component, Path, PathBuf};

use crate::diagnostics;
use crate::error::{EngineError, Result};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RuntimePaths {
    /// The immutable packaged bundle.
    pub resources: PathBuf,
    /// The journal, learned glosses and hand-written translations.
    pub user_data: PathBuf,
    pub cache: PathBuf,
    /// The generation directory holding the mutable working copies (`user_data/dictionaries/<content id>`).
    pub dictionaries: PathBuf,
}

impl RuntimePaths {
    /// A file in the resource bundle. Names are the constants in `assets`; a name built from user input goes through `join_checked` first.
    pub fn resource(&self, name: &str) -> PathBuf {
        join(&self.resources, name)
    }

    pub fn user(&self, name: &str) -> PathBuf {
        join(&self.user_data, name)
    }

    pub fn dictionary(&self, name: &str) -> PathBuf {
        join(&self.dictionaries, name)
    }

    /// Every root must be absolute; a relative root would resolve against whatever the host's working directory happens to be.
    pub fn validate(&self) -> Result<()> {
        for root in [
            &self.resources,
            &self.user_data,
            &self.cache,
            &self.dictionaries,
        ] {
            if !root.is_absolute() {
                return Err(EngineError::invalid(
                    diagnostics::RUNTIME_DIRECTORIES_MUST_BE_ABSOLUTE,
                ));
            }
        }
        Ok(())
    }
}

/// An empty root, an empty name or an absolute name yields an empty path, which every reader treats as a missing file, as the C++ did.
fn join(root: &Path, name: &str) -> PathBuf {
    if root.as_os_str().is_empty() || name.is_empty() || Path::new(name).is_absolute() {
        return PathBuf::new();
    }
    root.join(name)
}

/// `join` for a name that came from outside the crate: a `..` component is refused rather than allowed to escape the root.
pub fn join_checked(root: &Path, name: &str) -> Result<PathBuf> {
    if Path::new(name)
        .components()
        .any(|part| matches!(part, Component::ParentDir))
    {
        return Err(EngineError::invalid(
            diagnostics::RUNTIME_ASSET_PATH_ESCAPES,
        ));
    }
    Ok(join(root, name))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn joins_like_the_reference() {
        let paths = RuntimePaths {
            resources: PathBuf::from("/r"),
            user_data: PathBuf::from("/u"),
            cache: PathBuf::from("/c"),
            dictionaries: PathBuf::new(),
        };
        assert_eq!(paths.resource("msime.db"), PathBuf::from("/r/msime.db"));
        assert_eq!(paths.dictionary("msime.db"), PathBuf::new());
        assert_eq!(paths.user("/abs"), PathBuf::new());
        assert!(join_checked(Path::new("/r"), "helpcodes/../x").is_err());
        assert!(paths.validate().is_err());
    }
}
