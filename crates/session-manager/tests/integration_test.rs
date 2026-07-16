use session_manager::SessionManager;
use shared_types::{
    FsPermission, IdentityStatus, ModelConfig, NetworkPolicy, PermissionSet, SessionStatus,
};
use uuid::Uuid;

/// Helper: create a PermissionSet with reasonable defaults for testing.
fn test_perms() -> PermissionSet {
    PermissionSet {
        terminal: true,
        filesystem: FsPermission::ReadWrite,
        browser: false,
        network: NetworkPolicy::Allowlist(vec!["api.openai.com".into()]),
    }
}

/// Helper: create a ModelConfig for testing.
fn test_model() -> ModelConfig {
    ModelConfig {
        provider: "openai".into(),
        model: "gpt-4o".into(),
        api_key: None,
        base_url: None,
    }
}

#[tokio::test]
async fn create_and_retrieve_session() {
    let db_url = "sqlite::memory:";
    let mgr = SessionManager::new(db_url)
        .await
        .expect("failed to init session manager");

    let session = mgr
        .create("test-agent".into(), test_model(), test_perms(), None)
        .await
        .expect("failed to create session");

    assert_eq!(session.name, "test-agent");
    assert!(matches!(session.status, SessionStatus::Creating));

    // Retrieve by ID
    let retrieved = mgr.get(session.id).await.expect("query failed");
    assert!(retrieved.is_some());
    let retrieved = retrieved.unwrap();
    assert_eq!(retrieved.name, "test-agent");
    assert_eq!(retrieved.id, session.id);
}

#[tokio::test]
async fn create_multiple_and_list() {
    let db_url = "sqlite::memory:";
    let mgr = SessionManager::new(db_url).await.unwrap();

    let s1 = mgr
        .create("agent-1".into(), test_model(), test_perms(), None)
        .await
        .unwrap();
    let s2 = mgr
        .create("agent-2".into(), test_model(), test_perms(), None)
        .await
        .unwrap();
    let s3 = mgr
        .create("agent-3".into(), test_model(), test_perms(), None)
        .await
        .unwrap();

    let sessions = mgr.list().await.expect("list failed");
    assert_eq!(sessions.len(), 3);

    // List is ordered by created_at DESC — most recent first
    // All created within the same timestamp window, so just verify all are present
    let names: Vec<&str> = sessions.iter().map(|s| s.name.as_str()).collect();
    assert!(names.contains(&"agent-1"));
    assert!(names.contains(&"agent-2"));
    assert!(names.contains(&"agent-3"));

    // Verify all IDs are unique
    let ids: Vec<_> = sessions.iter().map(|s| s.id).collect();
    assert_eq!(ids.len(), 3);
    assert_ne!(s1.id, s2.id);
    assert_ne!(s2.id, s3.id);
}

#[tokio::test]
async fn get_nonexistent_session_returns_none() {
    let db_url = "sqlite::memory:";
    let mgr = SessionManager::new(db_url).await.unwrap();

    let random_id = uuid::Uuid::new_v4();
    let result = mgr.get(random_id).await.expect("query failed");
    assert!(result.is_none());
}

#[tokio::test]
async fn update_session_status() {
    let db_url = "sqlite::memory:";
    let mgr = SessionManager::new(db_url).await.unwrap();

    let session = mgr
        .create("status-test".into(), test_model(), test_perms(), None)
        .await
        .unwrap();

    // Initially Creating
    assert!(matches!(session.status, SessionStatus::Creating));

    // Transition to Running
    mgr.update_status(session.id, SessionStatus::Running)
        .await
        .expect("update failed");

    let updated = mgr.get(session.id).await.unwrap().unwrap();
    assert!(matches!(updated.status, SessionStatus::Running));

    // Transition to Paused
    mgr.update_status(session.id, SessionStatus::Paused)
        .await
        .unwrap();

    let updated = mgr.get(session.id).await.unwrap().unwrap();
    assert!(matches!(updated.status, SessionStatus::Paused));

    // Transition to Destroyed
    mgr.update_status(session.id, SessionStatus::Destroyed)
        .await
        .unwrap();

    let updated = mgr.get(session.id).await.unwrap().unwrap();
    assert!(matches!(updated.status, SessionStatus::Destroyed));
}

#[tokio::test]
async fn session_preserves_model_config() {
    let db_url = "sqlite::memory:";
    let mgr = SessionManager::new(db_url).await.unwrap();

    let model = ModelConfig {
        provider: "anthropic".into(),
        model: "claude-sonnet-4-20250514".into(),
        api_key: Some("sk-test-123".into()),
        base_url: Some("https://api.anthropic.com".into()),
    };

    let session = mgr
        .create("config-test".into(), model, test_perms(), None)
        .await
        .unwrap();
    let retrieved = mgr.get(session.id).await.unwrap().unwrap();

    assert_eq!(retrieved.model_config.provider, "anthropic");
    assert_eq!(retrieved.model_config.model, "claude-sonnet-4-20250514");
    assert_eq!(retrieved.model_config.api_key, Some("sk-test-123".into()));
    assert_eq!(
        retrieved.model_config.base_url,
        Some("https://api.anthropic.com".into())
    );
}

#[tokio::test]
async fn session_preserves_permissions() {
    let db_url = "sqlite::memory:";
    let mgr = SessionManager::new(db_url).await.unwrap();

    let perms = PermissionSet {
        terminal: false,
        filesystem: FsPermission::ReadOnly,
        browser: true,
        network: NetworkPolicy::Allowlist(vec!["github.com".into(), "api.openai.com".into()]),
    };

    let session = mgr
        .create("perms-test".into(), test_model(), perms, None)
        .await
        .unwrap();
    let retrieved = mgr.get(session.id).await.unwrap().unwrap();

    assert!(!retrieved.permissions.terminal);
    assert!(matches!(
        retrieved.permissions.filesystem,
        FsPermission::ReadOnly
    ));
    assert!(retrieved.permissions.browser);
    match &retrieved.permissions.network {
        NetworkPolicy::Allowlist(domains) => {
            assert_eq!(domains.len(), 2);
            assert!(domains.contains(&"github.com".to_string()));
        }
        other => panic!("expected Allowlist, got {:?}", other),
    }
}

#[tokio::test]
async fn empty_list_on_fresh_db() {
    let db_url = "sqlite::memory:";
    let mgr = SessionManager::new(db_url).await.unwrap();

    let sessions = mgr.list().await.expect("list failed");
    assert!(sessions.is_empty());
}

// ── Trust Recovery Tests ──────────────────────────────────────
//
// The trust recovery mechanism auto-promotes Monitored identities
// back to Active after RECOVERY_CLEAN_SESSIONS (3) consecutive clean
// sessions (delta >= 0). Any violation (delta < 0) resets the counter.

/// Helper: create an identity with a given trust_score and status directly in DB.
/// Returns the identity UUID.
async fn insert_test_identity(
    mgr: &SessionManager,
    id: Uuid,
    name: &str,
    trust_score: i32,
    status: &str,
    clean_sessions: i32,
) {
    let now = chrono::Utc::now().to_rfc3339();
    // We access the pool through a raw query since there's no public create_identity
    sqlx::query(
        "INSERT INTO agent_identities (id, name, display_name, vertical, created_at, status, trust_score, consecutive_clean_sessions) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(id.to_string())
    .bind(name)
    .bind(None::<String>)
    .bind(None::<String>)
    .bind(&now)
    .bind(status)
    .bind(trust_score)
    .bind(clean_sessions)
    .execute(
        mgr.get_pool(),
    )
    .await
    .unwrap();
}

#[tokio::test]
async fn monitored_recovers_after_three_clean_sessions() {
    let db_url = "sqlite::memory:";
    let mgr = SessionManager::new(db_url).await.unwrap();
    let id = Uuid::new_v4();

    // Insert a Monitored identity with trust_score -1 and 0 clean sessions
    insert_test_identity(&mgr, id, "test-recover-1", -1, "Monitored", 0).await;

    // First clean session → still Monitored (1/3)
    let updated = mgr.update_trust_score(id, 1).await.unwrap();
    assert_eq!(updated.consecutive_clean_sessions, 1);
    assert_eq!(updated.status, IdentityStatus::Monitored);

    // Second clean session → still Monitored (2/3)
    let updated = mgr.update_trust_score(id, 1).await.unwrap();
    assert_eq!(updated.consecutive_clean_sessions, 2);
    assert_eq!(updated.status, IdentityStatus::Monitored);

    // Third clean session → auto-recovers to Active (3/3)
    let updated = mgr.update_trust_score(id, 1).await.unwrap();
    assert_eq!(updated.consecutive_clean_sessions, 3);
    assert_eq!(updated.status, IdentityStatus::Active);
    assert_eq!(updated.trust_score, 2); // -1 + 1 + 1 + 1 = 2
}

#[tokio::test]
async fn monitored_with_two_clean_sessions_stays_monitored() {
    let db_url = "sqlite::memory:";
    let mgr = SessionManager::new(db_url).await.unwrap();
    let id = Uuid::new_v4();

    // Insert Monitored identity with -2 score and 0 clean sessions
    insert_test_identity(&mgr, id, "test-recover-2", -2, "Monitored", 0).await;

    // Two clean sessions → still Monitored (below RECOVERY_CLEAN_SESSIONS)
    let updated = mgr.update_trust_score(id, 1).await.unwrap();
    assert_eq!(updated.consecutive_clean_sessions, 1);
    assert_eq!(updated.status, IdentityStatus::Monitored);

    let updated = mgr.update_trust_score(id, 1).await.unwrap();
    assert_eq!(updated.consecutive_clean_sessions, 2);
    assert_eq!(updated.status, IdentityStatus::Monitored);
    assert_eq!(updated.trust_score, 0); // -2 + 1 + 1 = 0
}

#[tokio::test]
async fn violation_resets_clean_session_counter() {
    let db_url = "sqlite::memory:";
    let mgr = SessionManager::new(db_url).await.unwrap();
    let id = Uuid::new_v4();

    // Start with a confirmed Monitored identity (trust_score -1, 0 clean)
    insert_test_identity(&mgr, id, "test-violation-reset", -1, "Monitored", 0).await;

    // One clean session → counter = 1
    let updated = mgr.update_trust_score(id, 1).await.unwrap();
    assert_eq!(updated.consecutive_clean_sessions, 1);

    // Violation (network exfiltration → -5) → counter resets to 0
    let updated = mgr.update_trust_score(id, -5).await.unwrap();
    assert_eq!(updated.consecutive_clean_sessions, 0);
    // Score: -1 + 1 - 5 = -5 → Suspended
    assert_eq!(updated.trust_score, -5);
    assert_eq!(updated.status, IdentityStatus::Suspended);
}

#[tokio::test]
async fn active_identity_increments_clean_sessions_no_status_change() {
    let db_url = "sqlite::memory:";
    let mgr = SessionManager::new(db_url).await.unwrap();
    let id = Uuid::new_v4();

    // Insert an Active identity with score 5 and 0 clean sessions
    insert_test_identity(&mgr, id, "test-active-clean", 5, "Active", 0).await;

    // Clean session → counter increments, stays Active
    let updated = mgr.update_trust_score(id, 1).await.unwrap();
    assert_eq!(updated.consecutive_clean_sessions, 1);
    assert_eq!(updated.status, IdentityStatus::Active);
    assert_eq!(updated.trust_score, 6);
}

#[tokio::test]
async fn monitored_violation_exceeding_threshold_no_recovery() {
    let db_url = "sqlite::memory:";
    let mgr = SessionManager::new(db_url).await.unwrap();
    let id = Uuid::new_v4();

    // Monitored identity with score -3 and 2 clean sessions (close to recovery)
    insert_test_identity(&mgr, id, "test-near-recovery-violation", -3, "Monitored", 2).await;

    // A violation → counter resets, status may drop based on score
    let updated = mgr.update_trust_score(id, -3).await.unwrap();
    assert_eq!(updated.consecutive_clean_sessions, 0);
    assert_eq!(updated.trust_score, -6);
    assert_eq!(updated.status, IdentityStatus::Suspended); // -6 ≤ -5
}

#[tokio::test]
async fn multiple_clean_sessions_after_violation_gradual_recovery() {
    let db_url = "sqlite::memory:";
    let mgr = SessionManager::new(db_url).await.unwrap();
    let id = Uuid::new_v4();

    // Monitored identity with score -1 and 0 clean sessions
    insert_test_identity(&mgr, id, "test-gradual-recovery", -1, "Monitored", 0).await;

    // Previously had a violation, now building up clean sessions
    let updated = mgr.update_trust_score(id, 1).await.unwrap();
    assert_eq!(updated.consecutive_clean_sessions, 1);
    assert_eq!(updated.status, IdentityStatus::Monitored);

    let updated = mgr.update_trust_score(id, 1).await.unwrap();
    assert_eq!(updated.consecutive_clean_sessions, 2);
    assert_eq!(updated.status, IdentityStatus::Monitored);

    let updated = mgr.update_trust_score(id, 1).await.unwrap();
    assert_eq!(updated.consecutive_clean_sessions, 3);
    assert_eq!(updated.status, IdentityStatus::Active); // Recovered!

    // Next clean session → stays Active, counter keeps incrementing
    let updated = mgr.update_trust_score(id, 1).await.unwrap();
    assert_eq!(updated.consecutive_clean_sessions, 4);
    assert_eq!(updated.status, IdentityStatus::Active);
    assert_eq!(updated.trust_score, 3); // -1 + 1*4 = 3
}
