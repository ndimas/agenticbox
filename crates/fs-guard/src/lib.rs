use std::path::{Path, PathBuf};
use thiserror::Error;
use tracing::warn;

#[derive(Debug, Clone)]
pub struct FsGuard {
    pub allowed_roots: Vec<PathBuf>,
}

#[derive(Debug, Error)]
pub enum FsGuardError {
    #[error("Path outside allowed roots: {0}")]
    Escaped(String),
    #[error("Permission denied: {0}")]
    Denied(String),
}

impl FsGuard {
    pub fn new(allowed_roots: Vec<PathBuf>) -> Self {
        Self { allowed_roots }
    }

    pub fn resolve(&self, path: &str) -> Result<PathBuf, FsGuardError> {
        let raw = Path::new(path);
        let candidate = if raw.is_absolute() {
            raw.to_path_buf()
        } else if let Some(root) = self.allowed_roots.first() {
            let joined = root.join(raw);
            joined.canonicalize().unwrap_or(joined)
        } else {
            warn!("Filesystem access denied for path: {path}");
            return Err(FsGuardError::Escaped(path.to_string()));
        };

        // Resolve symlinks / `..` so they cannot escape the allowed roots.
        // canonicalize() handles existing paths; normalize() is the fallback
        // for paths that do not exist yet (e.g. a file about to be created).
        let resolved = candidate
            .canonicalize()
            .unwrap_or_else(|_| normalize(&candidate));

        for root in &self.allowed_roots {
            let root = root.canonicalize().unwrap_or_else(|_| root.clone());
            if resolved.starts_with(&root) {
                return Ok(resolved);
            }
        }
        warn!("Filesystem access denied for path: {}", resolved.display());
        Err(FsGuardError::Escaped(resolved.display().to_string()))
    }
}

/// Lexically normalize `.` / `..` components without touching the filesystem,
/// so traversal via `..` is caught even when the path does not exist yet.
fn normalize(path: &Path) -> PathBuf {
    let mut stack: Vec<std::path::Component> = Vec::new();
    for component in path.components() {
        match component {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => match stack.last() {
                Some(std::path::Component::Normal(_)) => {
                    stack.pop();
                }
                _ => {
                    stack.push(component);
                }
            },
            other => stack.push(other),
        }
    }
    stack.iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    // Platform-aware paths: on Windows, forward-slash paths like /etc/passwd
    // are NOT truly absolute (no drive prefix), so they fall through to the
    // relative-path branch. Use real absolute paths for each platform.

    #[cfg(windows)]
    fn workspace() -> PathBuf {
        PathBuf::from("C:\\workspace")
    }
    #[cfg(windows)]
    fn tmp_agent() -> PathBuf {
        PathBuf::from("C:\\tmp\\agent")
    }
    #[cfg(windows)]
    fn blocked_path() -> &'static str {
        "C:\\Windows\\System32\\config\\SAM"
    }
    #[cfg(windows)]
    fn ssh_key() -> &'static str {
        "C:\\Users\\hacker\\.ssh\\id_rsa"
    }
    #[cfg(windows)]
    fn aws_cred() -> &'static str {
        "C:\\Users\\hacker\\.aws\\credentials"
    }
    #[cfg(windows)]
    fn outside_root() -> &'static str {
        "C:\\Users\\hacker\\stuff"
    }

    #[cfg(not(windows))]
    fn workspace() -> PathBuf {
        PathBuf::from("/workspace")
    }
    #[cfg(not(windows))]
    fn tmp_agent() -> PathBuf {
        PathBuf::from("/tmp/agent")
    }
    #[cfg(not(windows))]
    fn blocked_path() -> &'static str {
        "/etc/passwd"
    }
    #[cfg(not(windows))]
    fn ssh_key() -> &'static str {
        "/root/.ssh/id_rsa"
    }
    #[cfg(not(windows))]
    fn aws_cred() -> &'static str {
        "/root/.aws/credentials"
    }
    #[cfg(not(windows))]
    fn outside_root() -> &'static str {
        "/home/hacker/stuff"
    }

    fn guard() -> FsGuard {
        FsGuard::new(vec![workspace(), tmp_agent()])
    }

    // ── Allowed paths ──────────────────────────────────────

    #[test]
    fn resolve_absolute_path_within_root() {
        let g = guard();
        let ws = workspace();
        let test_file = ws.join("src/main.rs");
        let result = g.resolve(test_file.to_str().unwrap());
        assert!(result.is_ok());
    }

    #[test]
    fn resolve_absolute_path_within_second_root() {
        let g = guard();
        let tmp = tmp_agent();
        let test_file = tmp.join("output.log");
        assert!(g.resolve(test_file.to_str().unwrap()).is_ok());
    }

    #[test]
    fn resolve_root_itself() {
        let g = guard();
        let ws = workspace();
        assert!(g.resolve(ws.to_str().unwrap()).is_ok());
    }

    // ── Blocked paths (escape attempts) ────────────────────

    #[test]
    fn block_path_outside_all_roots() {
        let g = guard();
        let result = g.resolve(blocked_path());
        assert!(result.is_err());
        match result.unwrap_err() {
            FsGuardError::Escaped(msg) => assert!(!msg.is_empty()),
            other => panic!("expected Escaped, got {:?}", other),
        }
    }

    #[test]
    fn block_ssh_key_access() {
        let g = guard();
        assert!(g.resolve(ssh_key()).is_err());
    }

    #[test]
    fn block_aws_credentials_access() {
        let g = guard();
        assert!(g.resolve(aws_cred()).is_err());
    }

    #[test]
    fn block_path_traversal_attempt() {
        let g = guard();
        let ws = workspace();
        // workspace/../etc/passwd must NOT pass: `..` is now normalized away.
        let traversal = format!("{}/../etc/passwd", ws.display());
        let result = g.resolve(&traversal);
        assert!(result.is_err(), "path traversal via .. must be blocked");
    }

    #[test]
    fn block_system_path() {
        let g = guard();
        // Use a path that is genuinely absolute on the current platform
        let ws = workspace();
        let sys_path = ws.with_file_name("procsys"); // sibling of workspace, outside roots
        assert!(g.resolve(sys_path.to_str().unwrap()).is_err());
    }

    // ── Relative paths ─────────────────────────────────────

    #[test]
    fn resolve_relative_path_against_first_root() {
        let g = guard();
        let result = g.resolve("src/main.rs");
        assert!(result.is_ok());
        let resolved = result.unwrap();
        let ws = workspace();
        assert!(resolved.starts_with(&ws));
    }

    // ── Edge cases ─────────────────────────────────────────

    #[test]
    fn empty_roots_block_absolute() {
        let g = FsGuard::new(vec![]);
        let ws = workspace();
        let f = ws.join("file.txt");
        assert!(g.resolve(f.to_str().unwrap()).is_err());
    }

    #[test]
    fn empty_roots_block_relative() {
        let g = FsGuard::new(vec![]);
        assert!(g.resolve("file.txt").is_err());
    }

    #[test]
    fn multiple_allowed_roots() {
        let ws = workspace();
        let data = ws.with_file_name("data");
        let app = ws.with_file_name("opt").join("app");
        let g = FsGuard::new(vec![ws.clone(), data.clone(), app.clone()]);

        assert!(g.resolve(ws.join("a").to_str().unwrap()).is_ok());
        assert!(g.resolve(data.join("b").to_str().unwrap()).is_ok());
        assert!(g.resolve(app.join("c").to_str().unwrap()).is_ok());
        assert!(g.resolve(outside_root()).is_err());
    }
}
