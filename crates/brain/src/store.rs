//! Persistent knowledge store: one SQLite `knowledge` table for everything,
//! plus an FTS5 index for exact-token search (error strings, flags, hostnames)
//! and a BLOB column for embedding vectors (semantic search).
//!
//! The schema is deliberately source-agnostic — every connector emits the same
//! row shape (the Cerebras "one embeddings table" idea). A `content_hash`
//! column makes re-ingestion incremental: rows whose content is unchanged are
//! skipped, so re-indexing a repo or thread never churns storage.

use anyhow::{Context, Result};
use sha2::{Digest, Sha256};
use sqlx::sqlite::{SqlitePool, SqlitePoolOptions};
use sqlx::{FromRow, Row};

/// A stored knowledge row. `embedding` is a packed `f32` LE vector.
#[derive(Debug, Clone, FromRow)]
pub struct KnowledgeRow {
    pub id: i64,
    pub source: String,
    pub source_id: String,
    pub chunk_key: String,
    pub kind: String,
    pub title: Option<String>,
    pub author: Option<String>,
    pub summary: Option<String>,
    pub question: Option<String>,
    pub resolution: Option<String>,
    pub systems: Option<String>,
    pub raw_text: String,
    pub embedding: Option<Vec<u8>>,
    pub dim: i64,
    pub distilled_by: Option<String>,
    pub embedded_by: Option<String>,
    pub project: Option<String>,
    pub created_at: i64,
    pub ingested_at: i64,
    pub content_hash: String,
}

/// An insert/update payload. `chunk_key` is the dedup identity
/// (`source + source_id + granularity`).
#[derive(Debug, Clone)]
pub struct NewKnowledge {
    pub source: String,
    pub source_id: String,
    pub chunk_key: String,
    pub kind: String,
    pub title: Option<String>,
    pub author: Option<String>,
    pub summary: Option<String>,
    pub question: Option<String>,
    pub resolution: Option<String>,
    pub systems: Vec<String>,
    pub raw_text: String,
    pub embedding: Option<Vec<f32>>,
    pub distilled_by: Option<String>,
    pub embedded_by: Option<String>,
    pub project: Option<String>,
    pub created_at: i64,
}

impl NewKnowledge {
    /// SHA-256 over the stable content fields — unchanged content ⇒ unchanged
    /// hash ⇒ no-op on re-ingest (incremental indexing, never a rebuild).
    pub fn content_hash(&self) -> String {
        let mut h = Sha256::new();
        h.update(self.source.as_bytes());
        h.update([0u8]);
        h.update(self.source_id.as_bytes());
        h.update([0u8]);
        h.update(self.chunk_key.as_bytes());
        h.update([0u8]);
        h.update(self.raw_text.as_bytes());
        format!("{:x}", h.finalize())
    }
}

/// Store-level statistics for the cost/coverage dashboard.
#[derive(Debug, Clone, Default)]
pub struct StoreStats {
    pub total_rows: i64,
    pub rows_with_embedding: i64,
    pub rows_with_distill: i64,
    pub by_source: Vec<(String, i64)>,
}

#[derive(Debug, Clone)]
pub struct RankedHit {
    pub id: i64,
    pub score: f64,
}

pub struct KnowledgeStore {
    pool: SqlitePool,
    /// False when the SQLite build lacks FTS5; exact search then falls back to LIKE.
    fts: bool,
}

impl KnowledgeStore {
    /// Connect and ensure the schema exists. `url` is a sqlx sqlite URL, e.g.
    /// `sqlite:data/brain.db?mode=rwc` (or `sqlite::memory:` in tests).
    pub async fn connect(url: &str) -> Result<Self> {
        let pool = SqlitePoolOptions::new()
            .max_connections(4)
            .connect(url)
            .await
            .context("brain: connect to knowledge store")?;
        let fts = Self::init_schema(&pool).await?;
        Ok(Self { pool, fts })
    }

    /// Returns true when FTS5 is available. Graceful degradation: a build
    /// without FTS5 still works via LIKE-based exact search.
    async fn init_schema(pool: &SqlitePool) -> Result<bool> {
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS knowledge (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                source TEXT NOT NULL,
                source_id TEXT NOT NULL,
                chunk_key TEXT NOT NULL UNIQUE,
                kind TEXT NOT NULL,
                title TEXT,
                author TEXT,
                summary TEXT,
                question TEXT,
                resolution TEXT,
                systems TEXT,
                raw_text TEXT NOT NULL,
                embedding BLOB,
                dim INTEGER,
                distilled_by TEXT,
                embedded_by TEXT,
                project TEXT,
                created_at INTEGER NOT NULL,
                ingested_at INTEGER NOT NULL,
                content_hash TEXT NOT NULL
            )",
        )
        .execute(pool)
        .await?;
        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_knowledge_source ON knowledge(source, source_id)",
        )
        .execute(pool)
        .await?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_knowledge_project ON knowledge(project)")
            .execute(pool)
            .await?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_knowledge_hash ON knowledge(content_hash)")
            .execute(pool)
            .await?;

        let fts = sqlx::query(
            "CREATE VIRTUAL TABLE IF NOT EXISTS knowledge_fts USING fts5(
                title, summary, question, resolution, raw_text,
                content='knowledge', content_rowid='id'
            )",
        )
        .execute(pool)
        .await
        .map(|_| true)
        .unwrap_or_else(|e| {
            tracing::warn!(error = %e, "brain: FTS5 unavailable — exact search falls back to LIKE");
            false
        });

        if fts {
            sqlx::query(
                "CREATE TRIGGER IF NOT EXISTS knowledge_ai AFTER INSERT ON knowledge BEGIN
                    INSERT INTO knowledge_fts(rowid, title, summary, question, resolution, raw_text)
                    VALUES (new.id, new.title, new.summary, new.question, new.resolution, new.raw_text);
                END",
            )
            .execute(pool)
            .await?;
            sqlx::query(
                "CREATE TRIGGER IF NOT EXISTS knowledge_ad AFTER DELETE ON knowledge BEGIN
                    INSERT INTO knowledge_fts(knowledge_fts, rowid, title, summary, question, resolution, raw_text)
                    VALUES ('delete', old.id, old.title, old.summary, old.question, old.resolution, old.raw_text);
                END",
            )
            .execute(pool)
            .await?;
            sqlx::query(
                "CREATE TRIGGER IF NOT EXISTS knowledge_au AFTER UPDATE ON knowledge BEGIN
                    INSERT INTO knowledge_fts(knowledge_fts, rowid, title, summary, question, resolution, raw_text)
                    VALUES ('delete', old.id, old.title, old.summary, old.question, old.resolution, old.raw_text);
                    INSERT INTO knowledge_fts(rowid, title, summary, question, resolution, raw_text)
                    VALUES (new.id, new.title, new.summary, new.question, new.resolution, new.raw_text);
                END",
            )
            .execute(pool)
            .await?;
        }
        Ok(fts)
    }

    /// Upsert rows. Returns `(inserted, updated, skipped)` — skipped rows had
    /// an identical `content_hash` (nothing changed since last ingest).
    pub async fn upsert(&self, rows: &[NewKnowledge]) -> Result<(usize, usize, usize)> {
        let mut inserted = 0usize;
        let mut updated = 0usize;
        let mut skipped = 0usize;
        for row in rows {
            let hash = row.content_hash();
            let existing: Option<(i64, String)> =
                sqlx::query_as("SELECT id, content_hash FROM knowledge WHERE chunk_key = ?1")
                    .bind(&row.chunk_key)
                    .fetch_optional(&self.pool)
                    .await?;
            let is_update = existing.is_some();
            if let Some((_id, prev_hash)) = &existing {
                if *prev_hash == hash {
                    skipped += 1;
                    continue;
                }
            }
            let embedding_bytes = row
                .embedding
                .as_ref()
                .map(|v| v.iter().flat_map(|f| f.to_le_bytes()).collect::<Vec<u8>>());
            let systems = if row.systems.is_empty() {
                None
            } else {
                Some(row.systems.join(","))
            };
            sqlx::query(
                "INSERT INTO knowledge (
                    source, source_id, chunk_key, kind, title, author, summary, question,
                    resolution, systems, raw_text, embedding, dim, distilled_by, embedded_by,
                    project, created_at, ingested_at, content_hash
                ) VALUES (
                    ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19
                )
                ON CONFLICT(chunk_key) DO UPDATE SET
                    title = excluded.title, author = excluded.author,
                    summary = excluded.summary, question = excluded.question,
                    resolution = excluded.resolution, systems = excluded.systems,
                    raw_text = excluded.raw_text, embedding = excluded.embedding,
                    dim = excluded.dim, distilled_by = excluded.distilled_by,
                    embedded_by = excluded.embedded_by, project = excluded.project,
                    created_at = excluded.created_at, ingested_at = excluded.ingested_at,
                    content_hash = excluded.content_hash",
            )
            .bind(&row.source)
            .bind(&row.source_id)
            .bind(&row.chunk_key)
            .bind(&row.kind)
            .bind(&row.title)
            .bind(&row.author)
            .bind(&row.summary)
            .bind(&row.question)
            .bind(&row.resolution)
            .bind(&systems)
            .bind(&row.raw_text)
            .bind(&embedding_bytes)
            .bind(row.embedding.as_ref().map(|v| v.len() as i64))
            .bind(&row.distilled_by)
            .bind(&row.embedded_by)
            .bind(&row.project)
            .bind(row.created_at)
            .bind(chrono::Utc::now().timestamp())
            .bind(&hash)
            .execute(&self.pool)
            .await?;
            if is_update {
                updated += 1;
            } else {
                inserted += 1;
            }
        }
        Ok((inserted, updated, skipped))
    }

    fn row_from(sqlx_row: sqlx::sqlite::SqliteRow) -> Result<KnowledgeRow> {
        let embedding: Option<Vec<u8>> = sqlx_row.try_get("embedding")?;
        let systems: Option<String> = sqlx_row.try_get("systems")?;
        Ok(KnowledgeRow {
            id: sqlx_row.try_get("id")?,
            source: sqlx_row.try_get("source")?,
            source_id: sqlx_row.try_get("source_id")?,
            chunk_key: sqlx_row.try_get("chunk_key")?,
            kind: sqlx_row.try_get("kind")?,
            title: sqlx_row.try_get("title")?,
            author: sqlx_row.try_get("author")?,
            summary: sqlx_row.try_get("summary")?,
            question: sqlx_row.try_get("question")?,
            resolution: sqlx_row.try_get("resolution")?,
            systems,
            raw_text: sqlx_row.try_get("raw_text")?,
            embedding: embedding.filter(|b| !b.is_empty()),
            dim: sqlx_row.try_get("dim")?,
            distilled_by: sqlx_row.try_get("distilled_by")?,
            embedded_by: sqlx_row.try_get("embedded_by")?,
            project: sqlx_row.try_get("project")?,
            created_at: sqlx_row.try_get("created_at")?,
            ingested_at: sqlx_row.try_get("ingested_at")?,
            content_hash: sqlx_row.try_get("content_hash")?,
        })
    }

    /// Exact-token search (FTS5, or LIKE fallback). FTS5 rank is negative
    /// (bm25); we negate it so higher = better, then the caller applies
    /// recency and fusion.
    pub async fn exact_search(
        &self,
        query: &str,
        project: Option<&str>,
        limit: usize,
    ) -> Result<Vec<RankedHit>> {
        if self.fts {
            // Phrase-escape: wrap the whole query in double quotes ("" escapes
            // embedded quotes) so punctuation like `::` and `-` isn't parsed
            // as FTS5 syntax.
            let phrase = query.replace('"', "\"\"");
            let base_sql = "SELECT k.id AS id, -kf.rank AS score
                       FROM knowledge_fts kf JOIN knowledge k ON k.id = kf.rowid
                       WHERE knowledge_fts MATCH ?1
                       ORDER BY kf.rank LIMIT ?2";
            let scoped_sql = "SELECT k.id AS id, -kf.rank AS score
                       FROM knowledge_fts kf JOIN knowledge k ON k.id = kf.rowid
                       WHERE knowledge_fts MATCH ?1 AND k.project = ?3
                       ORDER BY kf.rank LIMIT ?2";
            let (sql, phrase_bind, project_bind): (&str, &str, Option<String>) = match project {
                Some(p) => (scoped_sql, &phrase, Some(p.to_string())),
                None => (base_sql, &phrase, None),
            };
            let mut q = sqlx::query(sql)
                .bind(format!("\"{phrase_bind}\""))
                .bind(limit as i64);
            if let Some(p) = project_bind {
                q = q.bind(p);
            }
            let rows = q.fetch_all(&self.pool).await?;
            Ok(rows
                .iter()
                .map(|r| -> Result<RankedHit> {
                    Ok(RankedHit {
                        id: r.try_get("id")?,
                        score: r.try_get::<f64, _>("score")?,
                    })
                })
                .collect::<Result<Vec<_>>>()?)
        } else {
            // LIKE fallback across the searchable text fields.
            let like = format!("%{}%", query.replace('%', "\\%").replace('_', "\\_"));
            let mut q = sqlx::query(
                "SELECT id, 1.0 AS score FROM knowledge
                 WHERE (raw_text LIKE ?1 ESCAPE '\\' OR title LIKE ?1 ESCAPE '\\'
                        OR summary LIKE ?1 ESCAPE '\\' OR question LIKE ?1 ESCAPE '\\')
                 ORDER BY created_at DESC LIMIT ?2",
            )
            .bind(&like)
            .bind(limit as i64);
            if let Some(p) = project {
                q = sqlx::query(
                    "SELECT id, 1.0 AS score FROM knowledge
                     WHERE (raw_text LIKE ?1 ESCAPE '\\' OR title LIKE ?1 ESCAPE '\\'
                            OR summary LIKE ?1 ESCAPE '\\' OR question LIKE ?1 ESCAPE '\\')
                       AND project = ?3
                     ORDER BY created_at DESC LIMIT ?2",
                )
                .bind(&like)
                .bind(limit as i64)
                .bind(p.to_string());
            }
            let rows = q.fetch_all(&self.pool).await?;
            Ok(rows
                .iter()
                .map(|r| -> Result<RankedHit> {
                    Ok(RankedHit {
                        id: r.try_get("id")?,
                        score: r.try_get::<f64, _>("score")?,
                    })
                })
                .collect::<Result<Vec<_>>>()?)
        }
    }

    /// Brute-force cosine scan over stored embeddings. Deliberately simple:
    /// fine to ~100k rows (each row is a few µs of dot product); past that,
    /// swap to sqlite-vec or pgvector without touching the rest of the stack.
    pub async fn semantic_search(
        &self,
        query_embedding: &[f32],
        project: Option<&str>,
        limit: usize,
    ) -> Result<Vec<RankedHit>> {
        let mut q =
            sqlx::query("SELECT id, embedding, dim FROM knowledge WHERE embedding IS NOT NULL");
        if let Some(p) = project {
            q = sqlx::query(
                "SELECT id, embedding, dim FROM knowledge
                 WHERE embedding IS NOT NULL AND project = ?1",
            )
            .bind(p.to_string());
        }
        let rows = q.fetch_all(&self.pool).await?;
        let mut scored: Vec<(i64, f64)> = Vec::with_capacity(rows.len());
        for r in rows {
            let bytes: Option<Vec<u8>> = r.try_get("embedding")?;
            let Some(bytes) = bytes else { continue };
            let vec: Vec<f32> = bytes
                .chunks_exact(4)
                .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
                .collect();
            let sim = crate::retrieval::cosine(query_embedding, &vec);
            if sim > 0.0 {
                scored.push((r.try_get::<i64, _>("id")?, sim as f64));
            }
        }
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(limit);
        scored
            .into_iter()
            .map(|(id, score)| -> Result<RankedHit> { Ok(RankedHit { id, score }) })
            .collect()
    }

    /// Fetch full rows by id, preserving the given order.
    pub async fn rows_by_id(&self, ids: &[i64]) -> Result<Vec<KnowledgeRow>> {
        if ids.is_empty() {
            return Ok(vec![]);
        }
        let placeholders = (0..ids.len())
            .map(|i| format!("?{}", i + 1))
            .collect::<Vec<_>>()
            .join(",");
        let sql = format!("SELECT * FROM knowledge WHERE id IN ({placeholders})");
        let mut q = sqlx::query(&sql);
        for id in ids {
            q = q.bind(*id);
        }
        let rows = q.fetch_all(&self.pool).await?;
        rows.into_iter().map(Self::row_from).collect()
    }

    /// Look up a row by its connector identity.
    pub async fn by_source(&self, source: &str, source_id: &str) -> Result<Option<KnowledgeRow>> {
        let row =
            sqlx::query("SELECT * FROM knowledge WHERE source = ?1 AND source_id = ?2 LIMIT 1")
                .bind(source)
                .bind(source_id)
                .fetch_optional(&self.pool)
                .await?;
        match row {
            Some(r) => Ok(Some(Self::row_from(r)?)),
            None => Ok(None),
        }
    }

    /// Coverage + provenance stats for the dashboard.
    pub async fn stats(&self) -> Result<StoreStats> {
        let total_rows: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM knowledge")
            .fetch_one(&self.pool)
            .await?;
        let rows_with_embedding: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM knowledge WHERE embedding IS NOT NULL")
                .fetch_one(&self.pool)
                .await?;
        let rows_with_distill: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM knowledge WHERE summary IS NOT NULL")
                .fetch_one(&self.pool)
                .await?;
        let by_source: Vec<(String, i64)> = sqlx::query_as(
            "SELECT source, COUNT(*) AS n FROM knowledge GROUP BY source ORDER BY n DESC",
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(StoreStats {
            total_rows,
            rows_with_embedding,
            rows_with_distill,
            by_source,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(n: u32, raw: &str, embedding: Option<Vec<f32>>) -> NewKnowledge {
        NewKnowledge {
            source: "test".into(),
            source_id: format!("{n}"),
            chunk_key: format!("test/{n}"),
            kind: "doc".into(),
            title: Some(format!("Doc {n}")),
            author: Some("alice".into()),
            summary: Some(format!("summary {n}")),
            question: Some(format!("question {n}")),
            resolution: Some("resolved".into()),
            systems: vec!["core".into()],
            raw_text: raw.into(),
            embedding,
            distilled_by: Some("deepseek-v4-flash".into()),
            embedded_by: Some("qwen-embed".into()),
            project: Some("agenticbox-core".into()),
            created_at: 1_700_000_000 + i64::from(n),
        }
    }

    #[tokio::test]
    async fn upsert_dedupes_by_content_hash() {
        let store = KnowledgeStore::connect("sqlite::memory:").await.unwrap();
        let r = row(1, "docker socket path on macOS", None);
        let (ins, upd, skip) = store.upsert(std::slice::from_ref(&r)).await.unwrap();
        assert_eq!((ins, upd, skip), (1, 0, 0));
        let (ins, upd, skip) = store.upsert(std::slice::from_ref(&r)).await.unwrap();
        assert_eq!((ins, upd, skip), (0, 0, 1), "identical content must skip");
        let changed = NewKnowledge {
            raw_text: "docker socket path on macOS changed".into(),
            ..r
        };
        let (ins, upd, skip) = store.upsert(&[changed]).await.unwrap();
        assert_eq!((ins, upd, skip), (0, 1, 0), "changed content must update");
    }

    #[tokio::test]
    async fn exact_search_finds_error_string() {
        let store = KnowledgeStore::connect("sqlite::memory:").await.unwrap();
        store
            .upsert(&[row(
                1,
                "error: DATABASE_URL must include ?mode=rwc on first run",
                None,
            )])
            .await
            .unwrap();
        store
            .upsert(&[row(2, "colima start takes ~30s", None)])
            .await
            .unwrap();
        let hits = store.exact_search("?mode=rwc", None, 10).await.unwrap();
        assert!(
            !hits.is_empty() && hits[0].id == 1,
            "exact token match must rank first: {hits:?}"
        );
    }

    #[tokio::test]
    async fn semantic_search_ranks_similar_higher() {
        let store = KnowledgeStore::connect("sqlite::memory:").await.unwrap();
        let near = row(1, "restore hangs after manifest load", Some(vec![1.0, 0.0]));
        let far = row(2, "checkpoint stalls on nfs mount", Some(vec![0.0, 1.0]));
        store.upsert(&[near, far]).await.unwrap();
        let hits = store.semantic_search(&[0.9, 0.1], None, 10).await.unwrap();
        assert_eq!(hits[0].id, 1);
        assert_eq!(hits[1].id, 2);
    }

    #[tokio::test]
    async fn project_scoping_filters() {
        let store = KnowledgeStore::connect("sqlite::memory:").await.unwrap();
        store
            .upsert(&[row(1, "billing escalation runbook", None)])
            .await
            .unwrap();
        let other = NewKnowledge {
            project: Some("marketing".into()),
            ..row(2, "billing escalation runbook", None)
        };
        store.upsert(&[other]).await.unwrap();
        let hits = store
            .exact_search("billing", Some("agenticbox-core"), 10)
            .await
            .unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].id, 1);
    }

    #[tokio::test]
    async fn stats_counts_embedding_coverage() {
        let store = KnowledgeStore::connect("sqlite::memory:").await.unwrap();
        store
            .upsert(&[
                row(1, "a", Some(vec![1.0])),
                row(2, "b", Some(vec![1.0])),
                row(3, "c", None),
            ])
            .await
            .unwrap();
        let s = store.stats().await.unwrap();
        assert_eq!(s.total_rows, 3);
        assert_eq!(s.rows_with_embedding, 2);
        assert_eq!(s.rows_with_distill, 3);
    }
}
