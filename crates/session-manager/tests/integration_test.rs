use session_manager::SessionManager;
use shared_types::{FsPermission, ModelConfig, NetworkPolicy, PermissionSet, SessionStatus};

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
