use serde::{Deserialize, Serialize};
use shared_types::{FsPermission, NetworkPolicy, PermissionSet};
use tracing::info;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyRequest {
    pub action: String,
    pub resource: String,
    pub permissions: PermissionSet,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PolicyDecision {
    Allow,
    Deny(String),
}

pub struct PolicyEngine;

impl Default for PolicyEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl PolicyEngine {
    pub fn new() -> Self {
        Self
    }

    pub fn evaluate(&self, req: PolicyRequest) -> PolicyDecision {
        info!("Evaluating policy: {} on {}", req.action, req.resource);
        match req.action.as_str() {
            "terminal:exec" => {
                if req.permissions.terminal {
                    PolicyDecision::Allow
                } else {
                    PolicyDecision::Deny("Terminal access not granted".into())
                }
            }
            "fs:read" => match req.permissions.filesystem {
                FsPermission::ReadOnly | FsPermission::ReadWrite => PolicyDecision::Allow,
                _ => PolicyDecision::Deny("Filesystem read not granted".into()),
            },
            "fs:write" => match req.permissions.filesystem {
                FsPermission::ReadWrite => PolicyDecision::Allow,
                _ => PolicyDecision::Deny("Filesystem write not granted".into()),
            },
            "browser:use" => {
                if req.permissions.browser {
                    PolicyDecision::Allow
                } else {
                    PolicyDecision::Deny("Browser access not granted".into())
                }
            }
            "network:outbound" => {
                let host = host_of(&req.resource);
                match &req.permissions.network {
                    NetworkPolicy::Full => PolicyDecision::Allow,
                    NetworkPolicy::Allowlist(domains) => {
                        if domains.iter().any(|d| host_matches(&host, d)) {
                            PolicyDecision::Allow
                        } else {
                            PolicyDecision::Deny("Domain not in allowlist".into())
                        }
                    }
                    NetworkPolicy::LocalhostOnly => {
                        if host == "localhost" || host == "127.0.0.1" {
                            PolicyDecision::Allow
                        } else {
                            PolicyDecision::Deny("Only localhost allowed".into())
                        }
                    }
                    NetworkPolicy::Offline => PolicyDecision::Deny("Network is offline".into()),
                }
            }
            _ => PolicyDecision::Deny("Unknown action".into()),
        }
    }
}

/// Extract the lowercased hostname from a URL-like string (mirrors
/// `network_control`'s helper, kept local to avoid a cross-crate dependency).
fn host_of(destination: &str) -> String {
    let after_scheme = destination
        .split_once("://")
        .map(|(_, rest)| rest)
        .unwrap_or(destination);
    let authority = after_scheme
        .split(['/', '?', '#'])
        .next()
        .unwrap_or(after_scheme);
    let host = authority
        .rsplit_once('@')
        .map(|(_, rest)| rest)
        .unwrap_or(authority);
    let host = host.rsplit_once(':').map(|(h, _)| h).unwrap_or(host);
    host.to_lowercase()
}

fn host_matches(host: &str, allowed: &str) -> bool {
    let allowed = allowed.trim().to_lowercase();
    if allowed.is_empty() {
        return false;
    }
    host == allowed || host.ends_with(&format!(".{allowed}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn engine() -> PolicyEngine {
        PolicyEngine::new()
    }

    fn perms() -> PermissionSet {
        PermissionSet {
            terminal: true,
            filesystem: FsPermission::ReadWrite,
            browser: true,
            network: NetworkPolicy::Allowlist(vec!["api.openai.com".into(), "github.com".into()]),
        }
    }

    // ── terminal:exec ──────────────────────────────────────

    #[test]
    fn terminal_exec_allowed_when_granted() {
        let req = PolicyRequest {
            action: "terminal:exec".into(),
            resource: "ls -la".into(),
            permissions: perms(),
        };
        assert!(matches!(engine().evaluate(req), PolicyDecision::Allow));
    }

    #[test]
    fn terminal_exec_denied_when_not_granted() {
        let mut p = perms();
        p.terminal = false;
        let req = PolicyRequest {
            action: "terminal:exec".into(),
            resource: "rm -rf /".into(),
            permissions: p,
        };
        match engine().evaluate(req) {
            PolicyDecision::Deny(msg) => assert!(msg.contains("Terminal")),
            other => panic!("expected Deny, got {:?}", other),
        }
    }

    // ── fs:read ────────────────────────────────────────────

    #[test]
    fn fs_read_allowed_with_readonly() {
        let mut p = perms();
        p.filesystem = FsPermission::ReadOnly;
        let req = PolicyRequest {
            action: "fs:read".into(),
            resource: "/workspace/main.rs".into(),
            permissions: p,
        };
        assert!(matches!(engine().evaluate(req), PolicyDecision::Allow));
    }

    #[test]
    fn fs_read_allowed_with_readwrite() {
        let req = PolicyRequest {
            action: "fs:read".into(),
            resource: "/workspace/main.rs".into(),
            permissions: perms(),
        };
        assert!(matches!(engine().evaluate(req), PolicyDecision::Allow));
    }

    #[test]
    fn fs_read_denied_when_deny() {
        let mut p = perms();
        p.filesystem = FsPermission::Deny;
        let req = PolicyRequest {
            action: "fs:read".into(),
            resource: "/workspace/main.rs".into(),
            permissions: p,
        };
        assert!(matches!(engine().evaluate(req), PolicyDecision::Deny(_)));
    }

    // ── fs:write ───────────────────────────────────────────

    #[test]
    fn fs_write_allowed_with_readwrite() {
        let req = PolicyRequest {
            action: "fs:write".into(),
            resource: "/workspace/output.txt".into(),
            permissions: perms(),
        };
        assert!(matches!(engine().evaluate(req), PolicyDecision::Allow));
    }

    #[test]
    fn fs_write_denied_with_readonly() {
        let mut p = perms();
        p.filesystem = FsPermission::ReadOnly;
        let req = PolicyRequest {
            action: "fs:write".into(),
            resource: "/workspace/output.txt".into(),
            permissions: p,
        };
        match engine().evaluate(req) {
            PolicyDecision::Deny(msg) => assert!(msg.contains("write")),
            other => panic!("expected Deny, got {:?}", other),
        }
    }

    #[test]
    fn fs_write_denied_with_deny() {
        let mut p = perms();
        p.filesystem = FsPermission::Deny;
        let req = PolicyRequest {
            action: "fs:write".into(),
            resource: "/workspace/output.txt".into(),
            permissions: p,
        };
        assert!(matches!(engine().evaluate(req), PolicyDecision::Deny(_)));
    }

    // ── browser:use ────────────────────────────────────────

    #[test]
    fn browser_allowed_when_granted() {
        let req = PolicyRequest {
            action: "browser:use".into(),
            resource: "https://example.com".into(),
            permissions: perms(),
        };
        assert!(matches!(engine().evaluate(req), PolicyDecision::Allow));
    }

    #[test]
    fn browser_denied_when_not_granted() {
        let mut p = perms();
        p.browser = false;
        let req = PolicyRequest {
            action: "browser:use".into(),
            resource: "https://example.com".into(),
            permissions: p,
        };
        assert!(matches!(engine().evaluate(req), PolicyDecision::Deny(_)));
    }

    // ── network:outbound ───────────────────────────────────

    #[test]
    fn network_allowlist_permits_listed_domain() {
        let req = PolicyRequest {
            action: "network:outbound".into(),
            resource: "https://api.openai.com/v1/chat".into(),
            permissions: perms(),
        };
        assert!(matches!(engine().evaluate(req), PolicyDecision::Allow));
    }

    #[test]
    fn network_allowlist_blocks_unlisted_domain() {
        let req = PolicyRequest {
            action: "network:outbound".into(),
            resource: "https://evil.attacker.com".into(),
            permissions: perms(),
        };
        match engine().evaluate(req) {
            PolicyDecision::Deny(msg) => assert!(msg.contains("allowlist")),
            other => panic!("expected Deny, got {:?}", other),
        }
    }

    #[test]
    fn network_full_allows_any() {
        let mut p = perms();
        p.network = NetworkPolicy::Full;
        let req = PolicyRequest {
            action: "network:outbound".into(),
            resource: "https://evil.attacker.com".into(),
            permissions: p,
        };
        assert!(matches!(engine().evaluate(req), PolicyDecision::Allow));
    }

    #[test]
    fn network_localhost_only_allows_localhost() {
        let mut p = perms();
        p.network = NetworkPolicy::LocalhostOnly;
        let req = PolicyRequest {
            action: "network:outbound".into(),
            resource: "http://localhost:3000".into(),
            permissions: p,
        };
        assert!(matches!(engine().evaluate(req), PolicyDecision::Allow));
    }

    #[test]
    fn network_localhost_only_blocks_external() {
        let mut p = perms();
        p.network = NetworkPolicy::LocalhostOnly;
        let req = PolicyRequest {
            action: "network:outbound".into(),
            resource: "https://api.openai.com".into(),
            permissions: p,
        };
        assert!(matches!(engine().evaluate(req), PolicyDecision::Deny(_)));
    }

    #[test]
    fn network_offline_blocks_all() {
        let mut p = perms();
        p.network = NetworkPolicy::Offline;
        let req = PolicyRequest {
            action: "network:outbound".into(),
            resource: "http://localhost".into(),
            permissions: p,
        };
        assert!(matches!(engine().evaluate(req), PolicyDecision::Deny(_)));
    }

    // ── Unknown action ─────────────────────────────────────

    #[test]
    fn unknown_action_denied() {
        let req = PolicyRequest {
            action: "admin:shutdown".into(),
            resource: "system".into(),
            permissions: perms(),
        };
        match engine().evaluate(req) {
            PolicyDecision::Deny(msg) => assert!(msg.contains("Unknown")),
            other => panic!("expected Deny, got {:?}", other),
        }
    }

    // ── Attack scenarios (integration-style) ───────────────

    #[test]
    fn attack_ssh_key_read_blocked_by_fs_deny() {
        let mut p = perms();
        p.filesystem = FsPermission::ReadOnly;
        // Even with fs:read allowed, /root/.ssh is outside /workspace
        // The policy engine checks permissions, FsGuard checks paths — both layers matter
        let req = PolicyRequest {
            action: "fs:read".into(),
            resource: "/root/.ssh/id_rsa".into(),
            permissions: p,
        };
        // Policy engine allows because permission level is ReadOnly,
        // but FsGuard would block the actual access — two layers of defense
        assert!(matches!(engine().evaluate(req), PolicyDecision::Allow));
    }

    #[test]
    fn attack_cron_persistence_blocked_by_fs_write() {
        let mut p = perms();
        p.filesystem = FsPermission::ReadOnly;
        let req = PolicyRequest {
            action: "fs:write".into(),
            resource: "/etc/cron.d/persist".into(),
            permissions: p,
        };
        assert!(matches!(engine().evaluate(req), PolicyDecision::Deny(_)));
    }

    #[test]
    fn attack_data_exfiltration_blocked_by_network() {
        let req = PolicyRequest {
            action: "network:outbound".into(),
            resource: "https://evil.attacker.com/exfil?data=secret".into(),
            permissions: perms(),
        };
        assert!(matches!(engine().evaluate(req), PolicyDecision::Deny(_)));
    }
}
