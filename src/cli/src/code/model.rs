#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SyncStatus {
    Synced,
    PendingPush,
    PendingPull,
    Conflict,
}

impl SyncStatus {
    pub fn label(&self) -> &str {
        match self {
            Self::Synced => "已同步",
            Self::PendingPush => "待推送",
            Self::PendingPull => "待拉取",
            Self::Conflict => "冲突",
        }
    }
}

#[derive(Debug, Clone)]
pub struct ComponentStatus {
    pub name: String,
    pub status: SyncStatus,
    pub ahead: usize,
    pub behind: usize,
}

#[derive(Debug, Clone)]
pub struct StatusReport {
    pub root: String,
    pub components: Vec<ComponentStatus>,
    pub total: usize,
    pub synced: usize,
    pub pending: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sync_status_labels() {
        assert_eq!(SyncStatus::Synced.label(), "已同步");
        assert_eq!(SyncStatus::PendingPush.label(), "待推送");
        assert_eq!(SyncStatus::PendingPull.label(), "待拉取");
        assert_eq!(SyncStatus::Conflict.label(), "冲突");
    }

    #[test]
    fn test_sync_status_clone_eq() {
        assert_eq!(SyncStatus::Synced, SyncStatus::Synced);
        assert_ne!(SyncStatus::Synced, SyncStatus::PendingPush);
    }

    #[test]
    fn test_component_status_builder() {
        let c = ComponentStatus { name: "libs/foo".into(), status: SyncStatus::PendingPush, ahead: 3, behind: 0 };
        assert_eq!(c.name, "libs/foo");
        assert_eq!(c.ahead, 3);
    }

    #[test]
    fn test_status_report_counts() {
        let report = StatusReport {
            root: "/tmp".into(),
            components: vec![
                ComponentStatus { name: "a".into(), status: SyncStatus::Synced, ahead: 0, behind: 0 },
                ComponentStatus { name: "b".into(), status: SyncStatus::PendingPush, ahead: 1, behind: 0 },
                ComponentStatus { name: "c".into(), status: SyncStatus::PendingPull, ahead: 0, behind: 2 },
                ComponentStatus { name: "d".into(), status: SyncStatus::Conflict, ahead: 0, behind: 0 },
            ],
            total: 4,
            synced: 1,
            pending: 3,
        };
        assert_eq!(report.total, 4);
        assert_eq!(report.synced, 1);
        assert_eq!(report.pending, 3);
    }
}
