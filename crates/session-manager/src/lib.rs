use anyhow::Result;
use shared_types::{ModelConfig, PermissionSet, Session, SessionId, SessionStatus};
use sqlx::sqlite::SqlitePoolOptions;
use uuid::Uuid;

pub struct SessionManager {
    db: sqlx::SqlitePool,
}

impl SessionManager {
    /// Number of consecutive clean sessions required for a Monitored identity
    /// to auto-recover to Active status.
    pub const RECOVERY_CLEAN_SESSIONS: u32 = 3;

    pub async fn new(db_url: &str) -> Result<Self> {
        let pool = SqlitePoolOptions::new().connect(db_url).await?;
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS sessions (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                identity_id TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                model_config TEXT NOT NULL,
                permissions TEXT NOT NULL,
                status TEXT NOT NULL
            )
            "#,
        )
        .execute(&pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS agent_identities (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL UNIQUE,
                display_name TEXT,
                vertical TEXT,
                created_at TEXT NOT NULL,
                status TEXT NOT NULL DEFAULT 'Active',
                trust_score INTEGER NOT NULL DEFAULT 0,
                consecutive_clean_sessions INTEGER NOT NULL DEFAULT 0,
                metadata TEXT
            )
            "#,
        )
        .execute(&pool)
        .await?;

        // Migration: add consecutive_clean_sessions column if upgrading from an
        // earlier schema. Safe to call on every startup — ignores if column exists.
        let _ = sqlx::query(
            "ALTER TABLE agent_identities ADD COLUMN consecutive_clean_sessions INTEGER NOT NULL DEFAULT 0",
        )
        .execute(&pool)
        .await;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS credential_bindings (
                id TEXT PRIMARY KEY,
                identity_id TEXT NOT NULL REFERENCES agent_identities(id),
                credential_name TEXT NOT NULL,
                credential_type TEXT NOT NULL DEFAULT 'Env',
                encrypted_value BLOB,
                created_at TEXT NOT NULL,
                rotated_at TEXT,
                UNIQUE(identity_id, credential_name)
            )
            "#,
        )
        .execute(&pool)
        .await?;

        Ok(Self { db: pool })
    }

    /// Get a reference to the underlying database pool.
    /// Useful for direct queries in tests or advanced use cases.
    pub fn get_pool(&self) -> &sqlx::SqlitePool {
        &self.db
    }

    pub async fn create(
        &self,
        name: String,
        model_config: ModelConfig,
        permissions: PermissionSet,
        identity_id: Option<Uuid>,
    ) -> Result<Session> {
        let id = Uuid::new_v4();
        let now = chrono::Utc::now();
        let session = Session {
            id,
            name,
            created_at: now,
            updated_at: now,
            model_config,
            permissions,
            status: SessionStatus::Creating,
            identity_id,
        };
        let json_config = serde_json::to_string(&session.model_config)?;
        let json_perms = serde_json::to_string(&session.permissions)?;
        let status_str = serde_json::to_string(&session.status)?;
        sqlx::query(
            r#"INSERT INTO sessions (id, name, identity_id, created_at, updated_at, model_config, permissions, status)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)"#
        )
        .bind(session.id.to_string())
        .bind(&session.name)
        .bind(session.identity_id.map(|u| u.to_string()))
        .bind(session.created_at)
        .bind(session.updated_at)
        .bind(json_config)
        .bind(json_perms)
        .bind(status_str)
        .execute(&self.db).await?;
        Ok(session)
    }

    pub async fn list(&self) -> Result<Vec<Session>> {
        let rows =
            sqlx::query_as::<_, SessionRow>("SELECT * FROM sessions ORDER BY created_at DESC")
                .fetch_all(&self.db)
                .await?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    pub async fn get(&self, id: SessionId) -> Result<Option<Session>> {
        let row = sqlx::query_as::<_, SessionRow>("SELECT * FROM sessions WHERE id = ?")
            .bind(id.to_string())
            .fetch_optional(&self.db)
            .await?;
        Ok(row.map(Into::into))
    }

    pub async fn update_status(&self, id: SessionId, status: SessionStatus) -> Result<()> {
        let status_str = serde_json::to_string(&status)?;
        sqlx::query("UPDATE sessions SET status = ?, updated_at = ? WHERE id = ?")
            .bind(status_str)
            .bind(chrono::Utc::now())
            .bind(id.to_string())
            .execute(&self.db)
            .await?;
        Ok(())
    }

    // ── Identity lookup ────────────────────────────────────

    /// Get an agent identity by name.
    pub async fn get_identity_by_name(
        &self,
        name: &str,
    ) -> Result<Option<shared_types::AgentIdentity>> {
        let row = sqlx::query_as::<_, IdentityRow>("SELECT id, name, display_name, vertical, created_at, status, trust_score, consecutive_clean_sessions FROM agent_identities WHERE name = ?")
            .bind(name)
            .fetch_optional(&self.db)
            .await?;
        Ok(row.map(Into::into))
    }

    /// Get an agent identity by UUID.
    pub async fn get_identity_by_id(
        &self,
        id: Uuid,
    ) -> Result<Option<shared_types::AgentIdentity>> {
        let row = sqlx::query_as::<_, IdentityRow>("SELECT id, name, display_name, vertical, created_at, status, trust_score, consecutive_clean_sessions FROM agent_identities WHERE id = ?")
            .bind(id.to_string())
            .fetch_optional(&self.db)
            .await?;
        Ok(row.map(Into::into))
    }

    /// Update the trust score of an agent identity by applying a delta.
    ///
    /// The delta is added to the current score (can be negative). After
    /// applying, the identity's status is automatically adjusted:
    ///   score <= -15 → Revoked
    ///   score <=  -5 → Suspended
    ///   score <   0 → Monitored
    ///   score >=  0 → Active
    ///
    /// Returns the updated identity (new score + new status).
    pub async fn update_trust_score(
        &self,
        identity_id: Uuid,
        delta: i32,
    ) -> Result<shared_types::AgentIdentity> {
        // When delta >= 0, it's a clean session — increment consecutive_clean_sessions.
        // When delta < 0, it's a violation — reset to 0.
        // Then check if we should auto-recover from Monitored → Active.

        // First read current consecutive_clean_sessions and status
        let current: Option<(i32, String, i32)> = sqlx::query_as(
            "SELECT trust_score, status, consecutive_clean_sessions FROM agent_identities WHERE id = ?",
        )
        .bind(identity_id.to_string())
        .fetch_optional(&self.db)
        .await?;

        let (_old_score, _old_status, old_clean) = match current {
            Some(c) => c,
            None => return Err(anyhow::anyhow!("Identity not found: {}", identity_id)),
        };

        // Compute new consecutive_clean_sessions
        let new_clean = if delta >= 0 { old_clean + 1 } else { 0 };

        // Update trust_score and consecutive_clean_sessions atomically
        let new_score = _old_score + delta;

        // Determine new status based on score AND recovery state:
        //   - Score <= -15: Revoked
        //   - Score <= -5: Suspended
        //   - Was Monitored and not yet recovered: Monitored
        //   - Was Active but score dropped below 0: Monitored
        //   - All other cases: Active
        let recovery_threshold = Self::RECOVERY_CLEAN_SESSIONS as i32;
        let new_status = if new_score <= -15 {
            shared_types::IdentityStatus::Revoked
        } else if new_score <= -5 {
            shared_types::IdentityStatus::Suspended
        } else if _old_status == "Monitored" && new_clean < recovery_threshold {
            // Still in recovery: stay Monitored even if score is above 0
            shared_types::IdentityStatus::Monitored
        } else if _old_status == "Active" && new_score < 0 {
            // Active identity that dipped below 0 without reaching Suspended/Revoked
            shared_types::IdentityStatus::Monitored
        } else {
            // Fully recovered or normal
            shared_types::IdentityStatus::Active
        };

        let new_status_str = match new_status {
            shared_types::IdentityStatus::Active => "Active",
            shared_types::IdentityStatus::Monitored => "Monitored",
            shared_types::IdentityStatus::Suspended => "Suspended",
            shared_types::IdentityStatus::Revoked => "Revoked",
        };

        let row = sqlx::query_as::<_, IdentityRow>(
            "UPDATE agent_identities \
             SET trust_score = ?, status = ?, consecutive_clean_sessions = ? \
             WHERE id = ? \
             RETURNING id, name, display_name, vertical, created_at, status, trust_score, consecutive_clean_sessions",
        )
        .bind(new_score)
        .bind(new_status_str)
        .bind(new_clean)
        .bind(identity_id.to_string())
        .fetch_one(&self.db)
        .await?;

        Ok(shared_types::AgentIdentity {
            id: row.id.parse().unwrap_or_else(|_| Uuid::nil()),
            name: row.name,
            display_name: row.display_name,
            vertical: row.vertical,
            created_at: row
                .created_at
                .parse()
                .unwrap_or_else(|_| chrono::Utc::now()),
            status: new_status,
            trust_score: row.trust_score,
            consecutive_clean_sessions: row.consecutive_clean_sessions as u32,
        })
    }
}

// ── Row structs for sqlx ─────────────────────────────────

#[derive(sqlx::FromRow)]
struct SessionRow {
    id: String,
    name: String,
    identity_id: Option<String>,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
    model_config: String,
    permissions: String,
    status: String,
}

impl From<SessionRow> for Session {
    fn from(row: SessionRow) -> Self {
        Session {
            id: row.id.parse().unwrap_or_else(|_| Uuid::nil()),
            name: row.name,
            identity_id: row.identity_id.and_then(|s| s.parse().ok()),
            created_at: row.created_at,
            updated_at: row.updated_at,
            model_config: serde_json::from_str(&row.model_config).unwrap_or_default(),
            permissions: serde_json::from_str(&row.permissions).unwrap_or_default(),
            status: serde_json::from_str(&row.status).unwrap_or(SessionStatus::Creating),
        }
    }
}

#[derive(sqlx::FromRow)]
struct IdentityRow {
    id: String,
    name: String,
    display_name: Option<String>,
    vertical: Option<String>,
    created_at: String,
    status: String,
    trust_score: i32,
    consecutive_clean_sessions: i32,
}

impl From<IdentityRow> for shared_types::AgentIdentity {
    fn from(row: IdentityRow) -> Self {
        shared_types::AgentIdentity {
            id: row.id.parse().unwrap_or_else(|_| Uuid::nil()),
            name: row.name,
            display_name: row.display_name,
            vertical: row.vertical,
            created_at: row
                .created_at
                .parse()
                .unwrap_or_else(|_| chrono::Utc::now()),
            status: match row.status.as_str() {
                "Monitored" => shared_types::IdentityStatus::Monitored,
                "Suspended" => shared_types::IdentityStatus::Suspended,
                "Revoked" => shared_types::IdentityStatus::Revoked,
                _ => shared_types::IdentityStatus::Active,
            },
            trust_score: row.trust_score,
            consecutive_clean_sessions: row.consecutive_clean_sessions as u32,
        }
    }
}

/// Map a trust score to an identity status.
///
/// Thresholds:
///   score <= -15 → Revoked (credential rotation, sessions killed)
///   score <=  -5 → Suspended (cannot start new sessions)
///   score <   0 → Monitored (requires human approval to start)
///   score >=  0 → Active (normal operation)
pub fn trust_score_to_status(score: i32) -> shared_types::IdentityStatus {
    use shared_types::IdentityStatus;
    if score <= -15 {
        IdentityStatus::Revoked
    } else if score <= -5 {
        IdentityStatus::Suspended
    } else if score < 0 {
        IdentityStatus::Monitored
    } else {
        IdentityStatus::Active
    }
}

// ─── Tests ────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn score_above_zero_is_active() {
        assert_eq!(
            trust_score_to_status(10),
            shared_types::IdentityStatus::Active
        );
        assert_eq!(
            trust_score_to_status(0),
            shared_types::IdentityStatus::Active
        );
        assert_eq!(
            trust_score_to_status(100),
            shared_types::IdentityStatus::Active
        );
    }

    #[test]
    fn score_minus_one_to_minus_four_is_monitored() {
        assert_eq!(
            trust_score_to_status(-1),
            shared_types::IdentityStatus::Monitored
        );
        assert_eq!(
            trust_score_to_status(-4),
            shared_types::IdentityStatus::Monitored
        );
    }

    #[test]
    fn score_minus_five_to_minus_fourteen_is_suspended() {
        assert_eq!(
            trust_score_to_status(-5),
            shared_types::IdentityStatus::Suspended
        );
        assert_eq!(
            trust_score_to_status(-10),
            shared_types::IdentityStatus::Suspended
        );
        assert_eq!(
            trust_score_to_status(-14),
            shared_types::IdentityStatus::Suspended
        );
    }

    #[test]
    fn score_below_minus_fifteen_is_revoked() {
        assert_eq!(
            trust_score_to_status(-15),
            shared_types::IdentityStatus::Revoked
        );
        assert_eq!(
            trust_score_to_status(-20),
            shared_types::IdentityStatus::Revoked
        );
        assert_eq!(
            trust_score_to_status(-100),
            shared_types::IdentityStatus::Revoked
        );
    }

    #[test]
    fn score_at_boundaries() {
        assert_eq!(
            trust_score_to_status(0),
            shared_types::IdentityStatus::Active
        );
        assert_eq!(
            trust_score_to_status(-1),
            shared_types::IdentityStatus::Monitored
        );
        assert_eq!(
            trust_score_to_status(-5),
            shared_types::IdentityStatus::Suspended
        );
        assert_eq!(
            trust_score_to_status(-15),
            shared_types::IdentityStatus::Revoked
        );
    }
}
