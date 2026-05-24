use std::path::Path;

use crate::model::release::{FileStorage, ReleaseStatus, Storage, TransitionError};

pub fn run(version: &str, repo_path: &Path) -> Result<String, Box<dyn std::error::Error>> {
    let mut storage = FileStorage::new(repo_path);
    let mut record = storage
        .load(version)
        .ok_or_else(|| format!("版本 {} 不存在", version))?;

    if record.status != ReleaseStatus::Published {
        return Err(Box::new(TransitionError::NotPublished(version.to_string())));
    }

    record.status = ReleaseStatus::Retired;
    record.updated_at = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
        .to_string();
    storage.save(&record)?;

    let id = record.id.clone();
    println!("✓ 版本 {} 已退役 (发布尝试 ID: {})", version, id);
    Ok(id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::release::{ReleaseRecord, ReleaseStatus, Storage};

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
    fn test_retire_not_published() {
        let dir = tempfile::tempdir().unwrap();
        let mut s = FileStorage::new(dir.path());
        s.save(&make_record("v1.0.0", ReleaseStatus::Staged))
            .unwrap();
        assert!(run("v1.0.0", dir.path()).is_err());
    }

    #[test]
    fn test_retire_nonexistent() {
        let dir = tempfile::tempdir().unwrap();
        assert!(run("v9.9.9", dir.path()).is_err());
    }

    #[test]
    fn test_retire_from_published() {
        let dir = tempfile::tempdir().unwrap();
        {
            let mut s = FileStorage::new(dir.path());
            s.save(&make_record("v1.0.0", ReleaseStatus::Published))
                .unwrap();
        }
        run("v1.0.0", dir.path()).unwrap();
        let s = FileStorage::new(dir.path());
        assert_eq!(
            s.load("v1.0.0").unwrap().status,
            ReleaseStatus::Retired
        );
    }
}
