use serde::{Deserialize, Serialize};
use shared_types::NetworkPolicy;
use tracing::info;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkGuard {
    pub policy: NetworkPolicy,
}

impl NetworkGuard {
    pub fn new(policy: NetworkPolicy) -> Self {
        Self { policy }
    }

    pub fn check(&self, destination: &str) -> Result<(), NetworkError> {
        info!("Checking network policy for {}", destination);
        let host = host_of(destination);
        match &self.policy {
            NetworkPolicy::Full => Ok(()),
            NetworkPolicy::LocalhostOnly => {
                if host == "localhost" || host == "127.0.0.1" {
                    Ok(())
                } else {
                    Err(NetworkError::Blocked(format!(
                        "{} not localhost",
                        destination
                    )))
                }
            }
            NetworkPolicy::Allowlist(domains) => {
                if domains.iter().any(|d| host_matches(&host, d)) {
                    Ok(())
                } else {
                    Err(NetworkError::Blocked(format!(
                        "{} not in allowlist",
                        destination
                    )))
                }
            }
            NetworkPolicy::Offline => Err(NetworkError::Blocked("Offline mode".into())),
        }
    }
}

/// Extract the lowercased hostname from a URL-like string, stripping scheme,
/// path, query, fragment, userinfo, and port.
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
    // Drop the port (hostname / IPv4 case).
    let host = host.rsplit_once(':').map(|(h, _)| h).unwrap_or(host);
    host.to_lowercase()
}

/// `host` equals `allowed`, or is a subdomain of it (`host` ends with `.allowed`).
fn host_matches(host: &str, allowed: &str) -> bool {
    let allowed = allowed.trim().to_lowercase();
    if allowed.is_empty() {
        return false;
    }
    host == allowed || host.ends_with(&format!(".{allowed}"))
}

#[derive(Debug, thiserror::Error)]
pub enum NetworkError {
    #[error("Network blocked: {0}")]
    Blocked(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Allowlist policy ───────────────────────────────────

    #[test]
    fn allowlist_permits_listed_domain() {
        let g = NetworkGuard::new(NetworkPolicy::Allowlist(vec![
            "api.openai.com".into(),
            "github.com".into(),
        ]));
        assert!(g.check("https://api.openai.com/v1/models").is_ok());
        assert!(g
            .check("https://github.com/repos/morpheus-sh/agenticbox")
            .is_ok());
    }

    #[test]
    fn allowlist_blocks_unlisted_domain() {
        let g = NetworkGuard::new(NetworkPolicy::Allowlist(vec!["api.openai.com".into()]));
        let result = g.check("https://evil.attacker.com/exfil");
        assert!(result.is_err());
        match result.unwrap_err() {
            NetworkError::Blocked(msg) => assert!(msg.contains("evil.attacker.com")),
        }
    }

    #[test]
    fn allowlist_blocks_when_empty() {
        let g = NetworkGuard::new(NetworkPolicy::Allowlist(vec![]));
        assert!(g.check("https://api.openai.com").is_err());
    }

    #[test]
    fn allowlist_subdomain_matching() {
        let g = NetworkGuard::new(NetworkPolicy::Allowlist(vec!["github.com".into()]));
        // The impl uses `contains`, so api.github.com contains "github.com"
        assert!(g.check("https://api.github.com/user").is_ok());
    }

    // ── Full policy ────────────────────────────────────────

    #[test]
    fn full_policy_allows_everything() {
        let g = NetworkGuard::new(NetworkPolicy::Full);
        assert!(g.check("https://evil.attacker.com").is_ok());
        assert!(g.check("https://api.openai.com").is_ok());
        assert!(g.check("http://192.168.1.1:8080").is_ok());
    }

    // ── LocalhostOnly policy ───────────────────────────────

    #[test]
    fn localhost_permits_localhost() {
        let g = NetworkGuard::new(NetworkPolicy::LocalhostOnly);
        assert!(g.check("http://localhost:3000").is_ok());
        assert!(g.check("http://127.0.0.1:8080").is_ok());
    }

    #[test]
    fn localhost_blocks_external() {
        let g = NetworkGuard::new(NetworkPolicy::LocalhostOnly);
        assert!(g.check("https://api.openai.com").is_err());
    }

    // ── Offline policy ─────────────────────────────────────

    #[test]
    fn offline_blocks_all() {
        let g = NetworkGuard::new(NetworkPolicy::Offline);
        assert!(g.check("https://api.openai.com").is_err());
        assert!(g.check("http://localhost").is_err());
        assert!(g.check("http://127.0.0.1").is_err());
    }

    #[test]
    fn offline_error_message() {
        let g = NetworkGuard::new(NetworkPolicy::Offline);
        let err = g.check("https://example.com").unwrap_err();
        match err {
            NetworkError::Blocked(msg) => assert!(msg.contains("Offline")),
        }
    }

    // ── Default policy ─────────────────────────────────────

    #[test]
    fn default_policy_is_offline() {
        // NetworkPolicy::default() is Offline
        let g = NetworkGuard::new(NetworkPolicy::default());
        assert!(g.check("https://anything.com").is_err());
    }
}
