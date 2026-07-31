use anyhow::Result;
use shared_types::{ModelConfig, PermissionSet, Session, SessionId, SessionStatus};
use sqlx::sqlite::SqlitePoolOptions;
use uuid::Uuid;

pub struct SessionManager {
    db: sqlx::SqlitePool,
}

impl SessionManager {
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
                metadata TEXT
            )
            "#,
        )
        .execute(&pool)
        .await?;

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
        let row = sqlx::query_as::<_, IdentityRow>("SELECT id, name, display_name, vertical, created_at, status, trust_score FROM agent_identities WHERE name = ?")
            .bind(name)
            .fetch_optional(&self.db)
            .await?;
        Ok(row.map(Into::into))
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
        }
    }
}
