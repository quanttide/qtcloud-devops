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
