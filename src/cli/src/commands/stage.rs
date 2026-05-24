use std::path::Path;

use crate::model::release::{FileStorage, ReleaseRecord, ReleaseStatus, Storage};

pub fn run(version: &str, repo_path: &Path) -> Result<String, Box<dyn std::error::Error>> {
    if !crate::commands::release::validate_version(version) {
        return Err(format!("版本号格式错误: {}", version).into());
    }

    let mut storage = FileStorage::new(repo_path);

    if let Some(existing) = storage.load(version) {
        match existing.status {
            ReleaseStatus::Published => {
                return Err(format!("版本 {} 已发布，不可重复 stage", version).into());
            }
            ReleaseStatus::Staged => {
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs()
                    .to_string();
                let mut updated = existing.clone();
                updated.updated_at = now;
                storage.save(&updated)?;
                return Ok(updated.id);
            }
            ReleaseStatus::Cancelled => {}
            ReleaseStatus::Retired => {
                return Err(format!("版本 {} 已退役，不可重复 stage", version).into());
            }
        }
    }

    let record = ReleaseRecord::new_staged(version);
    storage.save(&record)?;
    println!("✓ 版本 {} 已进入 Staged 状态 (发布尝试 ID: {})", version, record.id);
    Ok(record.id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::release::Storage;

    fn make_record(version: &str, status: ReleaseStatus) -> ReleaseRecord {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
            .to_string();
        ReleaseRecord {
            id: uuid::Uuid::new_v4().to_string(),
            version: version.to_string(),
            status,
            created_at: now.clone(),
            updated_at: now,
        }
    }

    #[test]
    fn test_stage_new_version() {
        let dir = tempfile::tempdir().unwrap();
        let id = run("v1.0.0", dir.path()).unwrap();
        assert!(!id.is_empty());

        let s = FileStorage::new(dir.path());
        let r = s.load("v1.0.0").unwrap();
        assert_eq!(r.status, ReleaseStatus::Staged);
    }

    #[test]
    fn test_stage_invalid_version() {
        let dir = tempfile::tempdir().unwrap();
        assert!(run("bad", dir.path()).is_err());
    }

    #[test]
    fn test_stage_published_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let mut s = FileStorage::new(dir.path());
        s.save(&make_record("v1.0.0", ReleaseStatus::Published))
            .unwrap();
        let err = run("v1.0.0", dir.path()).unwrap_err().to_string();
        assert!(err.contains("已发布"));
    }

    #[test]
    fn test_stage_cancelled_restage() {
        let dir = tempfile::tempdir().unwrap();
        let old_id;
        {
            let mut s = FileStorage::new(dir.path());
            let r = make_record("v1.0.0", ReleaseStatus::Cancelled);
            old_id = r.id.clone();
            s.save(&r).unwrap();
        }
        let id = run("v1.0.0", dir.path()).unwrap();
        assert!(!id.is_empty());

        let s = FileStorage::new(dir.path());
        let loaded = s.load("v1.0.0").unwrap();
        assert_eq!(loaded.status, ReleaseStatus::Staged);
        assert_ne!(loaded.id, old_id);
    }

    #[test]
    fn test_stage_retired_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let mut s = FileStorage::new(dir.path());
        s.save(&make_record("v1.0.0", ReleaseStatus::Retired))
            .unwrap();
        assert!(run("v1.0.0", dir.path()).unwrap_err().to_string().contains("退役"));
    }

    #[test]
    fn test_stage_idempotent() {
        let dir = tempfile::tempdir().unwrap();
        let id1 = run("v1.0.0", dir.path()).unwrap();
        let id2 = run("v1.0.0", dir.path()).unwrap();
        assert_eq!(id1, id2);
        assert_eq!(FileStorage::new(dir.path()).list().len(), 1);
    }
}
