use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub type SessionId = Uuid;

/// Persistent identity for an agent — survives across sessions.
///
/// Every agent identity has a unique UUID, a human-readable name, an
/// optional vertical template reference, and a status that governs
/// whether new sessions can be started.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentIdentity {
    pub id: Uuid,
    pub name: String,
    pub display_name: Option<String>,
    pub vertical: Option<String>,
    pub created_at: DateTime<Utc>,
    pub status: IdentityStatus,
    pub trust_score: i32,
    /// Number of consecutive clean sessions (no deny events).
    /// Resets to 0 on any violation. Used for trust recovery:
    /// Monitored → Active after RECOVERY_CLEAN_SESSIONS clean sessions.
    pub consecutive_clean_sessions: u32,
}

/// Lifecycle status of an agent identity.
///
/// Active identities operate normally. Monitored identities require
/// human approval to start new sessions. Suspended identities cannot
/// start sessions. Revoked identities have their credentials rotated
/// and sessions killed.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum IdentityStatus {
    #[default]
    Active,
    Monitored,
    Suspended,
    Revoked,
}

/// A credential bound to an agent identity.
///
/// The credential value is encrypted and stored in the daemon's
/// database. The agent process receives the value as an environment
/// variable or file at container start, but never sees the credential
/// store itself.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CredentialBinding {
    pub id: Uuid,
    pub identity_id: Uuid,
    pub credential_name: String,
    pub credential_type: CredentialType,
    pub created_at: DateTime<Utc>,
    pub rotated_at: Option<DateTime<Utc>>,
}

/// How a credential is injected into the agent container.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum CredentialType {
    /// Injected as an environment variable (e.g. `OPENAI_API_KEY=sk-...`)
    #[default]
    Env,
    /// Written to a protected file path inside the container
    File,
    /// Reference to an external vault (Phase 3: Vault/AWS Secrets Manager)
    VaultRef,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: SessionId,
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub model_config: ModelConfig,
    pub permissions: PermissionSet,
    pub status: SessionStatus,
    /// Optional reference to a persistent agent identity.
    /// When set, the session is attributed to this identity in the audit log.
    #[serde(default)]
    pub identity_id: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateSessionRequest {
    pub name: String,
    pub model_config: ModelConfig,
    pub permissions: PermissionSet,
    /// Optional agent identity to associate this session with.
    #[serde(default)]
    pub identity_id: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SessionStatus {
    Creating,
    Running,
    Paused,
    Destroyed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfig {
    pub provider: String,
    pub model: String,
    pub api_key: Option<String>,
    pub base_url: Option<String>,
}

impl Default for ModelConfig {
    fn default() -> Self {
        Self {
            provider: "openai".into(),
            model: "gpt-4o".into(),
            api_key: None,
            base_url: None,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PermissionSet {
    pub terminal: bool,
    pub filesystem: FsPermission,
    pub browser: bool,
    pub network: NetworkPolicy,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum FsPermission {
    #[default]
    Deny,
    ReadOnly,
    ReadWrite,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum NetworkPolicy {
    #[default]
    Offline,
    Allowlist(Vec<String>),
    LocalhostOnly,
    Full,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub tool: String,
    pub arguments: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    pub call_id: String,
    pub output: serde_json::Value,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
    pub tool_calls: Option<Vec<ToolCall>>,
    pub tool_results: Option<Vec<ToolResult>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── ModelConfig ────────────────────────────────────────

    #[test]
    fn model_config_default_values() {
        let cfg = ModelConfig::default();
        assert_eq!(cfg.provider, "openai");
        assert_eq!(cfg.model, "gpt-4o");
        assert!(cfg.api_key.is_none());
        assert!(cfg.base_url.is_none());
    }

    #[test]
    fn model_config_serde_roundtrip() {
        let cfg = ModelConfig {
            provider: "anthropic".into(),
            model: "claude-sonnet-4-20250514".into(),
            api_key: Some("sk-test".into()),
            base_url: Some("https://api.anthropic.com".into()),
        };
        let json = serde_json::to_string(&cfg).unwrap();
        let deserialized: ModelConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.provider, cfg.provider);
        assert_eq!(deserialized.model, cfg.model);
        assert_eq!(deserialized.api_key, cfg.api_key);
        assert_eq!(deserialized.base_url, cfg.base_url);
    }

    // ── FsPermission ───────────────────────────────────────

    #[test]
    fn fs_permission_default_is_deny() {
        let fs = FsPermission::default();
        assert!(matches!(fs, FsPermission::Deny));
    }

    #[test]
    fn fs_permission_serde_camel_case() {
        let json = serde_json::to_string(&FsPermission::ReadOnly).unwrap();
        assert_eq!(json, "\"readOnly\"");
        let json = serde_json::to_string(&FsPermission::ReadWrite).unwrap();
        assert_eq!(json, "\"readWrite\"");
        let json = serde_json::to_string(&FsPermission::Deny).unwrap();
        assert_eq!(json, "\"deny\"");
    }

    #[test]
    fn fs_permission_deserialize_camel_case() {
        let fs: FsPermission = serde_json::from_str("\"readOnly\"").unwrap();
        assert!(matches!(fs, FsPermission::ReadOnly));
        let fs: FsPermission = serde_json::from_str("\"readWrite\"").unwrap();
        assert!(matches!(fs, FsPermission::ReadWrite));
    }

    // ── NetworkPolicy ──────────────────────────────────────

    #[test]
    fn network_policy_default_is_offline() {
        let np = NetworkPolicy::default();
        assert!(matches!(np, NetworkPolicy::Offline));
    }

    #[test]
    fn network_policy_serde_camel_case() {
        let json = serde_json::to_string(&NetworkPolicy::Offline).unwrap();
        assert_eq!(json, "\"offline\"");
        let json = serde_json::to_string(&NetworkPolicy::Full).unwrap();
        assert_eq!(json, "\"full\"");
        let json = serde_json::to_string(&NetworkPolicy::LocalhostOnly).unwrap();
        assert_eq!(json, "\"localhostOnly\"");
    }

    #[test]
    fn network_policy_allowlist_serde_roundtrip() {
        let policy = NetworkPolicy::Allowlist(vec!["api.openai.com".into(), "github.com".into()]);
        let json = serde_json::to_string(&policy).unwrap();
        let deserialized: NetworkPolicy = serde_json::from_str(&json).unwrap();
        match deserialized {
            NetworkPolicy::Allowlist(domains) => {
                assert_eq!(domains.len(), 2);
                assert!(domains.contains(&"api.openai.com".to_string()));
                assert!(domains.contains(&"github.com".to_string()));
            }
            other => panic!("expected Allowlist, got {:?}", other),
        }
    }

    // ── PermissionSet ──────────────────────────────────────

    #[test]
    fn permission_set_default_values() {
        let ps = PermissionSet::default();
        assert!(!ps.terminal);
        assert!(matches!(ps.filesystem, FsPermission::Deny));
        assert!(!ps.browser);
        assert!(matches!(ps.network, NetworkPolicy::Offline));
    }

    #[test]
    fn permission_set_serde_roundtrip() {
        let ps = PermissionSet {
            terminal: true,
            filesystem: FsPermission::ReadWrite,
            browser: false,
            network: NetworkPolicy::Allowlist(vec!["api.openai.com".into()]),
        };
        let json = serde_json::to_string(&ps).unwrap();
        let deserialized: PermissionSet = serde_json::from_str(&json).unwrap();
        assert!(deserialized.terminal);
        assert!(matches!(deserialized.filesystem, FsPermission::ReadWrite));
        assert!(!deserialized.browser);
        match deserialized.network {
            NetworkPolicy::Allowlist(d) => assert_eq!(d, vec!["api.openai.com"]),
            other => panic!("expected Allowlist, got {:?}", other),
        }
    }

    // ── SessionStatus ──────────────────────────────────────

    #[test]
    fn session_status_serde_roundtrip() {
        let json = serde_json::to_string(&SessionStatus::Running).unwrap();
        assert_eq!(json, "\"Running\"");

        let status: SessionStatus = serde_json::from_str("\"Paused\"").unwrap();
        assert!(matches!(status, SessionStatus::Paused));
    }

    // ── Full Session roundtrip ─────────────────────────────

    #[test]
    fn session_full_serde_roundtrip() {
        let session = Session {
            id: Uuid::new_v4(),
            name: "test-agent".into(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            model_config: ModelConfig::default(),
            permissions: PermissionSet::default(),
            status: SessionStatus::Running,
            identity_id: None,
        };
        let json = serde_json::to_string(&session).unwrap();
        let deserialized: Session = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.name, "test-agent");
        assert_eq!(deserialized.id, session.id);
        assert!(deserialized.identity_id.is_none());
    }

    #[test]
    fn session_with_identity_roundtrip() {
        let id = Uuid::new_v4();
        let identity_id = Uuid::new_v4();
        let session = Session {
            id,
            name: "aria-support".into(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            model_config: ModelConfig::default(),
            permissions: PermissionSet::default(),
            status: SessionStatus::Running,
            identity_id: Some(identity_id),
        };
        let json = serde_json::to_string(&session).unwrap();
        let deserialized: Session = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.identity_id, Some(identity_id));
    }

    #[test]
    fn identity_status_default_is_active() {
        assert_eq!(IdentityStatus::default(), IdentityStatus::Active);
    }

    #[test]
    fn identity_status_serde_roundtrip() {
        for status in &[
            IdentityStatus::Active,
            IdentityStatus::Monitored,
            IdentityStatus::Suspended,
            IdentityStatus::Revoked,
        ] {
            let json = serde_json::to_string(status).unwrap();
            let deserialized: IdentityStatus = serde_json::from_str(&json).unwrap();
            assert_eq!(deserialized, *status);
        }
    }

    #[test]
    fn credential_type_default_is_env() {
        assert_eq!(CredentialType::default(), CredentialType::Env);
    }

    #[test]
    fn credential_type_serde_roundtrip() {
        for ct in &[
            CredentialType::Env,
            CredentialType::File,
            CredentialType::VaultRef,
        ] {
            let json = serde_json::to_string(ct).unwrap();
            let deserialized: CredentialType = serde_json::from_str(&json).unwrap();
            assert_eq!(deserialized, *ct);
        }
    }

    #[test]
    fn agent_identity_serde_roundtrip() {
        let id = Uuid::new_v4();
        let identity = AgentIdentity {
            id,
            name: "aria-support".into(),
            display_name: Some("Aria — Customer Support Agent".into()),
            vertical: Some("customer-support".into()),
            created_at: chrono::Utc::now(),
            status: IdentityStatus::Active,
            trust_score: 10,
            consecutive_clean_sessions: 5,
        };
        let json = serde_json::to_string(&identity).unwrap();
        let deserialized: AgentIdentity = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.name, "aria-support");
        assert_eq!(deserialized.trust_score, 10);
        assert_eq!(deserialized.status, IdentityStatus::Active);
        assert_eq!(deserialized.consecutive_clean_sessions, 5);
    }

    #[test]
    fn credential_binding_serde_roundtrip() {
        let identity_id = Uuid::new_v4();
        let binding = CredentialBinding {
            id: Uuid::new_v4(),
            identity_id,
            credential_name: "OPENAI_API_KEY".into(),
            credential_type: CredentialType::Env,
            created_at: chrono::Utc::now(),
            rotated_at: None,
        };
        let json = serde_json::to_string(&binding).unwrap();
        let deserialized: CredentialBinding = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.credential_name, "OPENAI_API_KEY");
        assert_eq!(deserialized.identity_id, identity_id);
        assert_eq!(deserialized.credential_type, CredentialType::Env);
        assert!(deserialized.rotated_at.is_none());
    }

    // ── ToolCall / ToolResult ──────────────────────────────

    #[test]
    fn tool_call_serde() {
        let call = ToolCall {
            id: "call_1".into(),
            tool: "terminal".into(),
            arguments: serde_json::json!({"command": "ls"}),
        };
        let json = serde_json::to_string(&call).unwrap();
        let deserialized: ToolCall = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.tool, "terminal");
    }
}
