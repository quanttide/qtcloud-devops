use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct CommitHash(pub String);

impl std::fmt::Display for CommitHash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", &self.0[..self.0.len().min(7)])
    }
}

impl Default for CommitHash {
    fn default() -> Self {
        Self(String::from("0000000000000000000000000000000000000000"))
    }
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
    pub parent_pointer: CommitHash,
    pub local_head: CommitHash,
    pub remote_head: CommitHash,
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

impl RepoState {
    pub fn scan(root: &std::path::Path) -> Result<Self, Box<dyn std::error::Error>> {
        let gitmodules_path = root.join(".gitmodules");
        if !gitmodules_path.exists() {
            return Ok(RepoState {
                root_path: root.to_path_buf(),
                submodules: vec![],
                total: 0,
                clean_count: 0,
                needs_attention: vec![],
            });
        }

        let repo = match git2::Repository::open(root) {
            Ok(r) => r,
            Err(e) => return Err(format!("无法打开 Git 仓库 '{}': {}", root.display(), e).into()),
        };
        let mut submodules = Vec::new();

        let mut git_submodules = repo.submodules()?;
        git_submodules.sort_by(|a, b| a.name().cmp(&b.name()));

        for sm in &git_submodules {
            let name = sm.name().unwrap_or("unknown").to_string();
            let sm_path = sm.path();
            let full_sm_path = root.join(sm_path);
            let url = sm.url().unwrap_or("").to_string();
            let branch = sm.branch().unwrap_or("main").to_string();

            let raw_status = repo.submodule_status(&name, git2::SubmoduleIgnore::None)?;
            let is_uninitialized = raw_status.is_wd_uninitialized();
            let is_dirty = raw_status.is_wd_modified()
                || raw_status.is_index_modified()
                || raw_status.is_wd_untracked();

            // 父仓库记录的 commit
            let head_oid = sm.head_id().unwrap_or_else(git2::Oid::zero);
            let parent_pointer = CommitHash(head_oid.to_string());

            let (
                local_head,
                remote_head,
                is_detached,
                ahead_count,
                behind_count,
                is_orphaned,
                remote_unreachable,
            ) = if is_uninitialized {
                (
                    CommitHash::default(),
                    CommitHash::default(),
                    false,
                    0,
                    0,
                    false,
                    false,
                )
            } else {
                match git2::Repository::open(&full_sm_path) {
                    Ok(sub_repo) => {
                        let local = sub_repo
                            .head()
                            .ok()
                            .and_then(|r| r.target())
                            .map(|o| CommitHash(o.to_string()))
                            .unwrap_or_default();

                        let detached = sub_repo
                            .head()
                            .ok()
                            .map(|r| !r.is_branch())
                            .unwrap_or(false);

                        let (remote, unreachable) = sub_repo
                            .find_reference(&format!("refs/remotes/origin/{}", branch))
                            .ok()
                            .and_then(|r| r.target())
                            .map(|o| (CommitHash(o.to_string()), false))
                            .unwrap_or_else(|| (CommitHash::default(), true));

                        let ahead = count_between_opt(
                            &sub_repo,
                            parse_oid(&parent_pointer),
                            parse_oid(&local),
                        );
                        let behind = if unreachable {
                            0
                        } else {
                            count_between_opt(&sub_repo, parse_oid(&local), parse_oid(&remote))
                        };

                        let orphaned = if !unreachable
                            && remote != CommitHash::default()
                            && parent_pointer != remote
                        {
                            let p = parse_oid(&parent_pointer);
                            let r = parse_oid(&remote);
                            match (p, r) {
                                (Some(p_oid), Some(r_oid)) => sub_repo
                                    .merge_base(r_oid, p_oid)
                                    .map(|base| base != p_oid)
                                    .unwrap_or(false),
                                _ => false,
                            }
                        } else {
                            false
                        };

                        (
                            local,
                            remote,
                            detached,
                            ahead,
                            behind,
                            orphaned,
                            unreachable,
                        )
                    }
                    Err(_) => (
                        CommitHash::default(),
                        CommitHash::default(),
                        false,
                        0,
                        0,
                        false,
                        false,
                    ),
                }
            };

            let status = if is_uninitialized {
                SubmoduleStatus::Uninitialized
            } else if is_dirty {
                SubmoduleStatus::Dirty
            } else if is_detached {
                SubmoduleStatus::Detached
            } else if is_orphaned && !remote_unreachable {
                SubmoduleStatus::Orphaned
            } else if (remote_unreachable && local_head != parent_pointer)
                || (ahead_count > 0 && behind_count == 0)
            {
                SubmoduleStatus::AheadOfParent
            } else if behind_count > 0 && !remote_unreachable {
                SubmoduleStatus::BehindRemote
            } else {
                SubmoduleStatus::Clean
            };

            submodules.push(Submodule {
                name,
                path: sm_path.to_path_buf(),
                url,
                tracked_branch: branch,
                parent_pointer,
                local_head,
                remote_head,
                status,
                ahead_count,
                behind_count,
                remote_unreachable,
            });
        }

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

        Ok(RepoState {
            root_path: root.to_path_buf(),
            submodules,
            total,
            clean_count,
            needs_attention,
        })
    }

    pub fn scan_all(
        root: &std::path::Path,
    ) -> Result<(Vec<Submodule>, AggregateStatus), Box<dyn std::error::Error>> {
        let state = Self::scan(root)?;
        let agg = AggregateStatus::from_submodules(&state.submodules);
        Ok((state.submodules, agg))
    }
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

fn parse_oid(h: &CommitHash) -> Option<git2::Oid> {
    git2::Oid::from_str(&h.0).ok()
}

fn count_between_opt(
    repo: &git2::Repository,
    from: Option<git2::Oid>,
    to: Option<git2::Oid>,
) -> usize {
    let (Some(from), Some(to)) = (from, to) else {
        return 0;
    };
    if from == to {
        return 0;
    }
    let mut walk = match repo.revwalk() {
        Ok(w) => w,
        Err(_) => return 0,
    };
    if walk.push(to).is_err() || walk.hide(from).is_err() {
        return 0;
    }
    walk.count()
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---- SubmoduleStatus ----

    #[test]
    fn test_status_priority_ordering() {
        assert!(SubmoduleStatus::Dirty.priority() < SubmoduleStatus::Clean.priority());
        assert!(SubmoduleStatus::Orphaned.priority() < SubmoduleStatus::BehindRemote.priority());
        assert!(SubmoduleStatus::Detached.priority() < SubmoduleStatus::AheadOfParent.priority());
        assert!(SubmoduleStatus::Uninitialized.priority() < SubmoduleStatus::Clean.priority());
    }

    #[test]
    fn test_clean_is_lowest_priority() {
        let statuses = [
            SubmoduleStatus::Dirty,
            SubmoduleStatus::Orphaned,
            SubmoduleStatus::Detached,
            SubmoduleStatus::Uninitialized,
            SubmoduleStatus::BehindRemote,
            SubmoduleStatus::AheadOfParent,
        ];
        for s in &statuses {
            assert!(s.priority() < SubmoduleStatus::Clean.priority());
        }
    }

    #[test]
    fn test_all_priorities_are_unique() {
        let priorities: Vec<u8> = [
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
        let mut sorted = priorities.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(priorities.len(), sorted.len());
    }

    #[test]
    fn test_status_debug_output() {
        assert_eq!(format!("{:?}", SubmoduleStatus::Clean), "Clean");
        assert_eq!(format!("{:?}", SubmoduleStatus::Dirty), "Dirty");
        assert_eq!(format!("{:?}", SubmoduleStatus::Orphaned), "Orphaned");
        assert_eq!(format!("{:?}", SubmoduleStatus::Detached), "Detached");
        assert_eq!(
            format!("{:?}", SubmoduleStatus::Uninitialized),
            "Uninitialized"
        );
        assert_eq!(
            format!("{:?}", SubmoduleStatus::AheadOfParent),
            "AheadOfParent"
        );
        assert_eq!(
            format!("{:?}", SubmoduleStatus::BehindRemote),
            "BehindRemote"
        );
    }

    #[test]
    fn test_status_clone_eq() {
        let a = SubmoduleStatus::Dirty;
        let b = a.clone();
        assert_eq!(a, b);
    }

    // ---- CommitHash ----

    #[test]
    fn test_commit_hash_display_truncates() {
        let hash = CommitHash("abcdef1234567890".to_string());
        assert_eq!(hash.to_string(), "abcdef1");
    }

    #[test]
    fn test_commit_hash_display_short() {
        let hash = CommitHash("abc".to_string());
        assert_eq!(hash.to_string(), "abc");
    }

    #[test]
    fn test_commit_hash_display_empty() {
        let hash = CommitHash(String::new());
        assert_eq!(hash.to_string(), "");
    }

    #[test]
    fn test_commit_hash_equality() {
        let a = CommitHash("abc".to_string());
        let b = CommitHash("abc".to_string());
        let c = CommitHash("def".to_string());
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn test_commit_hash_default() {
        let d = CommitHash::default();
        assert_eq!(d.0, "0000000000000000000000000000000000000000");
        assert_eq!(d.to_string(), "0000000");
    }

    #[test]
    fn test_commit_hash_clone() {
        let a = CommitHash("deadbeef".to_string());
        let b = a.clone();
        assert_eq!(a, b);
    }

    // ---- Submodule ----

    #[test]
    fn test_submodule_builder() {
        let sm = Submodule {
            name: "test".into(),
            path: PathBuf::from("libs/test"),
            url: "https://example.com/test.git".into(),
            tracked_branch: "main".into(),
            parent_pointer: CommitHash("aaa".into()),
            local_head: CommitHash("bbb".into()),
            remote_head: CommitHash("ccc".into()),
            status: SubmoduleStatus::BehindRemote,
            ahead_count: 0,
            behind_count: 3,
            remote_unreachable: false,
        };
        assert_eq!(sm.name, "test");
        assert_eq!(sm.behind_count, 3);
        assert!(!sm.remote_unreachable);
    }

    // ---- AggregateStatus ----

    #[test]
    fn test_aggregate_status_default() {
        let agg = AggregateStatus::default();
        assert_eq!(agg.total, 0);
    }

    #[test]
    fn test_aggregate_status_from_submodules() {
        let sms = vec![
            Submodule {
                name: "a".into(),
                path: PathBuf::new(),
                url: String::new(),
                tracked_branch: "main".into(),
                parent_pointer: CommitHash::default(),
                local_head: CommitHash::default(),
                remote_head: CommitHash::default(),
                status: SubmoduleStatus::Clean,
                ahead_count: 0,
                behind_count: 0,
                remote_unreachable: false,
            },
            Submodule {
                name: "b".into(),
                path: PathBuf::new(),
                url: String::new(),
                tracked_branch: "main".into(),
                parent_pointer: CommitHash::default(),
                local_head: CommitHash::default(),
                remote_head: CommitHash::default(),
                status: SubmoduleStatus::Dirty,
                ahead_count: 0,
                behind_count: 0,
                remote_unreachable: false,
            },
            Submodule {
                name: "c".into(),
                path: PathBuf::new(),
                url: String::new(),
                tracked_branch: "main".into(),
                parent_pointer: CommitHash::default(),
                local_head: CommitHash::default(),
                remote_head: CommitHash::default(),
                status: SubmoduleStatus::Orphaned,
                ahead_count: 0,
                behind_count: 0,
                remote_unreachable: false,
            },
        ];
        let agg = AggregateStatus::from_submodules(&sms);
        assert_eq!(agg.total, 3);
        assert_eq!(agg.clean, 1);
        assert_eq!(agg.dirty, 1);
        assert_eq!(agg.orphaned, 1);
    }

    // ---- RepoState ----

    #[test]
    fn test_repo_state_empty() {
        let state = RepoState {
            root_path: PathBuf::from("/tmp"),
            submodules: vec![],
            total: 0,
            clean_count: 0,
            needs_attention: vec![],
        };
        assert_eq!(state.total, 0);
    }
}
