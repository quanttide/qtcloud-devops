pub use std::path::PathBuf;

/// 截断 OID 到 7 字符显示（等价旧 CommitHash Display）。
pub fn fmt_oid(id: &gix::ObjectId) -> String {
    gix::hash::Prefix::new(id, 7)
        .map(|p| p.to_string())
        .unwrap_or_else(|_| id.to_string())
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub enum SubmoduleStatus {
    Clean,
    AheadOfParent,
    BehindRemote,
    Detached,
    Dirty,
    Orphaned,
    Uninitialized,
}

impl SubmoduleStatus {
    pub fn priority(&self) -> u8 {
        match self {
            Self::Dirty => 0,
            Self::Orphaned => 1,
            Self::Detached => 2,
            Self::Uninitialized => 3,
            Self::BehindRemote => 4,
            Self::AheadOfParent => 5,
            Self::Clean => 6,
        }
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct Submodule {
    pub name: String,
    pub path: PathBuf,
    pub url: String,
    pub tracked_branch: String,
    pub parent_pointer: gix::ObjectId,
    pub local_head: gix::ObjectId,
    pub remote_head: gix::ObjectId,
    pub status: SubmoduleStatus,
    pub ahead_count: usize,
    pub behind_count: usize,
    pub remote_unreachable: bool,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct RepoState {
    pub root_path: PathBuf,
    pub submodules: Vec<Submodule>,
    pub total: usize,
    pub clean_count: usize,
    pub needs_attention: Vec<String>,
}

#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct AggregateStatus {
    pub total: usize,
    pub clean: usize,
    pub ahead_of_parent: usize,
    pub behind_remote: usize,
    pub detached: usize,
    pub dirty: usize,
    pub orphaned: usize,
    pub uninitialized: usize,
}

impl AggregateStatus {
    pub fn from_submodules(submodules: &[Submodule]) -> Self {
        let mut clean = 0;
        let mut ahead = 0;
        let mut behind = 0;
        let mut detached = 0;
        let mut dirty = 0;
        let mut orphaned = 0;
        let mut uninit = 0;
        for sm in submodules {
            match sm.status {
                SubmoduleStatus::Clean => clean += 1,
                SubmoduleStatus::AheadOfParent => ahead += 1,
                SubmoduleStatus::BehindRemote => behind += 1,
                SubmoduleStatus::Detached => detached += 1,
                SubmoduleStatus::Dirty => dirty += 1,
                SubmoduleStatus::Orphaned => orphaned += 1,
                SubmoduleStatus::Uninitialized => uninit += 1,
            }
        }
        AggregateStatus {
            total: submodules.len(),
            clean,
            ahead_of_parent: ahead,
            behind_remote: behind,
            detached,
            dirty,
            orphaned,
            uninitialized: uninit,
        }
    }
}

#[derive(Debug, Clone)]
pub struct HealthIssue {
    pub submodule_name: String,
    pub status: String,
    pub description: String,
    pub suggested_action: String,
}

pub fn describe_issue(status: &SubmoduleStatus) -> (String, String) {
    match status {
        SubmoduleStatus::AheadOfParent => (
            "本地领先于父仓库记录".into(),
            "运行 sync_to_parent 更新父仓库指针".into(),
        ),
        SubmoduleStatus::BehindRemote => (
            "远程有更新，本地落后".into(),
            "运行 code sync 获取最新代码".into(),
        ),
        SubmoduleStatus::Detached => (
            "处于游离 HEAD 状态".into(),
            "运行 checkout_branch 切换到跟踪分支".into(),
        ),
        SubmoduleStatus::Dirty => ("有未提交的修改".into(), "提交或 stash 当前修改".into()),
        SubmoduleStatus::Orphaned => (
            "父仓库记录的 commit 在远程已不存在".into(),
            "需手动干预".into(),
        ),
        SubmoduleStatus::Uninitialized => ("尚未初始化".into(), "运行 init 初始化子模块".into()),
        SubmoduleStatus::Clean => unreachable!(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---- SubmoduleStatus tests ----
    #[test]
    fn test_status_priority_ordering() {
        assert!(SubmoduleStatus::Dirty.priority() < SubmoduleStatus::Clean.priority());
        assert!(SubmoduleStatus::Orphaned.priority() < SubmoduleStatus::BehindRemote.priority());
    }
    #[test]
    fn test_clean_is_lowest_priority() {
        for s in &[
            SubmoduleStatus::Dirty,
            SubmoduleStatus::Orphaned,
            SubmoduleStatus::Detached,
            SubmoduleStatus::Uninitialized,
            SubmoduleStatus::BehindRemote,
            SubmoduleStatus::AheadOfParent,
        ] {
            assert!(s.priority() < SubmoduleStatus::Clean.priority());
        }
    }
    #[test]
    fn test_all_priorities_are_unique() {
        let p: Vec<u8> = [
            SubmoduleStatus::Dirty,
            SubmoduleStatus::Orphaned,
            SubmoduleStatus::Detached,
            SubmoduleStatus::Uninitialized,
            SubmoduleStatus::BehindRemote,
            SubmoduleStatus::AheadOfParent,
            SubmoduleStatus::Clean,
        ]
        .iter()
        .map(|s| s.priority())
        .collect();
        let mut s = p.clone();
        s.sort();
        s.dedup();
        assert_eq!(p.len(), s.len());
    }
    #[test]
    fn test_status_debug_output() {
        assert_eq!(format!("{:?}", SubmoduleStatus::Clean), "Clean");
    }
    #[test]
    fn test_status_clone_eq() {
        assert_eq!(SubmoduleStatus::Dirty, SubmoduleStatus::Dirty);
    }

    // ---- fmt_oid ----
    #[test]
    fn test_fmt_oid_truncates() {
        let oid = gix::ObjectId::from_hex(b"abcdef1234567890abcdef1234567890abcdef12").unwrap();
        assert_eq!(fmt_oid(&oid), "abcdef1");
    }
    #[test]
    fn test_fmt_oid_short() {
        // 创建一个只有 3 有效 hex 位的 OID 不好搞，直接测全零
        let oid = gix::ObjectId::null(gix::hash::Kind::Sha1);
        assert_eq!(fmt_oid(&oid).len(), 7);
    }
    #[test]
    fn test_fmt_oid_null() {
        let oid = gix::ObjectId::null(gix::hash::Kind::Sha1);
        let s = fmt_oid(&oid);
        assert_eq!(s.len(), 7);
        assert!(s.chars().all(|c| c == '0'));
    }

    // ---- Submodule ----
    #[test]
    fn test_submodule_builder() {
        let sm = Submodule {
            name: "test".into(),
            path: PathBuf::from("libs/test"),
            url: "https://example.com/test.git".into(),
            tracked_branch: "main".into(),
            parent_pointer: gix::ObjectId::null(gix::hash::Kind::Sha1),
            local_head: gix::ObjectId::null(gix::hash::Kind::Sha1),
            remote_head: gix::ObjectId::null(gix::hash::Kind::Sha1),
            status: SubmoduleStatus::BehindRemote,
            ahead_count: 0,
            behind_count: 3,
            remote_unreachable: false,
        };
        assert_eq!(sm.name, "test");
    }

    // ---- AggregateStatus ----
    #[test]
    fn test_aggregate_status_default() {
        assert_eq!(AggregateStatus::default().total, 0);
    }
    #[test]
    fn test_aggregate_status_from_submodules() {
        let sm = |s| Submodule {
            name: String::new(),
            path: PathBuf::new(),
            url: String::new(),
            tracked_branch: "main".into(),
            parent_pointer: gix::ObjectId::null(gix::hash::Kind::Sha1),
            local_head: gix::ObjectId::null(gix::hash::Kind::Sha1),
            remote_head: gix::ObjectId::null(gix::hash::Kind::Sha1),
            status: s,
            ahead_count: 0,
            behind_count: 0,
            remote_unreachable: false,
        };
        let agg = AggregateStatus::from_submodules(&[
            sm(SubmoduleStatus::Clean),
            sm(SubmoduleStatus::Dirty),
            sm(SubmoduleStatus::Orphaned),
        ]);
        assert_eq!(agg.total, 3);
        assert_eq!(agg.clean, 1);
        assert_eq!(agg.dirty, 1);
        assert_eq!(agg.orphaned, 1);
    }
    #[test]
    fn test_aggregate_status_all_variants() {
        let sm = |s| Submodule {
            name: String::new(),
            path: PathBuf::new(),
            url: String::new(),
            tracked_branch: "main".into(),
            parent_pointer: gix::ObjectId::null(gix::hash::Kind::Sha1),
            local_head: gix::ObjectId::null(gix::hash::Kind::Sha1),
            remote_head: gix::ObjectId::null(gix::hash::Kind::Sha1),
            status: s,
            ahead_count: 0,
            behind_count: 0,
            remote_unreachable: false,
        };
        let agg = AggregateStatus::from_submodules(&[
            sm(SubmoduleStatus::Clean),
            sm(SubmoduleStatus::AheadOfParent),
            sm(SubmoduleStatus::BehindRemote),
            sm(SubmoduleStatus::Detached),
            sm(SubmoduleStatus::Dirty),
            sm(SubmoduleStatus::Orphaned),
            sm(SubmoduleStatus::Uninitialized),
        ]);
        assert_eq!(agg.total, 7);
        assert_eq!(agg.clean, 1);
    }

    // ---- RepoState ----
    #[test]
    fn test_repo_state_empty() {
        let s = RepoState {
            root_path: PathBuf::from("/tmp"),
            submodules: vec![],
            total: 0,
            clean_count: 0,
            needs_attention: vec![],
        };
        assert_eq!(s.total, 0);
    }

    // ---- describe_issue ----
    #[test]
    fn test_describe_issue_ahead_of_parent() {
        let (d, a) = describe_issue(&SubmoduleStatus::AheadOfParent);
        assert!(d.contains("领先"));
        assert!(a.contains("sync"));
    }
    #[test]
    fn test_describe_issue_behind_remote() {
        let (d, a) = describe_issue(&SubmoduleStatus::BehindRemote);
        assert!(d.contains("落后"));
        assert!(a.contains("sync"));
    }
    #[test]
    fn test_describe_issue_detached() {
        let (d, a) = describe_issue(&SubmoduleStatus::Detached);
        assert!(d.contains("游离"));
        assert!(a.contains("checkout"));
    }
    #[test]
    fn test_describe_issue_dirty() {
        let (d, _a) = describe_issue(&SubmoduleStatus::Dirty);
        assert!(d.contains("修改"));
    }
    #[test]
    fn test_describe_issue_orphaned() {
        let (d, _a) = describe_issue(&SubmoduleStatus::Orphaned);
        assert!(d.contains("不存在"));
    }
    #[test]
    fn test_describe_issue_uninitialized() {
        let (d, _a) = describe_issue(&SubmoduleStatus::Uninitialized);
        assert!(d.contains("初始化"));
    }
    #[test]
    #[should_panic(expected = "unreachable")]
    fn test_describe_issue_clean_panics() {
        describe_issue(&SubmoduleStatus::Clean);
    }
}
