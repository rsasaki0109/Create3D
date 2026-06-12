use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::Path;

use c3d_scene_ops::Transaction;
use serde::{Deserialize, Serialize};

use crate::ClientId;

/// Append-only operation log entry broadcast through sync.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OperationLogEntry {
    /// Monotonic server-assigned sequence number.
    pub sequence: u64,
    /// Branch name, typically `main`.
    pub branch: String,
    /// Author client identifier.
    pub author: ClientId,
    /// Author display name.
    pub author_name: String,
    /// Applied scene transaction payload.
    pub transaction: Transaction,
    /// Timestamp in milliseconds since UNIX epoch.
    pub timestamp_ms: u64,
}

/// In-memory operation log with optional JSONL persistence.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct OperationLog {
    entries: Vec<OperationLogEntry>,
}

impl OperationLog {
    /// Create an empty operation log.
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// Load an operation log from a JSONL file.
    pub fn load(path: impl AsRef<Path>) -> std::io::Result<Self> {
        let path = path.as_ref();
        if !path.is_file() {
            return Ok(Self::new());
        }
        let file = fs::File::open(path)?;
        let reader = BufReader::new(file);
        let mut entries = Vec::new();
        for line in reader.lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }
            let entry: OperationLogEntry = serde_json::from_str(&line)
                .map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidData, err))?;
            entries.push(entry);
        }
        Ok(Self { entries })
    }

    /// Persist the full log to a JSONL file.
    pub fn save(&self, path: impl AsRef<Path>) -> std::io::Result<()> {
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut file = fs::File::create(path)?;
        for entry in &self.entries {
            let line = serde_json::to_string(entry)
                .map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidData, err))?;
            writeln!(file, "{line}")?;
        }
        Ok(())
    }

    /// Append an entry to the log and optionally persist it.
    pub fn append(&mut self, entry: OperationLogEntry, path: Option<&Path>) -> std::io::Result<()> {
        self.entries.push(entry.clone());
        if let Some(path) = path {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
            let line = serde_json::to_string(&entry)
                .map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidData, err))?;
            let mut file = OpenOptions::new().create(true).append(true).open(path)?;
            writeln!(file, "{line}")?;
        }
        Ok(())
    }

    /// Returns the current head sequence number.
    pub fn head_sequence(&self) -> u64 {
        self.entries.last().map(|entry| entry.sequence).unwrap_or(0)
    }

    /// Iterate log entries in order.
    pub fn entries(&self) -> &[OperationLogEntry] {
        &self.entries
    }

    /// Returns entries with sequence greater than `after`.
    pub fn entries_after(&self, after: u64) -> impl Iterator<Item = &OperationLogEntry> {
        self.entries
            .iter()
            .filter(move |entry| entry.sequence > after)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use c3d_scene_ops::{SceneOperation, Transaction};
    use c3d_scene_schema::Name;

    #[test]
    fn log_round_trips_through_jsonl() {
        let temp = tempfile::tempdir().expect("tempdir");
        let path = temp.path().join("operation_log.jsonl");
        let mut log = OperationLog::new();
        log.append(
            OperationLogEntry {
                sequence: 1,
                branch: "main".into(),
                author: ClientId::new(),
                author_name: "Alice".into(),
                transaction: Transaction::new(
                    c3d_core::TransactionId::new(),
                    vec![SceneOperation::SetName {
                        entity_id: c3d_core::EntityId::new(),
                        name: Name::new("Synced"),
                    }],
                ),
                timestamp_ms: 1,
            },
            Some(&path),
        )
        .expect("append");

        let loaded = OperationLog::load(&path).expect("load");
        assert_eq!(loaded.entries().len(), 1);
        assert_eq!(loaded.head_sequence(), 1);
    }
}
