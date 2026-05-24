use std::collections::HashMap;
use std::io::Write;
use std::path::Path;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReleaseStatus {
    Staged,
    Published,
    Cancelled,
    Retired,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReleaseEntry {
    pub id: String,
    pub version: String,
    pub status: ReleaseStatus,
    pub created_at: String,
}

#[derive(Debug, Clone)]
pub struct ReleaseRecord {
    pub id: String,
    pub version: String,
    pub status: ReleaseStatus,
    pub created_at: String,
    pub updated_at: String,
}

impl ReleaseRecord {
    pub fn new_staged(version: &str) -> Self {
        let now = timestamp();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            version: version.to_string(),
            status: ReleaseStatus::Staged,
            created_at: now.clone(),
            updated_at: now,
        }
    }
}

fn timestamp() -> String {
    let d = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    format!("{}", d.as_secs())
}

#[derive(Debug)]
pub enum TransitionError {
    NotStaged(String),
    NotPublished(String),
}

impl std::fmt::Display for TransitionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotStaged(v) => write!(f, "版本 {} 不处于 Staged 状态", v),
            Self::NotPublished(v) => write!(f, "版本 {} 不处于 Published 状态", v),
        }
    }
}

impl std::error::Error for TransitionError {}

pub trait Storage {
    fn save(&mut self, record: &ReleaseRecord) -> Result<(), Box<dyn std::error::Error>>;
    fn load(&self, version: &str) -> Option<ReleaseRecord>;
    fn list(&self) -> Vec<ReleaseRecord>;
}

fn replay_events(path: &Path) -> Vec<ReleaseRecord> {
    if !path.exists() {
        return Vec::new();
    }
    let mut records: HashMap<String, ReleaseRecord> = HashMap::new();
    if let Ok(content) = std::fs::read_to_string(path) {
        for line in content.lines() {
            if let Ok(entry) = serde_json::from_str::<ReleaseEntry>(line) {
                let first_created = records
                    .get(&entry.version)
                    .map(|r| r.created_at.clone())
                    .unwrap_or_else(|| entry.created_at.clone());
                records.insert(
                    entry.version.clone(),
                    ReleaseRecord {
                        id: entry.id,
                        version: entry.version,
                        status: entry.status,
                        created_at: first_created,
                        updated_at: entry.created_at,
                    },
                );
            }
        }
    }
    records.into_values().collect()
}

pub struct FileStorage {
    events_path: std::path::PathBuf,
    records: Vec<ReleaseRecord>,
}

impl FileStorage {
    pub fn new(base_path: &Path) -> Self {
        let events_path = base_path.join(".quanttide/devops/release-journal.jsonl");
        let records = replay_events(&events_path);
        Self {
            events_path,
            records,
        }
    }
}

impl Storage for FileStorage {
    fn save(&mut self, record: &ReleaseRecord) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(existing) = self
            .records
            .iter_mut()
            .find(|r| r.version == record.version)
        {
            *existing = record.clone();
        } else {
            self.records.push(record.clone());
        }

        let entry = ReleaseEntry {
            id: record.id.clone(),
            version: record.version.clone(),
            status: record.status.clone(),
            created_at: record.updated_at.clone(),
        };

        if let Some(parent) = self.events_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string(&entry)?;
        let mut f = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.events_path)?;
        writeln!(f, "{}", json)?;

        Ok(())
    }

    fn load(&self, version: &str) -> Option<ReleaseRecord> {
        self.records.iter().find(|r| r.version == version).cloned()
    }

    fn list(&self) -> Vec<ReleaseRecord> {
        self.records.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_record(version: &str, status: ReleaseStatus) -> ReleaseRecord {
        let now = timestamp();
        ReleaseRecord {
            id: uuid::Uuid::new_v4().to_string(),
            version: version.to_string(),
            status,
            created_at: now.clone(),
            updated_at: now,
        }
    }

    #[test]
    fn test_status_debug() {
        assert_eq!(format!("{:?}", ReleaseStatus::Staged), "Staged");
        assert_eq!(format!("{:?}", ReleaseStatus::Published), "Published");
        assert_eq!(format!("{:?}", ReleaseStatus::Cancelled), "Cancelled");
        assert_eq!(format!("{:?}", ReleaseStatus::Retired), "Retired");
    }

    #[test]
    fn test_status_clone_eq() {
        assert_eq!(ReleaseStatus::Staged, ReleaseStatus::Staged);
    }

    #[test]
    fn test_record_new_staged() {
        let r = ReleaseRecord::new_staged("v1.0.0");
        assert_eq!(r.version, "v1.0.0");
        assert_eq!(r.status, ReleaseStatus::Staged);
        assert!(!r.id.is_empty());
        assert_eq!(r.created_at, r.updated_at);
    }

    #[test]
    fn test_storage_save_and_load() {
        let dir = tempfile::tempdir().unwrap();
        let mut s = FileStorage::new(dir.path());
        let r = make_record("v1.0.0", ReleaseStatus::Staged);
        s.save(&r).unwrap();
        assert!(s.load("v1.0.0").is_some());
    }

    #[test]
    fn test_storage_update() {
        let dir = tempfile::tempdir().unwrap();
        let mut s = FileStorage::new(dir.path());
        let mut r = make_record("v1.0.0", ReleaseStatus::Staged);
        s.save(&r).unwrap();
        r.status = ReleaseStatus::Published;
        r.updated_at = "999".into();
        s.save(&r).unwrap();
        let loaded = s.load("v1.0.0").unwrap();
        assert_eq!(loaded.status, ReleaseStatus::Published);
    }

    #[test]
    fn test_storage_list() {
        let dir = tempfile::tempdir().unwrap();
        let mut s = FileStorage::new(dir.path());
        s.save(&make_record("v1.0.0", ReleaseStatus::Staged))
            .unwrap();
        s.save(&make_record("v2.0.0", ReleaseStatus::Published))
            .unwrap();
        assert_eq!(s.list().len(), 2);
    }

    #[test]
    fn test_storage_persists() {
        let dir = tempfile::tempdir().unwrap();
        {
            let mut s = FileStorage::new(dir.path());
            s.save(&make_record("v1.0.0", ReleaseStatus::Staged))
                .unwrap();
        }
        {
            let s = FileStorage::new(dir.path());
            assert!(s.load("v1.0.0").is_some());
        }
    }

    #[test]
    fn test_journal_appended() {
        let dir = tempfile::tempdir().unwrap();
        let mut s = FileStorage::new(dir.path());
        let r = make_record("v1.0.0", ReleaseStatus::Staged);
        s.save(&r).unwrap();

        let journal = dir.path().join(".quanttide/devops/release-journal.jsonl");
        let content = std::fs::read_to_string(&journal).unwrap();
        assert!(content.contains("v1.0.0"));

        let mut r2 = r.clone();
        r2.status = ReleaseStatus::Published;
        s.save(&r2).unwrap();
        let content = std::fs::read_to_string(&journal).unwrap();
        assert_eq!(content.trim().lines().count(), 2);
    }

    #[test]
    fn test_created_at_preserved() {
        let dir = tempfile::tempdir().unwrap();
        let first_ts;
        {
            let mut s = FileStorage::new(dir.path());
            let r = make_record("v1.0.0", ReleaseStatus::Staged);
            first_ts = r.created_at.clone();
            s.save(&r).unwrap();
        }
        {
            let mut s = FileStorage::new(dir.path());
            let mut r = s.load("v1.0.0").unwrap();
            r.status = ReleaseStatus::Published;
            r.updated_at = timestamp();
            s.save(&r).unwrap();
        }
        {
            let s = FileStorage::new(dir.path());
            let r = s.load("v1.0.0").unwrap();
            assert_eq!(r.created_at, first_ts);
            assert_eq!(r.status, ReleaseStatus::Published);
        }
    }

    #[test]
    fn test_transition_error_display() {
        let e = TransitionError::NotStaged("v1.0.0".into());
        assert!(e.to_string().contains("Staged"));

        let e = TransitionError::NotPublished("v1.0.0".into());
        assert!(e.to_string().contains("Published"));
    }
}
