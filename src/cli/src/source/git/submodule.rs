use std::path::{Path, PathBuf};
#[cfg(test)]
use std::process::Command;

pub use quanttide_devops::source::git::submodule::{
    AggregateStatus, HealthIssue, RepoState, Submodule, SubmoduleStatus,
    describe_issue, fmt_oid,
};

/// 子模块状态快照，用于 `determine_submodule_status` 入参。
struct SubmoduleState {
    is_uninitialized: bool,
    is_dirty: bool,
    is_detached: bool,
    is_orphaned: bool,
    remote_unreachable: bool,
    ahead_count: usize,
    behind_count: usize,
    local_head: gix::ObjectId,
    parent_pointer: gix::ObjectId,
}

impl Default for SubmoduleState {
    fn default() -> Self {
        let null = gix::ObjectId::null(gix::hash::Kind::Sha1);
        Self {
            is_uninitialized: false,
            is_dirty: false,
            is_detached: false,
            is_orphaned: false,
            remote_unreachable: false,
            ahead_count: 0,
            behind_count: 0,
            local_head: null,
            parent_pointer: null,
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════
// scan
// ═══════════════════════════════════════════════════════════════════════

/// 从 .gitmodules 解析子模块列表：name, path, url, branch
pub fn parse_gitmodules(root: &Path) -> Vec<(String, PathBuf, String, String)> {
    let cfg_path = root.join(".gitmodules");
    let content = match std::fs::read_to_string(&cfg_path) {
        Ok(c) => c,
        Err(_) => return vec![],
    };
    let file = match gix::submodule::File::from_bytes(
        content.as_bytes(),
        None,
        &gix::config::File::default(),
    ) {
        Ok(f) => f,
        Err(_) => return vec![],
    };
    file.names()
        .filter_map(|name| {
            let name_str = name.to_string();
            let p = file.path(name).ok()?;
            let url = file.url(name).ok()?;
            let branch = match file.branch(name).ok().flatten() {
                Some(gix::submodule::config::Branch::Name(b)) => b.to_string(),
                _ => "main".to_string(),
            };
            Some((
                name_str,
                PathBuf::from(p.as_ref().to_string()),
                url.to_string(),
                branch,
            ))
        })
        .collect()
}

/// 用 gix 统计两个 commit 之间的提交数（等价于 `git rev-list --count from..to`）。
fn gix_count_between(repo: &gix::Repository, from: gix::ObjectId, to: gix::ObjectId) -> usize {
    // ponytail: 逐 parent 遍历而非 revwalk Platform，避免 API 兼容问题
    let mut count = 0;
    let mut current = to;
    loop {
        if current == from {
            break;
        }
        if count > 10000 {
            break;
        }
        let commit = match repo.find_commit(current) {
            Ok(c) => c,
            Err(_) => break,
        };
        let mut parents = commit.parent_ids();
        match parents.next() {
            Some(id) => current = id.into(),
            None => break,
        }
        count += 1;
    }
    count
}

/// 用 gix 读取 HEAD:path tree entry 的 OID（支持多级路径）。
fn gix_tree_entry_id(repo: &gix::Repository, path: &Path) -> Option<gix::ObjectId> {
    let commit = repo.head_commit().ok()?;
    let tree = commit.tree().ok()?;
    let path_str = path.to_string_lossy();
    let components: Vec<&str> = path_str.split('/').collect();
    let entry = tree.lookup_entry(components).ok()??;
    Some(entry.id().into())
}

pub fn scan_repo_state(root: &Path) -> Result<RepoState, Box<dyn std::error::Error>> {
    scan_repo_state_with_options(root, false)
}

    pub fn scan_repo_state_offline(root: &Path) -> Result<RepoState, Box<dyn std::error::Error>> {
    scan_repo_state_with_options(root, true)
}

    fn scan_repo_state_with_options(root: &Path, offline: bool) -> Result<RepoState, Box<dyn std::error::Error>> {
        if gix::open(root).is_err() {
            return Err(format!("不在 git 仓库中: {:?}", root).into());
        }

        let raw_entries = parse_gitmodules(root);
        let mut submodules: Vec<Submodule> = Vec::with_capacity(raw_entries.len());

        let parent_repo = gix::open(root).ok();

        for (name, sm_path, url, branch) in &raw_entries {
            let full_sm_path = root.join(sm_path);

            let parent_pointer = parent_repo
                .as_ref()
                .and_then(|r| gix_tree_entry_id(r, sm_path))
                .unwrap_or(gix::ObjectId::null(gix::hash::Kind::Sha1));

            let (
                local_head,
                remote_head,
                is_detached,
                ahead_count,
                behind_count,
                is_orphaned,
                remote_unreachable,
                is_uninitialized,
                is_dirty,
            ) = scan_single_submodule(&full_sm_path, branch, &parent_pointer, offline);

            let status = determine_submodule_status(&SubmoduleState {
                is_uninitialized,
                is_dirty,
                is_detached,
                is_orphaned,
                remote_unreachable,
                ahead_count,
                behind_count,
                local_head,
                parent_pointer,
            });

            submodules.push(Submodule {
                name: name.clone(),
                path: sm_path.clone(),
                url: url.clone(),
                tracked_branch: branch.clone(),
                parent_pointer,
                local_head,
                remote_head,
                status,
                ahead_count,
                behind_count,
                remote_unreachable,
            });
        }

        Ok(build_repo_state(root, submodules))
    }

    fn build_repo_state(root: &Path, mut submodules: Vec<Submodule>) -> RepoState {
        submodules.sort_by(|a, b| a.name.cmp(&b.name));
        let total = submodules.len();
        let clean_count = submodules
            .iter()
            .filter(|s| s.status == SubmoduleStatus::Clean)
            .count();
        let needs_attention: Vec<String> = submodules
            .iter()
            .filter(|s| s.status != SubmoduleStatus::Clean)
            .map(|s| s.name.clone())
            .collect();

        RepoState {
            root_path: root.to_path_buf(),
            submodules,
            total,
            clean_count,
            needs_attention,
        }
    }

    fn default_uninitialized_result() -> (
        gix::ObjectId,
        gix::ObjectId,
        bool,
        usize,
        usize,
        bool,
        bool,
        bool,
        bool,
    ) {
        let null = gix::ObjectId::null(gix::hash::Kind::Sha1);
        (null, null, false, 0, 0, false, false, true, false)
    }

    fn read_local_head(sm_repo: &Option<gix::Repository>) -> gix::ObjectId {
        sm_repo
            .as_ref()
            .and_then(|r| r.head_commit().ok().map(|c| c.id().into()))
            .unwrap_or(gix::ObjectId::null(gix::hash::Kind::Sha1))
    }

    fn check_detached(sm_repo: &Option<gix::Repository>) -> bool {
        sm_repo
            .as_ref()
            .map(|r| r.head().ok().map(|h| h.is_detached()).unwrap_or(false))
            .unwrap_or(false)
    }

    fn check_dirty(full_sm_path: &Path) -> bool {
        git2::Repository::open(full_sm_path)
            .map(|r| {
                r.statuses(Some(git2::StatusOptions::new().include_untracked(true)))
                    .map(|s| s.len() > 0)
                    .unwrap_or(false)
            })
            .unwrap_or(false)
    }

    fn try_fetch_remote(full_sm_path: &Path, offline: bool) {
        if offline {
            return;
        }
        if let Ok(repo) = git2::Repository::open(full_sm_path) {
            if let Ok(mut remote) = repo.find_remote("origin") {
                let _ = remote.fetch(&[] as &[&str], None, None);
            }
        }
    }

    fn read_remote_state(sm_repo: &Option<gix::Repository>, branch: &str) -> (gix::ObjectId, bool) {
        let remote_ref = format!("refs/remotes/origin/{}", branch);
        sm_repo
            .as_ref()
            .and_then(|r| r.find_reference(&remote_ref).ok())
            .map(|r| {
                let target = r.target();
                let id: gix::ObjectId = (*target.id()).into();
                if id.is_null() {
                    (gix::ObjectId::null(gix::hash::Kind::Sha1), true)
                } else {
                    (id, false)
                }
            })
            .unwrap_or((gix::ObjectId::null(gix::hash::Kind::Sha1), true))
    }

    #[allow(clippy::too_many_arguments)]
    fn scan_single_submodule(
        full_sm_path: &Path,
        branch: &str,
        parent_pointer: &gix::ObjectId,
        offline: bool,
    ) -> (
        gix::ObjectId,
        gix::ObjectId,
        bool,
        usize,
        usize,
        bool,
        bool,
        bool,
        bool,
    ) {
        if !full_sm_path.exists() || !full_sm_path.join(".git").exists() {
            return default_uninitialized_result();
        }

        let sm_repo = gix::open(full_sm_path).ok();
        let local_head = read_local_head(&sm_repo);
        let is_detached = check_detached(&sm_repo);
        let is_dirty = check_dirty(full_sm_path);
        try_fetch_remote(full_sm_path, offline);

        let (remote_head, remote_unreachable) = read_remote_state(&sm_repo, branch);
        let ahead = sm_repo
            .as_ref()
            .map(|r| gix_count_between(r, *parent_pointer, local_head))
            .unwrap_or(0);
        let behind = if remote_unreachable {
            0
        } else {
            sm_repo
                .as_ref()
                .map(|r| gix_count_between(r, local_head, remote_head))
                .unwrap_or(0)
        };
        let is_orphaned = !remote_unreachable
            && remote_head != gix::ObjectId::null(gix::hash::Kind::Sha1)
            && *parent_pointer != remote_head;

        (
            local_head,
            remote_head,
            is_detached,
            ahead,
            behind,
            is_orphaned,
            remote_unreachable,
            false,
            is_dirty,
        )
    }

    fn determine_submodule_status(state: &SubmoduleState) -> SubmoduleStatus {
        if state.is_uninitialized {
            return SubmoduleStatus::Uninitialized;
        }
        if state.is_dirty {
            return SubmoduleStatus::Dirty;
        }
        if state.is_detached {
            return SubmoduleStatus::Detached;
        }
        if state.is_orphaned && !state.remote_unreachable {
            return SubmoduleStatus::Orphaned;
        }
        if (state.remote_unreachable && state.local_head != state.parent_pointer)
            || (state.ahead_count > 0 && state.behind_count == 0)
        {
            return SubmoduleStatus::AheadOfParent;
        }
        if state.behind_count > 0 && !state.remote_unreachable {
            return SubmoduleStatus::BehindRemote;
        }
        SubmoduleStatus::Clean
    }

    pub fn scan_all_submodules(
    root: &Path,
) -> Result<(Vec<Submodule>, AggregateStatus), Box<dyn std::error::Error>> {
    let state = scan_repo_state(root)?;
    let agg = AggregateStatus::from_submodules(&state.submodules);
    Ok((state.submodules, agg))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn git_init(path: &std::path::Path) {
        let repo = git2::Repository::init(path).unwrap();
        let mut cfg = repo.config().unwrap();
        cfg.set_str("user.email", "t@t").unwrap();
        cfg.set_str("user.name", "t").unwrap();
    }

    fn git_commit(path: &std::path::Path, msg: &str) {
        std::fs::write(path.join("f"), msg).unwrap();
        let repo = git2::Repository::open(path).unwrap();
        let mut index = repo.index().unwrap();
        index.add_path(std::path::Path::new("f")).unwrap();
        index.write().unwrap();
        let tree_id = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();
        let sig = repo.signature().unwrap();
        let parent = repo.head().and_then(|h| h.peel_to_commit()).ok();
        let parents: Vec<&git2::Commit> = parent.iter().collect();
        repo.commit(Some("HEAD"), &sig, &sig, msg, &tree, &parents)
            .unwrap();
    }

    fn setup_repo_with_submodule(tmp: &Path) -> PathBuf {
        let parent = tmp.join("parent");
        let sub = tmp.join("sub");
        std::fs::create_dir_all(&sub).unwrap();
        git_init(&sub);
        git_commit(&sub, "init sub");
        std::fs::create_dir_all(&parent).unwrap();
        git_init(&parent);
        git_commit(&parent, "init parent");
        Command::new("git")
            .args(["submodule", "add", &sub.to_string_lossy(), "libs/sub"])
            .current_dir(&parent)
            .output()
            .unwrap();
        Command::new("git")
            .args(["commit", "-m", "add submodule"])
            .current_dir(&parent)
            .output()
            .unwrap();
        parent
    }

    fn dh() -> gix::ObjectId {
        gix::ObjectId::null(gix::hash::Kind::Sha1)
    }

    fn h(s: &str) -> gix::ObjectId {
        let padded = format!("{:0>40}", s);
        gix::ObjectId::from_hex(padded.as_bytes())
            .unwrap_or(gix::ObjectId::null(gix::hash::Kind::Sha1))
    }

    fn ds(state: SubmoduleState) -> SubmoduleStatus {
        determine_submodule_status(&state)
    }

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

    // ---- determine_submodule_status ----
    #[test]
    fn test_determine_status_uninitialized() {
        assert_eq!(
            ds(SubmoduleState {
                is_uninitialized: true,
                ..SubmoduleState::default()
            }),
            SubmoduleStatus::Uninitialized
        );
    }
    #[test]
    fn test_determine_status_dirty() {
        assert_eq!(
            ds(SubmoduleState {
                is_dirty: true,
                ..SubmoduleState::default()
            }),
            SubmoduleStatus::Dirty
        );
    }
    #[test]
    fn test_determine_status_detached() {
        assert_eq!(
            ds(SubmoduleState {
                is_detached: true,
                ..SubmoduleState::default()
            }),
            SubmoduleStatus::Detached
        );
    }
    #[test]
    fn test_determine_status_orphaned() {
        assert_eq!(
            ds(SubmoduleState {
                is_orphaned: true,
                ..SubmoduleState::default()
            }),
            SubmoduleStatus::Orphaned
        );
    }
    #[test]
    fn test_determine_status_ahead_of_parent() {
        assert_eq!(
            ds(SubmoduleState {
                remote_unreachable: true,
                local_head: h("abc"),
                ..SubmoduleState::default()
            }),
            SubmoduleStatus::AheadOfParent
        );
        assert_eq!(
            ds(SubmoduleState {
                ahead_count: 5,
                ..SubmoduleState::default()
            }),
            SubmoduleStatus::AheadOfParent
        );
        assert_eq!(
            ds(SubmoduleState {
                ahead_count: 5,
                behind_count: 3,
                ..SubmoduleState::default()
            }),
            SubmoduleStatus::BehindRemote
        );
    }
    #[test]
    fn test_determine_status_behind_remote() {
        assert_eq!(
            ds(SubmoduleState {
                behind_count: 3,
                ..SubmoduleState::default()
            }),
            SubmoduleStatus::BehindRemote
        );
        assert_eq!(
            ds(SubmoduleState {
                behind_count: 3,
                remote_unreachable: true,
                ..SubmoduleState::default()
            }),
            SubmoduleStatus::Clean
        );
    }
    #[test]
    fn test_determine_status_clean() {
        assert_eq!(ds(SubmoduleState::default()), SubmoduleStatus::Clean);
    }

    // ---- gix_count_between ----
    #[test]
    fn test_count_between_commits() {
        let t = tempfile::tempdir().unwrap();
        git_init(t.path());
        git_commit(t.path(), "c1");
        let repo = gix::open(t.path()).unwrap();
        let head: gix::ObjectId = repo.head_commit().unwrap().id().into();
        assert_eq!(gix_count_between(&repo, head, head), 0);
        git_commit(t.path(), "c2");
        let head2: gix::ObjectId = repo.head_commit().unwrap().id().into();
        assert_eq!(gix_count_between(&repo, head, head2), 1);
    }
    #[test]
    fn test_count_between_one_commit() {
        let t = tempfile::tempdir().unwrap();
        git_init(t.path());
        git_commit(t.path(), "c1");
        let repo = gix::open(t.path()).unwrap();
        let c1: gix::ObjectId = repo.head_commit().unwrap().id().into();
        git_commit(t.path(), "c2");
        let head: gix::ObjectId = repo.head_commit().unwrap().id().into();
        assert_eq!(gix_count_between(&repo, c1, head), 1);
    }

    // ---- scan tests ----
    #[test]
    fn test_scan_no_gitmodules() {
        assert!(scan_repo_state(&tempfile::tempdir().unwrap().path()).is_err());
    }
    #[test]
    fn test_scan_git_repo_but_no_submodules() {
        let t = tempfile::tempdir().unwrap();
        git_init(t.path());
        git_commit(t.path(), "initial");
        assert_eq!(scan_repo_state(t.path()).unwrap().total, 0);
    }
    #[test]
    fn test_scan_non_git_directory() {
        let t = tempfile::tempdir().unwrap();
        std::fs::write(t.path().join(".gitmodules"), "").unwrap();
        assert!(scan_repo_state(t.path()).is_err());
    }
    #[test]
    fn test_scan_with_submodule() {
        let t = tempfile::tempdir().unwrap();
        let p = setup_repo_with_submodule(t.path());
        let s = scan_repo_state(&p).unwrap();
        assert_eq!(s.total, 1);
        assert_eq!(s.submodules[0].name, "libs/sub");
    }
    #[test]
    fn test_scan_all_no_gitmodules() {
        assert!(scan_all_submodules(&tempfile::tempdir().unwrap().path()).is_err());
    }
    #[test]
    fn test_scan_all_with_submodule() {
        let t = tempfile::tempdir().unwrap();
        let p = setup_repo_with_submodule(t.path());
        let (subs, _) = scan_all_submodules(&p).unwrap();
        assert_eq!(subs.len(), 1);
    }
    #[test]
    fn test_scan_with_uninitialized_submodule() {
        let tmp = tempfile::tempdir().unwrap();
        let parent = tmp.path().join("parent");
        std::fs::create_dir_all(&parent).unwrap();
        git_init(&parent);
        git_commit(&parent, "init");
        let sub = tmp.path().join("sub");
        std::fs::create_dir_all(&sub).unwrap();
        git_init(&sub);
        git_commit(&sub, "init");
        Command::new("git")
            .args(["submodule", "add", &sub.to_string_lossy(), "libs/sub"])
            .current_dir(&parent)
            .output()
            .unwrap();
        Command::new("git")
            .args(["commit", "-m", "add submodule"])
            .current_dir(&parent)
            .output()
            .unwrap();
        Command::new("git")
            .args(["submodule", "deinit", "-f", "libs/sub"])
            .current_dir(&parent)
            .output()
            .unwrap();
        assert_eq!(
            scan_repo_state(&parent).unwrap().submodules[0].status,
            SubmoduleStatus::Uninitialized
        );
    }
    #[test]
    fn test_scan_with_detached_submodule() {
        let tmp = tempfile::tempdir().unwrap();
        let parent = setup_repo_with_submodule(tmp.path());
        let sm_path = parent.join("libs/sub");
        let hash = String::from_utf8_lossy(
            &Command::new("git")
                .args(["rev-parse", "HEAD"])
                .current_dir(&sm_path)
                .output()
                .unwrap()
                .stdout,
        )
        .trim()
        .to_string();
        Command::new("git")
            .args(["checkout", "--detach", &hash])
            .current_dir(&sm_path)
            .output()
            .unwrap();
        assert_eq!(
            scan_repo_state(&parent).unwrap().submodules[0].status,
            SubmoduleStatus::Detached
        );
    }
    #[test]
    fn test_scan_with_ahead_via_remote_unreachable() {
        let tmp = tempfile::tempdir().unwrap();
        let parent = setup_repo_with_submodule(tmp.path());
        let sm_path = parent.join("libs/sub");
        std::fs::write(sm_path.join("new-file"), "content").unwrap();
        Command::new("git")
            .args(["add", "."])
            .current_dir(&sm_path)
            .output()
            .unwrap();
        Command::new("git")
            .args(["commit", "-m", "ahead commit"])
            .current_dir(&sm_path)
            .output()
            .unwrap();
        Command::new("git")
            .args(["remote", "remove", "origin"])
            .current_dir(&sm_path)
            .output()
            .unwrap();
        let state = scan_repo_state(&parent).unwrap();
        assert_eq!(state.submodules[0].status, SubmoduleStatus::AheadOfParent);
    }
    #[test]
    fn test_scan_with_subrepo_open_error() {
        let tmp = tempfile::tempdir().unwrap();
        let parent = setup_repo_with_submodule(tmp.path());
        let sm_git = parent.join("libs/sub/.git");
        if sm_git.is_dir() {
            std::fs::remove_dir_all(&sm_git).unwrap();
        } else {
            std::fs::remove_file(&sm_git).unwrap();
        }
        assert_eq!(
            scan_repo_state(&parent).unwrap().submodules[0].local_head,
            gix::ObjectId::null(gix::hash::Kind::Sha1)
        );
    }
    #[test]
    fn test_scan_with_behind_remote() {
        let tmp = tempfile::tempdir().unwrap();
        let parent = tmp.path().join("parent");
        let sub = tmp.path().join("sub");
        let bare = tmp.path().join("bare");
        std::fs::create_dir_all(&bare).unwrap();
        Command::new("git")
            .args(["init", "--bare", &bare.to_string_lossy()])
            .current_dir(tmp.path())
            .output()
            .unwrap();
        Command::new("git")
            .args(["clone", &bare.to_string_lossy(), &sub.to_string_lossy()])
            .current_dir(tmp.path())
            .output()
            .unwrap();
        git_init(&sub);
        git_commit(&sub, "init");
        Command::new("git")
            .args(["push", "origin", "main"])
            .current_dir(&sub)
            .output()
            .unwrap();
        std::fs::create_dir_all(&parent).unwrap();
        git_init(&parent);
        git_commit(&parent, "init parent");
        Command::new("git")
            .args(["submodule", "add", &sub.to_string_lossy(), "libs/sub"])
            .current_dir(&parent)
            .output()
            .unwrap();
        Command::new("git")
            .args(["commit", "-m", "add submodule"])
            .current_dir(&parent)
            .output()
            .unwrap();
        git_commit(&sub, "remote ahead");
        Command::new("git")
            .args(["push", "origin", "main"])
            .current_dir(&sub)
            .output()
            .unwrap();
        Command::new("git")
            .args(["fetch", "origin"])
            .current_dir(&parent.join("libs/sub"))
            .output()
            .unwrap();
        assert_eq!(
            scan_repo_state(&parent).unwrap().submodules[0].behind_count,
            1
        );
    }
    #[test]
    fn test_scan_with_orphaned_submodule() {
        let tmp = tempfile::tempdir().unwrap();
        let parent = setup_repo_with_submodule(tmp.path());
        let sm_path = parent.join("libs/sub");
        Command::new("git")
            .args(["remote", "remove", "origin"])
            .current_dir(&sm_path)
            .output()
            .unwrap();
        let ref_dir = parent.join(".git/modules/libs/sub/refs/remotes/origin");
        std::fs::create_dir_all(&ref_dir).unwrap();
        std::fs::write(
            ref_dir.join("main"),
            "1111111111111111111111111111111111111111\n",
        )
        .unwrap();
        assert_eq!(
            scan_repo_state(&parent).unwrap().submodules[0].status,
            SubmoduleStatus::Orphaned
        );
    }
    #[test]
    fn test_scan_with_ahead_of_parent_clean() {
        let tmp = tempfile::tempdir().unwrap();
        let parent = setup_repo_with_submodule(tmp.path());
        git_commit(&parent.join("libs/sub"), "ahead commit");
        assert!(scan_repo_state(&parent).unwrap().submodules[0].ahead_count > 0);
    }
    #[test]
    fn test_orphaned_parse_oid_failure() {
        let tmp = tempfile::tempdir().unwrap();
        let parent = setup_repo_with_submodule(tmp.path());
        let ref_dir = parent.join(".git/modules/libs/sub/refs/remotes/origin");
        if !ref_dir.exists() {
            std::fs::create_dir_all(&ref_dir).unwrap();
        }
        std::fs::write(ref_dir.join("main"), "not-a-valid-oid\n").unwrap();
        assert!(!scan_repo_state(&parent).unwrap().submodules.is_empty());
    }
    #[test]
    fn test_ahead_of_parent_via_ahead_count() {
        let tmp = tempfile::tempdir().unwrap();
        let parent = setup_repo_with_submodule(tmp.path());
        let sm_path = parent.join("libs/sub");
        Command::new("git")
            .args(["remote", "remove", "origin"])
            .current_dir(&sm_path)
            .output()
            .unwrap();
        std::fs::write(sm_path.join("new-file"), "content").unwrap();
        Command::new("git")
            .args(["add", "."])
            .current_dir(&sm_path)
            .output()
            .unwrap();
        Command::new("git")
            .args(["commit", "-m", "ahead"])
            .current_dir(&sm_path)
            .output()
            .unwrap();
        let state = scan_repo_state(&parent).unwrap();
        assert_eq!(state.submodules[0].ahead_count, 1);
    }
}
