//! Persistent, tamper-evident audit logging for AgenticBox.
//!
//! Every policy decision (Allow/Deny) is recorded as an `AuditEntry` and
//! appended to a JSONL file. Entries are sequential, chain-hashed (each
//! entry includes a hash of the previous entry), and timestamped — forming
//! a deterministic, verifiable audit trail.
//!
//! This is the "Accountability" pillar: every agent action is attributed,
//! logged, and auditable. No LLM, no probabilistic logic — pure deterministic
//! record-keeping that a CISO can trust.

use chrono::{DateTime, Utc};
use fs2::FileExt;
use serde::{Deserialize, Serialize};
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use thiserror::Error;
use uuid::Uuid;

/// Default maximum size for a single audit log file before rotation (10 MB).
pub const DEFAULT_MAX_LOG_SIZE: u64 = 10 * 1024 * 1024;
/// Default maximum age of an audit log file before rotation (30 days).
pub const DEFAULT_MAX_LOG_AGE_DAYS: u64 = 30;
/// Default number of rotated log files to keep.
pub const DEFAULT_MAX_LOG_FILES: usize = 5;

#[derive(Debug, Error)]
pub enum AuditError {
    #[error("audit log I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("audit log serialization error: {0}")]
    Serialize(#[from] serde_json::Error),
    #[error("audit log chain broken at entry {seq}: expected prev_hash {expected}, got {actual}")]
    ChainBroken {
        seq: u64,
        expected: String,
        actual: String,
    },
}

/// The outcome of a policy evaluation — mirrors `PolicyDecision` but is
/// self-contained for the audit log (no dependency on the policy-engine crate).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Decision {
    Allow,
    Deny(String),
}

impl Decision {
    pub fn is_allowed(&self) -> bool {
        matches!(self, Decision::Allow)
    }

    pub fn is_denied(&self) -> bool {
        matches!(self, Decision::Deny(_))
    }

    pub fn reason(&self) -> &str {
        match self {
            Decision::Allow => "within permissions",
            Decision::Deny(r) => r,
        }
    }
}

/// A single audit entry — one policy decision for one agent action.
///
/// Entries are append-only and chain-hashed: `prev_hash` is the SHA-256 of
/// the previous entry's `self_hash`. The first entry has `prev_hash = "genesis"`.
/// This makes tampering detectable — modifying or removing any entry breaks
/// the chain for all subsequent entries.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    /// Sequential entry number (1-based, monotonically increasing)
    pub seq: u64,
    /// ISO 8601 timestamp
    pub timestamp: DateTime<Utc>,
    /// Session ID (links to the agent session that triggered this decision)
    pub session_id: Uuid,
    /// Agent name (human-readable, from the manifest)
    pub agent_name: String,
    /// Optional persistent agent identity UUID (links to AgentIdentity)
    /// When set, the entry is attributed to a specific agent identity
    /// that survives across sessions.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub identity_id: Option<Uuid>,
    /// Action type (e.g. "fs:read", "network:outbound", "terminal:exec")
    pub action: String,
    /// Resource being accessed (e.g. file path, URL, command string)
    pub resource: String,
    /// The policy decision
    pub decision: Decision,
    /// SHA-256 hash of the previous entry's `self_hash` (chain link)
    pub prev_hash: String,
    /// SHA-256 hash of this entry's canonical JSON (computed after all other fields)
    pub self_hash: String,
}

impl AuditEntry {
    /// Compute the SHA-256 hash of this entry's canonical JSON representation
    /// (excluding `self_hash`, which is derived from the other fields).
    fn compute_hash(&self) -> String {
        use std::collections::BTreeMap;
        // Use a BTreeMap for canonical key ordering
        let mut map = BTreeMap::new();
        map.insert("seq", serde_json::Value::from(self.seq));
        map.insert(
            "timestamp",
            serde_json::Value::from(self.timestamp.to_rfc3339()),
        );
        map.insert(
            "session_id",
            serde_json::Value::from(self.session_id.to_string()),
        );
        map.insert(
            "agent_name",
            serde_json::Value::from(self.agent_name.as_str()),
        );
        if let Some(ref id) = self.identity_id {
            map.insert("identity_id", serde_json::Value::from(id.to_string()));
        }
        map.insert("action", serde_json::Value::from(self.action.as_str()));
        map.insert("resource", serde_json::Value::from(self.resource.as_str()));
        map.insert("decision", serde_json::to_value(&self.decision).unwrap());
        map.insert(
            "prev_hash",
            serde_json::Value::from(self.prev_hash.as_str()),
        );
        let canonical = serde_json::to_string(&map).unwrap_or_default();
        sha256_hex(&canonical)
    }
}

/// SHA-256 hash function (pure Rust, no external crypto dependency).
/// Uses a minimal SHA-256 implementation to keep the dependency tree lean.
fn sha256_hex(input: &str) -> String {
    let bytes = input.as_bytes();
    let hash = sha256_raw(bytes);
    hash.iter().map(|b| format!("{:02x}", b)).collect()
}

/// Minimal SHA-256 implementation.
fn sha256_raw(data: &[u8]) -> [u8; 32] {
    // SHA-256 constants
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
        0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
        0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
        0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
        0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
        0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
        0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
        0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
        0xc67178f2,
    ];

    let mut h: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
        0x5be0cd19,
    ];

    // Pre-processing: padding
    let bit_len = (data.len() as u64) * 8;
    let mut padded = data.to_vec();
    padded.push(0x80);
    while padded.len() % 64 != 56 {
        padded.push(0);
    }
    padded.extend_from_slice(&bit_len.to_be_bytes());

    // Process each 512-bit block
    for chunk in padded.chunks(64) {
        let mut w = [0u32; 64];
        for (i, word) in chunk.chunks(4).enumerate().take(16) {
            w[i] = u32::from_be_bytes([word[0], word[1], word[2], word[3]]);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1);
        }

        let mut a = h[0];
        let mut b = h[1];
        let mut c = h[2];
        let mut d = h[3];
        let mut e = h[4];
        let mut f = h[5];
        let mut g = h[6];
        let mut hh = h[7];

        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ ((!e) & g);
            let temp1 = hh
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(K[i])
                .wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let temp2 = s0.wrapping_add(maj);

            hh = g;
            g = f;
            f = e;
            e = d.wrapping_add(temp1);
            d = c;
            c = b;
            b = a;
            a = temp1.wrapping_add(temp2);
        }

        h[0] = h[0].wrapping_add(a);
        h[1] = h[1].wrapping_add(b);
        h[2] = h[2].wrapping_add(c);
        h[3] = h[3].wrapping_add(d);
        h[4] = h[4].wrapping_add(e);
        h[5] = h[5].wrapping_add(f);
        h[6] = h[6].wrapping_add(g);
        h[7] = h[7].wrapping_add(hh);
    }

    let mut result = [0u8; 32];
    for (i, val) in h.iter().enumerate() {
        result[i * 4..i * 4 + 4].copy_from_slice(&val.to_be_bytes());
    }
    result
}

/// The persistent audit logger. Appends entries to a JSONL file.
///
/// Each line is a complete JSON object (one `AuditEntry`). The file is
/// append-only — entries are never modified or deleted. The chain hash
/// makes tampering detectable.
///
/// Rotation is automatic: when the current log file exceeds the configured
/// size or age, it is renamed to `<path>.rotated.<N>` and a fresh file is
/// started. Old rotated files beyond `max_files` are pruned.
pub struct AuditLogger {
    /// Path to the JSONL audit log file
    path: PathBuf,
    /// The hash of the most recently written entry (for chaining)
    last_hash: String,
    /// The sequence number for the next entry
    next_seq: u64,
    /// Rotation configuration (defaults: 10 MB, 30 days, 5 files)
    rotation: RotationConfig,
}

/// Where the audit log lives by default.
/// On Linux/macOS: `~/.local/share/agenticbox/audit.log` or `~/.agenticbox/audit.log`
/// On Windows: `C:\\Users\\<user>\\AppData\\Local\\agenticbox\\audit.log`
pub fn default_audit_log_path() -> PathBuf {
    if let Some(data_dir) = dirs::data_local_dir() {
        data_dir.join("agenticbox").join("audit.log")
    } else if let Some(home) = dirs::home_dir() {
        home.join(".agenticbox").join("audit.log")
    } else {
        PathBuf::from("audit.log")
    }
}

impl AuditLogger {
    /// Open or create an audit log at the given path.
    /// If the file exists, reads the last entry to continue the chain.
    /// Uses default rotation configuration (10 MB, 30 days, 5 files).
    pub fn open(path: impl AsRef<Path>) -> Result<Self, AuditError> {
        Self::open_with_rotation(path, RotationConfig::default())
    }

    /// Open or create an audit log with custom rotation configuration.
    pub fn open_with_rotation(
        path: impl AsRef<Path>,
        rotation: RotationConfig,
    ) -> Result<Self, AuditError> {
        let path = path.as_ref().to_path_buf();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let (last_hash, next_seq) = if path.exists() {
            read_chain_tail(&path)?
        } else {
            ("genesis".to_string(), 1u64)
        };

        Ok(AuditLogger {
            path,
            last_hash,
            next_seq,
            rotation,
        })
    }

    /// Open the audit log at the default location.
    pub fn default_log() -> Result<Self, AuditError> {
        Self::open(default_audit_log_path())
    }

    /// Create a new audit entry and append it to the log file.
    ///
    /// This method is **concurrency-safe**: it acquires an exclusive file lock,
    /// re-reads the chain tail from disk (in case another process wrote entries
    /// since this logger was opened), computes the entry with the correct chain
    /// link, appends it, and releases the lock. Multiple `agenticbox run`
    /// processes can log concurrently without corrupting the chain.
    ///
    /// Before writing, checks if the current log file exceeds the configured
    /// rotation limits (size or age). If so, rotates the file before appending.
    ///
    /// Returns the created entry (with computed hash).
    pub fn log(
        &mut self,
        session_id: Uuid,
        agent_name: &str,
        action: &str,
        resource: &str,
        decision: Decision,
        identity_id: Option<Uuid>,
    ) -> Result<AuditEntry, AuditError> {
        use std::io::{Read, Seek, SeekFrom};

        // Check if rotation is needed before opening the file for writing.
        // We check size/age first, then acquire the lock for the actual rotation.
        if self.should_rotate() {
            self.rotate()?;
        }

        // Open the file in read-write mode (NOT append mode) and acquire an
        // exclusive lock. On Windows, append mode can interfere with seeking
        // back to read, so we use read-write mode and seek to end for writes.
        // The lock is held for the duration of this call, preventing other
        // processes from interleaving writes.
        let mut file = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(&self.path)?;
        file.lock_exclusive()?;

        // Re-read the chain tail from the *same* locked handle. We can't open
        // a separate read handle because the exclusive lock blocks other opens
        // on Windows. Seek to start, read all content, extract the last entry.
        file.seek(SeekFrom::Start(0))?;
        let mut content = String::new();
        file.read_to_string(&mut content)?;
        let last_entry = content
            .lines()
            .rfind(|l| !l.trim().is_empty())
            .and_then(|line| serde_json::from_str::<AuditEntry>(line).ok());

        let (prev_hash, seq) = match last_entry {
            Some(ref entry) => (entry.self_hash.clone(), entry.seq + 1),
            None => ("genesis".to_string(), 1u64),
        };

        let entry = AuditEntry {
            seq,
            timestamp: Utc::now(),
            session_id,
            agent_name: agent_name.to_string(),
            identity_id,
            action: action.to_string(),
            resource: resource.to_string(),
            decision,
            prev_hash: prev_hash.clone(),
            self_hash: String::new(), // placeholder, computed below
        };

        let self_hash = entry.compute_hash();
        let entry = AuditEntry { self_hash, ..entry };

        // Seek to end before writing (we're in read-write mode, not append mode).
        // This ensures we always append to the end of the file.
        file.seek(SeekFrom::End(0))?;
        let line = serde_json::to_string(&entry)?;
        writeln!(&file, "{}", line)?;
        file.flush()?;

        // Release the lock
        let _ = file.unlock();

        // Update in-memory chain state
        self.last_hash = entry.self_hash.clone();
        self.next_seq = seq + 1;

        tracing::info!(
            seq = entry.seq,
            action = %entry.action,
            resource = %entry.resource,
            decision = ?entry.decision,
            "Audit entry logged"
        );

        Ok(entry)
    }

    /// Convenience: log an Allow decision.
    pub fn log_allow(
        &mut self,
        session_id: Uuid,
        agent_name: &str,
        action: &str,
        resource: &str,
    ) -> Result<AuditEntry, AuditError> {
        self.log(
            session_id,
            agent_name,
            action,
            resource,
            Decision::Allow,
            None,
        )
    }

    /// Convenience: log an Allow decision with identity attribution.
    pub fn log_allow_with_identity(
        &mut self,
        session_id: Uuid,
        agent_name: &str,
        action: &str,
        resource: &str,
        identity_id: Option<Uuid>,
    ) -> Result<AuditEntry, AuditError> {
        self.log(
            session_id,
            agent_name,
            action,
            resource,
            Decision::Allow,
            identity_id,
        )
    }

    /// Convenience: log a Deny decision with a reason.
    pub fn log_deny(
        &mut self,
        session_id: Uuid,
        agent_name: &str,
        action: &str,
        resource: &str,
        reason: &str,
    ) -> Result<AuditEntry, AuditError> {
        self.log(
            session_id,
            agent_name,
            action,
            resource,
            Decision::Deny(reason.to_string()),
            None,
        )
    }

    /// Convenience: log a Deny decision with identity attribution.
    pub fn log_deny_with_identity(
        &mut self,
        session_id: Uuid,
        agent_name: &str,
        action: &str,
        resource: &str,
        reason: &str,
        identity_id: Option<Uuid>,
    ) -> Result<AuditEntry, AuditError> {
        self.log(
            session_id,
            agent_name,
            action,
            resource,
            Decision::Deny(reason.to_string()),
            identity_id,
        )
    }

    /// Read all entries from the log file.
    pub fn read_all(&self) -> Result<Vec<AuditEntry>, AuditError> {
        read_all_entries(&self.path)
    }

    /// Read the last N entries from the log file.
    pub fn read_recent(&self, n: usize) -> Result<Vec<AuditEntry>, AuditError> {
        let all = self.read_all()?;
        let start = all.len().saturating_sub(n);
        Ok(all[start..].to_vec())
    }

    /// Verify the integrity of the entire audit chain.
    /// Returns Ok(()) if the chain is unbroken, or an error describing
    /// where the chain was broken.
    pub fn verify_chain(&self) -> Result<(), AuditError> {
        let entries = self.read_all()?;
        let mut expected_prev = "genesis".to_string();

        for entry in &entries {
            if entry.prev_hash != expected_prev {
                return Err(AuditError::ChainBroken {
                    seq: entry.seq,
                    expected: expected_prev,
                    actual: entry.prev_hash.clone(),
                });
            }
            // Verify self_hash
            let computed = entry.compute_hash();
            if computed != entry.self_hash {
                return Err(AuditError::ChainBroken {
                    seq: entry.seq,
                    expected: entry.self_hash.clone(),
                    actual: computed,
                });
            }
            expected_prev = entry.self_hash.clone();
        }

        Ok(())
    }

    /// Count entries by decision type (allow/deny).
    pub fn count_by_decision(&self) -> Result<DecisionCounts, AuditError> {
        let entries = self.read_all()?;
        let mut counts = DecisionCounts::default();
        for entry in &entries {
            if entry.decision.is_allowed() {
                counts.allowed += 1;
            } else {
                counts.denied += 1;
            }
        }
        counts.total = entries.len() as u64;
        Ok(counts)
    }

    /// Filter entries by agent name.
    pub fn filter_by_agent(&self, agent_name: &str) -> Result<Vec<AuditEntry>, AuditError> {
        let entries = self.read_all()?;
        Ok(entries
            .into_iter()
            .filter(|e| e.agent_name == agent_name)
            .collect())
    }

    /// The path to the audit log file.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Check whether the current log file should be rotated based on
    /// configured size and age limits.
    fn should_rotate(&self) -> bool {
        if !self.path.exists() {
            return false;
        }
        // Check size
        if let Ok(metadata) = std::fs::metadata(&self.path) {
            if metadata.len() >= self.rotation.max_size_bytes {
                return true;
            }
        }
        // Check age (file modification time)
        if let Ok(metadata) = std::fs::metadata(&self.path) {
            if let Ok(modified) = metadata.modified() {
                if let Ok(elapsed) = modified.elapsed() {
                    let max_age_secs = self.rotation.max_age_days * 86400;
                    if elapsed.as_secs() >= max_age_secs {
                        return true;
                    }
                }
            }
        }
        false
    }

    /// Rotate the audit log file.
    ///
    /// Renames the current log file to `<path>.rotated.<N>` where N is the
    /// next available number, then starts a fresh file. Old rotated files
    /// beyond `max_files` are pruned (oldest first).
    ///
    /// After rotation, the logger's chain state is reset (genesis, seq=1)
    /// because the new file starts fresh.
    fn rotate(&mut self) -> Result<(), AuditError> {
        if !self.path.exists() {
            return Ok(());
        }

        // Find the next available rotation number
        let dir = self.path.parent().unwrap_or(Path::new("."));
        let stem = self.path.file_stem().unwrap_or_default().to_string_lossy();
        let ext = self
            .path
            .extension()
            .map(|e| format!(".{}", e.to_string_lossy()))
            .unwrap_or_default();

        let mut n = 1;
        let mut rotated_path = dir.join(format!("{}.rotated.{}{}", stem, n, ext));
        while rotated_path.exists() {
            n += 1;
            rotated_path = dir.join(format!("{}.rotated.{}{}", stem, n, ext));
        }

        // Rename current log to rotated name
        std::fs::rename(&self.path, &rotated_path)?;

        tracing::info!(
            from = %self.path.display(),
            to = %rotated_path.display(),
            "Audit log rotated"
        );

        // Prune old rotated files beyond max_files
        self.prune_rotated_files(dir, &stem, &ext)?;

        // Reset chain state for the new file
        self.last_hash = "genesis".to_string();
        self.next_seq = 1;

        Ok(())
    }

    /// Remove old rotated files beyond the configured `max_files` limit.
    /// Keeps the most recent `max_files` rotated files, removes the rest.
    fn prune_rotated_files(&self, dir: &Path, stem: &str, ext: &str) -> Result<(), AuditError> {
        use std::fs;

        let pattern = format!("{}.rotated.", stem);
        let mut rotated: Vec<(std::time::SystemTime, PathBuf)> = Vec::new();

        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with(&pattern) && name.ends_with(ext) {
                if let Ok(metadata) = entry.metadata() {
                    if let Ok(modified) = metadata.modified() {
                        rotated.push((modified, entry.path()));
                    }
                }
            }
        }

        // Sort by modification time (oldest first)
        rotated.sort_by_key(|(time, _)| *time);

        // Remove oldest files beyond max_files
        while rotated.len() > self.rotation.max_files {
            let (_, path) = rotated.remove(0);
            let _ = fs::remove_file(&path);
            tracing::info!(path = %path.display(), "Pruned old rotated audit log");
        }

        Ok(())
    }

    /// Manually trigger a rotation of the audit log.
    ///
    /// This is useful for CLI commands like `agenticbox audit --rotate`.
    /// Returns the number of entries in the rotated file.
    pub fn rotate_now(&mut self) -> Result<u64, AuditError> {
        let count = self.read_all()?.len() as u64;
        self.rotate()?;
        Ok(count)
    }

    /// Get the current rotation configuration.
    pub fn rotation_config(&self) -> &RotationConfig {
        &self.rotation
    }

    /// Set a new rotation configuration.
    pub fn set_rotation_config(&mut self, config: RotationConfig) {
        self.rotation = config;
    }
}

/// Summary counts of allow/deny decisions.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DecisionCounts {
    pub total: u64,
    pub allowed: u64,
    pub denied: u64,
}

/// Configuration for audit log rotation.
///
/// Controls when the audit log is rotated (by size or age) and how many
/// rotated files are retained. Rotation is transparent to callers — the
/// `AuditLogger` handles it automatically during `log()` if the current
/// file exceeds the configured limits.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RotationConfig {
    /// Maximum size in bytes before rotation triggers (default: 10 MB).
    pub max_size_bytes: u64,
    /// Maximum age in days before rotation triggers (default: 30).
    pub max_age_days: u64,
    /// Maximum number of rotated files to keep (oldest are pruned).
    pub max_files: usize,
}

impl Default for RotationConfig {
    fn default() -> Self {
        RotationConfig {
            max_size_bytes: DEFAULT_MAX_LOG_SIZE,
            max_age_days: DEFAULT_MAX_LOG_AGE_DAYS,
            max_files: DEFAULT_MAX_LOG_FILES,
        }
    }
}

// ─── Internal helpers ──────────────────────────────────────────

/// Read all entries from a JSONL file. Returns empty vec if file doesn't exist.
fn read_all_entries(path: &Path) -> Result<Vec<AuditEntry>, AuditError> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let mut entries = Vec::new();

    for line in reader.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let entry: AuditEntry = serde_json::from_str(&line)?;
        entries.push(entry);
    }

    Ok(entries)
}

/// Read the last entry's hash and the next sequence number from an existing log.
fn read_chain_tail(path: &Path) -> Result<(String, u64), AuditError> {
    let entries = read_all_entries(path)?;
    if entries.is_empty() {
        return Ok(("genesis".to_string(), 1));
    }
    let last = entries.last().unwrap();
    Ok((last.self_hash.clone(), last.seq + 1))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_log_path() -> PathBuf {
        let dir = std::env::temp_dir().join("agenticbox-audit-tests");
        let _ = std::fs::create_dir_all(&dir);
        dir.join(format!("audit-{}.log", Uuid::new_v4()))
    }

    fn cleanup(path: &Path) {
        let _ = std::fs::remove_file(path);
    }

    // ── Basic logging ──────────────────────────────────────

    #[test]
    fn log_creates_file_and_writes_entry() {
        let path = temp_log_path();
        let mut logger = AuditLogger::open(&path).unwrap();

        let session_id = Uuid::new_v4();
        let entry = logger
            .log_allow(session_id, "test-agent", "fs:read", "/workspace/deploy.sh")
            .unwrap();

        assert_eq!(entry.seq, 1);
        assert_eq!(entry.agent_name, "test-agent");
        assert_eq!(entry.action, "fs:read");
        assert_eq!(entry.resource, "/workspace/deploy.sh");
        assert!(entry.decision.is_allowed());
        assert_eq!(entry.prev_hash, "genesis");
        assert!(!entry.self_hash.is_empty());
        assert!(path.exists());

        cleanup(&path);
    }

    #[test]
    fn log_deny_records_reason() {
        let path = temp_log_path();
        let mut logger = AuditLogger::open(&path).unwrap();

        let entry = logger
            .log_deny(
                Uuid::new_v4(),
                "test-agent",
                "network:outbound",
                "https://evil.com",
                "Domain not in allowlist",
            )
            .unwrap();

        assert!(entry.decision.is_denied());
        assert_eq!(entry.decision.reason(), "Domain not in allowlist");

        cleanup(&path);
    }

    // ── Chain integrity ────────────────────────────────────

    #[test]
    fn chain_links_are_correct() {
        let path = temp_log_path();
        let mut logger = AuditLogger::open(&path).unwrap();
        let sid = Uuid::new_v4();

        let e1 = logger.log_allow(sid, "agent", "fs:read", "/a").unwrap();
        let e2 = logger
            .log_deny(sid, "agent", "fs:write", "/b", "readonly")
            .unwrap();
        let e3 = logger
            .log_allow(sid, "agent", "network:outbound", "api.github.com")
            .unwrap();

        assert_eq!(e1.prev_hash, "genesis");
        assert_eq!(e2.prev_hash, e1.self_hash);
        assert_eq!(e3.prev_hash, e2.self_hash);
        assert_eq!(e1.seq, 1);
        assert_eq!(e2.seq, 2);
        assert_eq!(e3.seq, 3);

        cleanup(&path);
    }

    #[test]
    fn verify_chain_passes_on_intact_log() {
        let path = temp_log_path();
        let mut logger = AuditLogger::open(&path).unwrap();
        let sid = Uuid::new_v4();

        logger.log_allow(sid, "agent", "fs:read", "/a").unwrap();
        logger
            .log_deny(sid, "agent", "fs:write", "/b", "denied")
            .unwrap();
        logger
            .log_allow(sid, "agent", "network:outbound", "api.github.com")
            .unwrap();

        assert!(logger.verify_chain().is_ok());

        cleanup(&path);
    }

    #[test]
    fn verify_chain_detects_tampered_entry() {
        let path = temp_log_path();
        let mut logger = AuditLogger::open(&path).unwrap();
        let sid = Uuid::new_v4();

        logger.log_allow(sid, "agent", "fs:read", "/a").unwrap();
        logger.log_allow(sid, "agent", "fs:read", "/b").unwrap();

        // Tamper: rewrite the file with a modified entry
        let entries = logger.read_all().unwrap();
        let mut tampered = entries.clone();
        tampered[0].resource = "/etc/shadow".to_string(); // changed!
        let tampered_json: Vec<String> = tampered
            .iter()
            .map(|e| serde_json::to_string(e).unwrap())
            .collect();
        std::fs::write(&path, tampered_json.join("\n") + "\n").unwrap();

        let result = logger.verify_chain();
        assert!(result.is_err());
        match result {
            Err(AuditError::ChainBroken { seq, .. }) => assert_eq!(seq, 1),
            _ => panic!("expected ChainBroken error"),
        }

        cleanup(&path);
    }

    // ── Reopening continues the chain ──────────────────────

    #[test]
    fn reopen_continues_chain() {
        let path = temp_log_path();

        // First session: write 2 entries
        let mut logger1 = AuditLogger::open(&path).unwrap();
        let sid = Uuid::new_v4();
        let _e1 = logger1.log_allow(sid, "agent", "fs:read", "/a").unwrap();
        let e2 = logger1
            .log_deny(sid, "agent", "fs:write", "/b", "no")
            .unwrap();
        assert_eq!(e2.seq, 2);
        drop(logger1);

        // Second session: reopen and write 1 more
        let mut logger2 = AuditLogger::open(&path).unwrap();
        let e3 = logger2.log_allow(sid, "agent", "fs:read", "/c").unwrap();

        assert_eq!(e3.seq, 3);
        assert_eq!(e3.prev_hash, e2.self_hash);

        // Chain should still verify
        assert!(logger2.verify_chain().is_ok());

        cleanup(&path);
    }

    // ── Reading and filtering ──────────────────────────────

    #[test]
    fn read_all_returns_all_entries() {
        let path = temp_log_path();
        let mut logger = AuditLogger::open(&path).unwrap();
        let sid = Uuid::new_v4();

        logger.log_allow(sid, "a1", "fs:read", "/1").unwrap();
        logger.log_deny(sid, "a2", "fs:write", "/2", "no").unwrap();
        logger
            .log_allow(sid, "a1", "network:outbound", "api.x.com")
            .unwrap();

        let all = logger.read_all().unwrap();
        assert_eq!(all.len(), 3);

        cleanup(&path);
    }

    #[test]
    fn read_recent_returns_last_n() {
        let path = temp_log_path();
        let mut logger = AuditLogger::open(&path).unwrap();
        let sid = Uuid::new_v4();

        for i in 0..10 {
            logger
                .log_allow(sid, "agent", "fs:read", &format!("/file{}", i))
                .unwrap();
        }

        let recent = logger.read_recent(3).unwrap();
        assert_eq!(recent.len(), 3);
        assert_eq!(recent[0].resource, "/file7");
        assert_eq!(recent[2].resource, "/file9");

        cleanup(&path);
    }

    #[test]
    fn filter_by_agent() {
        let path = temp_log_path();
        let mut logger = AuditLogger::open(&path).unwrap();
        let sid = Uuid::new_v4();

        logger.log_allow(sid, "alpha", "fs:read", "/a").unwrap();
        logger.log_allow(sid, "beta", "fs:read", "/b").unwrap();
        logger.log_allow(sid, "alpha", "fs:read", "/c").unwrap();

        let alpha = logger.filter_by_agent("alpha").unwrap();
        assert_eq!(alpha.len(), 2);
        assert!(alpha.iter().all(|e| e.agent_name == "alpha"));

        cleanup(&path);
    }

    // ── Decision counting ──────────────────────────────────

    #[test]
    fn count_by_decision() {
        let path = temp_log_path();
        let mut logger = AuditLogger::open(&path).unwrap();
        let sid = Uuid::new_v4();

        logger.log_allow(sid, "agent", "fs:read", "/a").unwrap();
        logger.log_allow(sid, "agent", "fs:read", "/b").unwrap();
        logger
            .log_deny(sid, "agent", "fs:write", "/c", "readonly")
            .unwrap();
        logger
            .log_deny(
                sid,
                "agent",
                "network:outbound",
                "evil.com",
                "not in allowlist",
            )
            .unwrap();
        logger
            .log_allow(sid, "agent", "terminal:exec", "ls")
            .unwrap();

        let counts = logger.count_by_decision().unwrap();
        assert_eq!(counts.total, 5);
        assert_eq!(counts.allowed, 3);
        assert_eq!(counts.denied, 2);

        cleanup(&path);
    }

    // ── Empty log ──────────────────────────────────────────

    #[test]
    fn empty_log_verifies_ok() {
        let path = temp_log_path();
        let logger = AuditLogger::open(&path).unwrap();
        assert!(logger.verify_chain().is_ok());
        assert_eq!(logger.read_all().unwrap().len(), 0);
        let counts = logger.count_by_decision().unwrap();
        assert_eq!(counts.total, 0);
        cleanup(&path);
    }

    // ── SHA-256 correctness ────────────────────────────────

    #[test]
    fn sha256_empty_string() {
        let hash = sha256_hex("");
        assert_eq!(
            hash,
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    #[test]
    fn sha256_known_vectors() {
        assert_eq!(
            sha256_hex("abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        assert_eq!(
            sha256_hex("hello"),
            "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"
        );
    }

    // ── Decision helpers ───────────────────────────────────

    #[test]
    fn decision_helpers() {
        assert!(Decision::Allow.is_allowed());
        assert!(!Decision::Allow.is_denied());
        assert_eq!(Decision::Allow.reason(), "within permissions");

        let deny = Decision::Deny("blocked".into());
        assert!(!deny.is_allowed());
        assert!(deny.is_denied());
        assert_eq!(deny.reason(), "blocked");
    }

    // ── Serde roundtrip ────────────────────────────────────

    #[test]
    fn audit_entry_serde_roundtrip() {
        let entry = AuditEntry {
            seq: 42,
            timestamp: Utc::now(),
            session_id: Uuid::new_v4(),
            agent_name: "test".into(),
            identity_id: None,
            action: "fs:read".into(),
            resource: "/x".into(),
            decision: Decision::Deny("test reason".into()),
            prev_hash: "abc123".into(),
            self_hash: "def456".into(),
        };
        let json = serde_json::to_string(&entry).unwrap();
        let parsed: AuditEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.seq, 42);
        assert_eq!(parsed.agent_name, "test");
        assert!(parsed.decision.is_denied());
    }

    // ── Default path is valid ──────────────────────────────

    #[test]
    fn default_audit_log_path_is_not_empty() {
        let path = default_audit_log_path();
        assert!(!path.as_os_str().is_empty());
        assert!(path.to_string_lossy().contains("audit"));
    }

    // ── Concurrency: file locking prevents chain corruption ──

    #[test]
    fn concurrent_loggers_maintain_chain_integrity() {
        use std::sync::Arc;
        use std::thread;

        let path = temp_log_path();
        cleanup(&path); // start clean

        // Each thread opens its own AuditLogger pointing at the same file,
        // then writes a batch of entries. Without file locking this would
        // produce interleaved writes and a broken chain. With locking, the
        // chain should remain intact.
        let path = Arc::new(path);
        let mut handles = Vec::new();

        for thread_id in 0..4 {
            let p = Arc::clone(&path);
            handles.push(thread::spawn(move || {
                let mut logger = AuditLogger::open(&*p).unwrap();
                let session = Uuid::new_v4();
                for i in 0..25 {
                    logger
                        .log_allow(
                            session,
                            &format!("agent-{}", thread_id),
                            "fs:read",
                            &format!("/data/{}.txt", i),
                        )
                        .unwrap();
                }
            }));
        }

        for h in handles {
            h.join().unwrap();
        }

        // Verify chain integrity — all 100 entries should form a valid chain.
        let logger = AuditLogger::open(&*path).unwrap();
        let entries = logger.read_all().unwrap();
        assert_eq!(entries.len(), 100, "expected 100 entries (4 threads × 25)");

        // Chain must verify
        logger.verify_chain().unwrap_or_else(|e| {
            panic!("chain broken after concurrent writes: {}", e);
        });

        // Sequence numbers must be 1..=100 with no gaps or duplicates
        let seqs: Vec<u64> = entries.iter().map(|e| e.seq).collect();
        let expected: Vec<u64> = (1..=100).collect();
        assert_eq!(
            seqs, expected,
            "sequence numbers must be contiguous 1..=100"
        );

        cleanup(&path);
    }

    #[test]
    fn stale_logger_picks_up_new_tail_on_next_log() {
        // Simulate the scenario: logger A opens, logger B opens and writes,
        // then logger A writes — A should pick up B's tail from disk.
        let path = temp_log_path();
        cleanup(&path);

        let mut logger_a = AuditLogger::open(&path).unwrap();
        let mut logger_b = AuditLogger::open(&path).unwrap();

        let session = Uuid::new_v4();

        // A writes first
        logger_a
            .log_allow(session, "agent-a", "fs:read", "/a.txt")
            .unwrap();

        // B writes — B's in-memory tail was "genesis", but the lock + re-read
        // should pick up A's entry as the real tail.
        logger_b
            .log_allow(session, "agent-b", "fs:read", "/b.txt")
            .unwrap();

        // A writes again — A's in-memory tail is now stale (B wrote after).
        // The re-read should pick up B's entry as the real tail.
        logger_a
            .log_allow(session, "agent-a", "fs:read", "/a2.txt")
            .unwrap();

        // Verify: 3 entries, chain intact
        let logger = AuditLogger::open(&path).unwrap();
        let entries = logger.read_all().unwrap();
        assert_eq!(entries.len(), 3);
        logger.verify_chain().unwrap_or_else(|e| {
            panic!("chain broken after interleaved writes: {}", e);
        });

        // Check ordering: a, b, a2
        assert_eq!(entries[0].agent_name, "agent-a");
        assert_eq!(entries[1].agent_name, "agent-b");
        assert_eq!(entries[2].agent_name, "agent-a");

        cleanup(&path);
    }

    // ── JSON serialization (for --json output) ──────────────

    #[test]
    fn decision_counts_json_serialization() {
        let counts = DecisionCounts {
            total: 10,
            allowed: 7,
            denied: 3,
        };
        let json = serde_json::to_string_pretty(&counts).unwrap();
        assert!(json.contains("\"total\": 10"));
        assert!(json.contains("\"allowed\": 7"));
        assert!(json.contains("\"denied\": 3"));

        // Round-trip: parse back
        let parsed: DecisionCounts = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.total, 10);
        assert_eq!(parsed.allowed, 7);
        assert_eq!(parsed.denied, 3);
    }

    #[test]
    fn audit_entry_json_roundtrip() {
        // Create an entry with all fields populated
        let entry = AuditEntry {
            seq: 1,
            timestamp: Utc::now(),
            session_id: Uuid::new_v4(),
            agent_name: "demo-agent".into(),
            identity_id: None,
            action: "network:outbound".into(),
            resource: "https://api.github.com".into(),
            decision: Decision::Deny("Domain not in allowlist".into()),
            prev_hash: "genesis".into(),
            self_hash: "abc123".into(),
        };

        // Serialize to JSON
        let json = serde_json::to_string_pretty(&entry).unwrap();

        // Verify JSON structure
        assert!(json.contains("\"seq\": 1"));
        assert!(json.contains("\"agent_name\": \"demo-agent\""));
        assert!(json.contains("\"action\": \"network:outbound\""));
        assert!(json.contains("\"resource\": \"https://api.github.com\""));
        assert!(json.contains("\"prev_hash\": \"genesis\""));
        assert!(json.contains("\"self_hash\": \"abc123\""));
        assert!(json.contains("\"deny\": \"Domain not in allowlist\""));

        // Round-trip: parse back
        let parsed: AuditEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.seq, 1);
        assert_eq!(parsed.agent_name, "demo-agent");
        assert!(parsed.decision.is_denied());
        assert_eq!(parsed.decision.reason(), "Domain not in allowlist");
    }

    #[test]
    fn audit_entry_json_allow_decision() {
        // Verify Allow decision serialization
        let entry = AuditEntry {
            seq: 2,
            timestamp: Utc::now(),
            session_id: Uuid::new_v4(),
            agent_name: "test-agent".into(),
            identity_id: None,
            action: "fs:read".into(),
            resource: "/workspace/file.txt".into(),
            decision: Decision::Allow,
            prev_hash: "prevhash".into(),
            self_hash: "selfhash".into(),
        };

        let json = serde_json::to_string_pretty(&entry).unwrap();
        assert!(json.contains("\"decision\": \"allow\""));
        assert!(json.contains("\"seq\": 2"));

        let parsed: AuditEntry = serde_json::from_str(&json).unwrap();
        assert!(parsed.decision.is_allowed());
    }

    #[test]
    fn audit_entries_array_json() {
        // Verify that a Vec<AuditEntry> serializes as a valid JSON array
        let mut entries = Vec::new();
        for i in 0..3 {
            entries.push(AuditEntry {
                seq: i + 1,
                timestamp: Utc::now(),
                session_id: Uuid::new_v4(),
                agent_name: format!("agent-{}", i),
                identity_id: None,
                action: "fs:read".into(),
                resource: format!("/file/{}", i),
                decision: Decision::Allow,
                prev_hash: if i == 0 {
                    "genesis".into()
                } else {
                    "hash".into()
                },
                self_hash: "hash".into(),
            });
        }

        let json = serde_json::to_string_pretty(&entries).unwrap();
        // Should be a JSON array
        assert!(json.starts_with('['));
        assert!(json.ends_with(']'));
        // Should contain 3 entries
        assert_eq!(json.matches("\"seq\"").count(), 3);

        // Round-trip
        let parsed: Vec<AuditEntry> = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.len(), 3);
        assert_eq!(parsed[0].agent_name, "agent-0");
        assert_eq!(parsed[2].agent_name, "agent-2");
    }

    // ── Rotation tests ──────────────────────────────────────

    #[test]
    fn rotation_config_defaults() {
        let config = RotationConfig::default();
        assert_eq!(config.max_size_bytes, DEFAULT_MAX_LOG_SIZE);
        assert_eq!(config.max_age_days, DEFAULT_MAX_LOG_AGE_DAYS);
        assert_eq!(config.max_files, DEFAULT_MAX_LOG_FILES);
    }

    #[test]
    fn rotation_creates_rotated_file() {
        let path = temp_log_path();
        cleanup(&path);

        // Create a logger with tiny max_size to force rotation
        let config = RotationConfig {
            max_size_bytes: 100, // 100 bytes — tiny
            max_age_days: 365,
            max_files: 3,
        };
        let mut logger = AuditLogger::open_with_rotation(&path, config).unwrap();
        let sid = Uuid::new_v4();

        // Write enough entries to exceed 100 bytes
        for i in 0..20 {
            logger
                .log_allow(sid, "agent", "fs:read", &format!("/data/file_{}.txt", i))
                .unwrap();
        }

        // The original file should exist (new file after rotation)
        assert!(path.exists(), "current log file should exist");

        // There should be at least one rotated file
        let dir = path.parent().unwrap();
        let stem = path.file_stem().unwrap().to_string_lossy();
        let ext = path
            .extension()
            .map(|e| format!(".{}", e.to_string_lossy()))
            .unwrap_or_default();
        let pattern = format!("{}.rotated.", stem);
        let rotated_count = std::fs::read_dir(dir)
            .unwrap()
            .filter_map(Result::ok)
            .filter(|e| {
                e.file_name().to_string_lossy().starts_with(&pattern)
                    && e.file_name().to_string_lossy().ends_with(&ext)
            })
            .count();
        assert!(
            rotated_count >= 1,
            "expected at least 1 rotated file, got {}",
            rotated_count
        );

        // Clean up rotated files too
        for entry in std::fs::read_dir(dir).unwrap().flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with(&pattern) {
                let _ = std::fs::remove_file(entry.path());
            }
        }
        cleanup(&path);
    }

    #[test]
    fn rotation_resets_chain() {
        let path = temp_log_path();
        cleanup(&path);

        // Create a logger with tiny max_size
        let config = RotationConfig {
            max_size_bytes: 100,
            max_age_days: 365,
            max_files: 3,
        };
        let mut logger = AuditLogger::open_with_rotation(&path, config).unwrap();
        let sid = Uuid::new_v4();

        // Write enough to trigger rotation
        for i in 0..20 {
            logger
                .log_allow(sid, "agent", "fs:read", &format!("/data/file_{}.txt", i))
                .unwrap();
        }

        // After rotation, the new file should start fresh with seq=1
        // Write one more entry and verify it's seq=1 in the new file
        let entry = logger
            .log_allow(sid, "agent", "fs:read", "/new_file.txt")
            .unwrap();
        assert_eq!(entry.seq, 1, "new file should start at seq 1");
        assert_eq!(
            entry.prev_hash, "genesis",
            "new file should start with genesis"
        );

        // Clean up rotated files
        let dir = path.parent().unwrap();
        let stem = path.file_stem().unwrap().to_string_lossy();
        let _ext = path
            .extension()
            .map(|e| format!(".{}", e.to_string_lossy()))
            .unwrap_or_default();
        let pattern = format!("{}.rotated.", stem);
        for entry in std::fs::read_dir(dir).unwrap().flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with(&pattern) {
                let _ = std::fs::remove_file(entry.path());
            }
        }
        cleanup(&path);
    }

    #[test]
    fn rotate_now_returns_entry_count() {
        let path = temp_log_path();
        let mut logger = AuditLogger::open(&path).unwrap();
        let sid = Uuid::new_v4();

        logger.log_allow(sid, "agent", "fs:read", "/a").unwrap();
        logger.log_allow(sid, "agent", "fs:read", "/b").unwrap();
        logger.log_allow(sid, "agent", "fs:read", "/c").unwrap();

        let count = logger.rotate_now().unwrap();
        assert_eq!(count, 3, "rotate_now should return 3 entries");

        // After rotation, new file should be empty
        assert_eq!(logger.read_all().unwrap().len(), 0);

        // Clean up rotated file
        let dir = path.parent().unwrap();
        let stem = path.file_stem().unwrap().to_string_lossy();
        let _ext = path
            .extension()
            .map(|e| format!(".{}", e.to_string_lossy()))
            .unwrap_or_default();
        let pattern = format!("{}.rotated.", stem);
        for entry in std::fs::read_dir(dir).unwrap().flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with(&pattern) {
                let _ = std::fs::remove_file(entry.path());
            }
        }
        cleanup(&path);
    }

    #[test]
    fn rotation_prunes_old_files() {
        let path = temp_log_path();
        cleanup(&path);

        // Create a logger with tiny max_size and max_files=2
        let config = RotationConfig {
            max_size_bytes: 50, // very tiny
            max_age_days: 365,
            max_files: 2,
        };
        let mut logger = AuditLogger::open_with_rotation(&path, config).unwrap();
        let sid = Uuid::new_v4();

        // Write enough to trigger multiple rotations
        for i in 0..50 {
            logger
                .log_allow(sid, "agent", "fs:read", &format!("/data/file_{}.txt", i))
                .unwrap();
        }

        // Check that at most max_files rotated files exist
        let dir = path.parent().unwrap();
        let stem = path.file_stem().unwrap().to_string_lossy();
        let ext = path
            .extension()
            .map(|e| format!(".{}", e.to_string_lossy()))
            .unwrap_or_default();
        let pattern = format!("{}.rotated.", stem);
        let rotated_count = std::fs::read_dir(dir)
            .unwrap()
            .filter_map(Result::ok)
            .filter(|e| {
                e.file_name().to_string_lossy().starts_with(&pattern)
                    && e.file_name().to_string_lossy().ends_with(&ext)
            })
            .count();
        assert!(
            rotated_count <= 2,
            "expected at most 2 rotated files, got {}",
            rotated_count
        );

        // Clean up rotated files
        for entry in std::fs::read_dir(dir).unwrap().flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with(&pattern) {
                let _ = std::fs::remove_file(entry.path());
            }
        }
        cleanup(&path);
    }

    #[test]
    fn rotation_config_serde_roundtrip() {
        let config = RotationConfig {
            max_size_bytes: 5 * 1024 * 1024,
            max_age_days: 7,
            max_files: 10,
        };
        let json = serde_json::to_string(&config).unwrap();
        let parsed: RotationConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.max_size_bytes, 5 * 1024 * 1024);
        assert_eq!(parsed.max_age_days, 7);
        assert_eq!(parsed.max_files, 10);
    }
}
