use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// Manages identity-scoped persistent workspaces for agents.
///
/// Each workspace lives at `~/.local/share/agenticbox/workspaces/<identity>/`
/// and persists across sessions. Workspace paths are derived from the identity
/// name so they're deterministic and scoped — agent A can't see agent B's workspace.
pub struct WorkspaceManager;

impl WorkspaceManager {
    /// Root directory for all workspaces.
    fn root() -> PathBuf {
        dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("agenticbox")
            .join("workspaces")
    }

    /// Get the workspace path for a given identity name.
    ///
    /// Creates the directory if it doesn't exist.
    /// Returns the canonicalized path (to prevent symlink escapes).
    pub fn ensure(identity_name: &str) -> anyhow::Result<PathBuf> {
        let path = Self::root().join(sanitize_name(identity_name));
        fs::create_dir_all(&path)?;
        let canonical = path.canonicalize()?;
        Ok(canonical)
    }

    /// Get the workspace path for an identity without creating it.
    /// Returns `None` if the directory doesn't exist yet.
    pub fn path(identity_name: &str) -> Option<PathBuf> {
        let path = Self::root().join(sanitize_name(identity_name));
        if path.exists() {
            path.canonicalize().ok()
        } else {
            None
        }
    }

    /// List all workspaces with their sizes and last modified times.
    pub fn list() -> anyhow::Result<Vec<WorkspaceInfo>> {
        let root = Self::root();
        if !root.exists() {
            return Ok(vec![]);
        }

        let mut workspaces = Vec::new();
        let dir_entries = fs::read_dir(&root)?;
        for entry in dir_entries {
            let entry = entry?;
            if entry.file_type()?.is_dir() {
                let path = entry.path();
                let name = path
                    .file_name()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_default();
                let size = dir_size(&path)?;
                let modified = entry
                    .metadata()
                    .ok()
                    .and_then(|m| m.modified().ok())
                    .unwrap_or(SystemTime::UNIX_EPOCH);

                workspaces.push(WorkspaceInfo {
                    identity: name,
                    path,
                    size,
                    modified,
                });
            }
        }

        // Sort by modified time descending (newest first)
        workspaces.sort_by_key(|w| std::cmp::Reverse(w.modified));

        Ok(workspaces)
    }

    /// Remove the workspace for an identity.
    pub fn clean(identity_name: &str) -> anyhow::Result<()> {
        let path = Self::root().join(sanitize_name(identity_name));
        if path.exists() {
            fs::remove_dir_all(&path)?;
        }
        Ok(())
    }

    /// Check if a path is inside the workspace for a given identity.
    /// Returns `true` if the path canonicalizes to within the workspace directory.
    pub fn contains(identity_name: &str, path: &Path) -> anyhow::Result<bool> {
        let workspace = Self::ensure(identity_name)?;
        let canonical = path.canonicalize()?;
        Ok(canonical.starts_with(&workspace))
    }
}

/// Info about a single workspace.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WorkspaceInfo {
    pub identity: String,
    pub path: PathBuf,
    pub size: u64,
    pub modified: SystemTime,
}

/// Sanitize an identity name for use as a directory name.
/// Replaces non-alphanumeric characters (except hyphen and underscore) with underscores.
fn sanitize_name(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

/// Recursively calculate the total size of a directory in bytes.
fn dir_size(path: &Path) -> anyhow::Result<u64> {
    let mut total = 0u64;
    if path.is_dir() {
        let mut stack = vec![path.to_path_buf()];
        while let Some(dir) = stack.pop() {
            for entry in fs::read_dir(&dir)? {
                let entry = entry?;
                let meta = entry.metadata()?;
                if meta.is_dir() {
                    stack.push(entry.path());
                } else {
                    total += meta.len();
                }
            }
        }
    }
    Ok(total)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_name() {
        assert_eq!(sanitize_name("deploy-bot"), "deploy-bot");
        assert_eq!(sanitize_name("aria_support"), "aria_support");
        assert_eq!(sanitize_name("my agent!"), "my_agent_");
        assert_eq!(sanitize_name("hello@world"), "hello_world");
        assert_eq!(sanitize_name("normal123"), "normal123");
    }

    #[test]
    fn test_workspace_root_uses_data_dir() {
        let root = WorkspaceManager::root();
        assert!(root.ends_with("agenticbox/workspaces"));
    }

    #[test]
    fn test_ensure_creates_directory() {
        let tmp = tempfile::tempdir().unwrap();
        let data_home = tmp.path().join("data");
        std::env::set_var("XDG_DATA_HOME", &data_home);

        let identity = "test-identity";
        let path = WorkspaceManager::ensure(identity).unwrap();
        assert!(path.exists());
        assert!(path.is_dir());

        let _ = WorkspaceManager::clean(identity);
        std::env::remove_var("XDG_DATA_HOME");
    }

    #[test]
    fn test_workspace_is_scoped_to_identity() {
        let tmp = tempfile::tempdir().unwrap();
        let data_home = tmp.path().join("data");
        std::env::set_var("XDG_DATA_HOME", &data_home);

        let alice = WorkspaceManager::ensure("alice").unwrap();
        let bob = WorkspaceManager::ensure("bob").unwrap();

        assert_ne!(alice, bob);
        assert!(alice.to_string_lossy().contains("alice"));
        assert!(bob.to_string_lossy().contains("bob"));

        std::env::remove_var("XDG_DATA_HOME");
    }

    #[test]
    fn test_path_returns_none_if_not_exists() {
        let tmp = tempfile::tempdir().unwrap();
        let data_home = tmp.path().join("data");
        std::env::set_var("XDG_DATA_HOME", &data_home);

        let result = WorkspaceManager::path("nonexistent-identity");
        assert!(result.is_none());

        std::env::remove_var("XDG_DATA_HOME");
    }

    #[test]
    fn test_clean_removes_workspace() {
        let tmp = tempfile::tempdir().unwrap();
        let data_home = tmp.path().join("data");
        std::env::set_var("XDG_DATA_HOME", &data_home);

        let path = WorkspaceManager::ensure("clean-test").unwrap();
        assert!(path.exists());

        WorkspaceManager::clean("clean-test").unwrap();
        assert!(!path.exists());

        std::env::remove_var("XDG_DATA_HOME");
    }
}
