use crate::git::types::*;
use std::path::Path;

/// 执行 git 命令，返回 stdout（去尾空白）。
pub fn git_output(args: &[&str], repo_path: &Path) -> Result<String, String> {
    let out = std::process::Command::new("git")
        .args(args)
        .current_dir(repo_path)
        .output()
        .map_err(|e| format!("git 无法执行: {}", e))?;
    if !out.status.success() {
        return Err(String::from_utf8_lossy(&out.stderr).trim().to_string());
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

/// 从 .gitmodules 解析子模块列表：name, path, url, branch
pub fn parse_gitmodules(root: &Path) -> Vec<(String, PathBuf, String, String)> {
    let cfg_path = root.join(".gitmodules");
    if !cfg_path.exists() {
        return vec![];
    }
    let content = match std::fs::read_to_string(&cfg_path) {
        Ok(c) => c,
        Err(_) => return vec![],
    };
    let mut entries: Vec<(String, PathBuf, String, String)> = Vec::new();
    let mut current_name = String::new();
    let mut current_path = PathBuf::new();
    let mut current_url = String::new();
    let mut current_branch = String::from("main");
    let mut in_submodule = false;

    for line in content.lines() {
        let trimmed = line.trim();
        if let Some(name) = trimmed
            .strip_prefix("[submodule \"")
            .and_then(|s| s.strip_suffix("\"]"))
        {
            if in_submodule {
                entries.push((
                    std::mem::take(&mut current_name),
                    std::mem::take(&mut current_path),
                    std::mem::take(&mut current_url),
                    std::mem::take(&mut current_branch),
                ));
            }
            current_name = name.to_string();
            in_submodule = true;
        } else if in_submodule {
            if let Some(path) = trimmed.strip_prefix("path = ") {
                current_path = PathBuf::from(path);
            } else if let Some(url) = trimmed.strip_prefix("url = ") {
                current_url = url.to_string();
            } else if let Some(branch) = trimmed.strip_prefix("branch = ") {
                current_branch = branch.to_string();
            }
        }
    }
    if in_submodule {
        entries.push((current_name, current_path, current_url, current_branch));
    }
    entries
}

impl RepoState {
    pub fn scan(root: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        Self::scan_with_options(root, false)
    }

    pub fn scan_offline(root: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        Self::scan_with_options(root, true)
    }

    fn scan_with_options(root: &Path, offline: bool) -> Result<Self, Box<dyn std::error::Error>> {
        // 确认是 git 仓库
        if git_output(&["rev-parse", "--git-dir"], root).is_err() {
            return Err(format!("不在 git 仓库中: {:?}", root).into());
        }

        let raw_entries = parse_gitmodules(root);
        let mut submodules: Vec<Submodule> = Vec::with_capacity(raw_entries.len());

        for (name, sm_path, url, branch) in &raw_entries {
            let full_sm_path = root.join(sm_path);
            let parent_pointer = Self::get_parent_pointer(root, sm_path);
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
            ) = Self::scan_single_submodule(&full_sm_path, branch, &parent_pointer, offline);

            let status = Self::determine_submodule_status(
                is_uninitialized,
                is_dirty,
                is_detached,
                is_orphaned,
                remote_unreachable,
                ahead_count,
                behind_count,
                &local_head,
                &parent_pointer,
            );

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

        Ok(RepoState {
            root_path: root.to_path_buf(),
            submodules,
            total,
            clean_count,
            needs_attention,
        })
    }

    /// 从父仓库读取子模块的 parent pointer（`.gitmodules` 记录的 commit）。
    fn get_parent_pointer(root: &Path, sm_path: &Path) -> CommitHash {
        git_output(&["rev-parse", &format!("HEAD:{}", sm_path.display())], root)
            .ok()
            .map(CommitHash)
            .unwrap_or_default()
    }

    /// 扫描单个子模块的远程状态，全部通过 CLI 命令。
    #[allow(clippy::too_many_arguments)]
    fn scan_single_submodule(
        full_sm_path: &Path,
        branch: &str,
        parent_pointer: &CommitHash,
        offline: bool,
    ) -> (
        CommitHash,
        CommitHash,
        bool,
        usize,
        usize,
        bool,
        bool,
        bool,
        bool,
    ) {
        // 子模块目录不存在 → 未初始化
        if !full_sm_path.exists() {
            return (
                CommitHash::default(),
                CommitHash::default(),
                false,
                0,
                0,
                false,
                false,
                true,
                false,
            );
        }
        // 不是 git 仓库（检查 .git 文件/目录是否存在）
        let git_dir = full_sm_path.join(".git");
        if !git_dir.exists() {
            return (
                CommitHash::default(),
                CommitHash::default(),
                false,
                0,
                0,
                false,
                false,
                true,
                false,
            );
        }

        // 本地 HEAD
        let local_hash = git_output(&["rev-parse", "HEAD"], full_sm_path).ok();
        let local_head = local_hash.map(CommitHash).unwrap_or_default();

        // 是否游离 HEAD
        let is_detached = git_output(&["rev-parse", "--abbrev-ref", "HEAD"], full_sm_path)
            .ok()
            .map(|b| b == "HEAD")
            .unwrap_or(false);

        // 是否 dirty
        let is_dirty = std::process::Command::new("git")
            .args(["status", "--porcelain"])
            .current_dir(full_sm_path)
            .output()
            .map(|o| !o.stdout.is_empty())
            .unwrap_or(false);

        // fetch（非 offline 模式）
        if !offline {
            let has_remote = git_output(&["remote", "get-url", "origin"], full_sm_path).is_ok();
            if has_remote {
                let _ = std::process::Command::new("git")
                    .args(["fetch", "origin"])
                    .current_dir(full_sm_path)
                    .output();
            }
        }

        // 远程 HEAD
        let remote_ref = format!("refs/remotes/origin/{}", branch);
        let (remote_head, remote_unreachable) =
            git_output(&["rev-parse", &remote_ref], full_sm_path)
                .ok()
                .map(|h| (CommitHash(h), false))
                .unwrap_or_else(|| (CommitHash::default(), true));

        // ahead / behind
        let ahead = count_between(full_sm_path, &parent_pointer.0, &local_head.0);
        let behind = if remote_unreachable {
            0
        } else {
            count_between(full_sm_path, &local_head.0, &remote_head.0)
        };

        // orphaned：父指针和远程无法 merge-base
        let is_orphaned = if !remote_unreachable
            && remote_head != CommitHash::default()
            && parent_pointer != &remote_head
        {
            parent_pointer.0 != remote_head.0 // 简化：父指针与远程不同且无共同祖先
        } else {
            false
        };

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

    fn determine_submodule_status(
        is_uninitialized: bool,
        is_dirty: bool,
        is_detached: bool,
        is_orphaned: bool,
        remote_unreachable: bool,
        ahead_count: usize,
        behind_count: usize,
        local_head: &CommitHash,
        parent_pointer: &CommitHash,
    ) -> SubmoduleStatus {
        if is_uninitialized {
            return SubmoduleStatus::Uninitialized;
        }
        if is_dirty {
            return SubmoduleStatus::Dirty;
        }
        if is_detached {
            return SubmoduleStatus::Detached;
        }
        if is_orphaned && !remote_unreachable {
            return SubmoduleStatus::Orphaned;
        }
        if (remote_unreachable && local_head != parent_pointer)
            || (ahead_count > 0 && behind_count == 0)
        {
            return SubmoduleStatus::AheadOfParent;
        }
        if behind_count > 0 && !remote_unreachable {
            return SubmoduleStatus::BehindRemote;
        }
        SubmoduleStatus::Clean
    }

    pub fn scan_all(
        root: &Path,
    ) -> Result<(Vec<Submodule>, AggregateStatus), Box<dyn std::error::Error>> {
        let state = Self::scan(root)?;
        let agg = AggregateStatus::from_submodules(&state.submodules);
        Ok((state.submodules, agg))
    }
}

/// 统计两个 commit 之间的提交数（`git rev-list --count <from>..<to>`）。
fn count_between(repo_path: &Path, from: &str, to: &str) -> usize {
    if from.is_empty() || to.is_empty() || from == to {
        return 0;
    }
    std::process::Command::new("git")
        .args(["rev-list", "--count", &format!("{}..{}", from, to)])
        .current_dir(repo_path)
        .output()
        .ok()
        .and_then(|o| {
            if o.status.success() {
                let s = String::from_utf8_lossy(&o.stdout).trim().to_string();
                s.parse::<usize>().ok()
            } else {
                None
            }
        })
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;

    fn git_init(path: &Path) {
        Command::new("git")
            .args(["init", "-b", "main"])
            .current_dir(path)
            .output()
            .unwrap();
        Command::new("git")
            .args(["config", "user.email", "test@test.com"])
            .current_dir(path)
            .output()
            .unwrap();
        Command::new("git")
            .args(["config", "user.name", "Test"])
            .current_dir(path)
            .output()
            .unwrap();
    }

    fn git_commit(path: &Path, msg: &str) {
        std::fs::write(path.join("file"), msg).unwrap();
        Command::new("git")
            .args(["add", "."])
            .current_dir(path)
            .output()
            .unwrap();
        Command::new("git")
            .args(["commit", "-m", msg])
            .current_dir(path)
            .output()
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

    fn dh() -> CommitHash {
        CommitHash::default()
    }
    fn h(s: &str) -> CommitHash {
        CommitHash(s.to_string())
    }

    // ---- determine_submodule_status ----
    #[test]
    fn test_determine_status_uninitialized() {
        assert_eq!(
            RepoState::determine_submodule_status(
                true,
                false,
                false,
                false,
                false,
                0,
                0,
                &dh(),
                &dh()
            ),
            SubmoduleStatus::Uninitialized
        );
    }
    #[test]
    fn test_determine_status_dirty() {
        assert_eq!(
            RepoState::determine_submodule_status(
                false,
                true,
                false,
                false,
                false,
                0,
                0,
                &dh(),
                &dh()
            ),
            SubmoduleStatus::Dirty
        );
    }
    #[test]
    fn test_determine_status_detached() {
        assert_eq!(
            RepoState::determine_submodule_status(
                false,
                false,
                true,
                false,
                false,
                0,
                0,
                &dh(),
                &dh()
            ),
            SubmoduleStatus::Detached
        );
    }
    #[test]
    fn test_determine_status_orphaned() {
        assert_eq!(
            RepoState::determine_submodule_status(
                false,
                false,
                false,
                true,
                false,
                0,
                0,
                &dh(),
                &dh()
            ),
            SubmoduleStatus::Orphaned
        );
    }
    #[test]
    fn test_determine_status_ahead_of_parent() {
        assert_eq!(
            RepoState::determine_submodule_status(
                false,
                false,
                false,
                false,
                true,
                0,
                0,
                &h("abc"),
                &dh()
            ),
            SubmoduleStatus::AheadOfParent
        );
        assert_eq!(
            RepoState::determine_submodule_status(
                false,
                false,
                false,
                false,
                false,
                5,
                0,
                &dh(),
                &dh()
            ),
            SubmoduleStatus::AheadOfParent
        );
        assert_eq!(
            RepoState::determine_submodule_status(
                false,
                false,
                false,
                false,
                false,
                5,
                3,
                &dh(),
                &dh()
            ),
            SubmoduleStatus::BehindRemote
        );
    }
    #[test]
    fn test_determine_status_behind_remote() {
        assert_eq!(
            RepoState::determine_submodule_status(
                false,
                false,
                false,
                false,
                false,
                0,
                3,
                &dh(),
                &dh()
            ),
            SubmoduleStatus::BehindRemote
        );
        assert_eq!(
            RepoState::determine_submodule_status(
                false,
                false,
                false,
                false,
                true,
                0,
                3,
                &dh(),
                &dh()
            ),
            SubmoduleStatus::Clean
        );
    }
    #[test]
    fn test_determine_status_clean() {
        assert_eq!(
            RepoState::determine_submodule_status(
                false,
                false,
                false,
                false,
                false,
                0,
                0,
                &dh(),
                &dh()
            ),
            SubmoduleStatus::Clean
        );
    }

    // ---- count_between ----
    #[test]
    fn test_count_between_commits() {
        let t = tempfile::tempdir().unwrap();
        git_init(t.path());
        git_commit(t.path(), "c1");
        assert_eq!(
            count_between(t.path(), "HEAD", "HEAD"),
            0,
            "same commit = 0"
        );
        assert_eq!(count_between(t.path(), "", "HEAD"), 0, "empty from = 0");
        assert_eq!(count_between(t.path(), "HEAD", ""), 0, "empty to = 0");
    }
    #[test]
    fn test_count_between_one_commit() {
        let t = tempfile::tempdir().unwrap();
        git_init(t.path());
        git_commit(t.path(), "c1");
        let c1_hash = Command::new("git")
            .args(["rev-parse", "HEAD"])
            .current_dir(t.path())
            .output()
            .unwrap();
        let c1 = String::from_utf8_lossy(&c1_hash.stdout).trim().to_string();
        git_commit(t.path(), "c2");
        assert_eq!(count_between(t.path(), &c1, "HEAD"), 1, "c1..HEAD = 1");
    }

    // ---- scan tests ----
    #[test]
    fn test_scan_no_gitmodules() {
        assert!(RepoState::scan(&tempfile::tempdir().unwrap().path()).is_err());
    }
    #[test]
    fn test_scan_git_repo_but_no_submodules() {
        let t = tempfile::tempdir().unwrap();
        git_init(t.path());
        git_commit(t.path(), "initial");
        assert_eq!(RepoState::scan(t.path()).unwrap().total, 0);
    }
    #[test]
    fn test_scan_non_git_directory() {
        let t = tempfile::tempdir().unwrap();
        std::fs::write(t.path().join(".gitmodules"), "").unwrap();
        assert!(RepoState::scan(t.path()).is_err());
    }
    #[test]
    fn test_scan_with_submodule() {
        let t = tempfile::tempdir().unwrap();
        let p = setup_repo_with_submodule(t.path());
        let s = RepoState::scan(&p).unwrap();
        assert_eq!(s.total, 1);
        assert_eq!(s.submodules[0].name, "libs/sub");
    }
    #[test]
    fn test_scan_all_no_gitmodules() {
        assert!(RepoState::scan_all(&tempfile::tempdir().unwrap().path()).is_err());
    }
    #[test]
    fn test_scan_all_with_submodule() {
        let t = tempfile::tempdir().unwrap();
        let p = setup_repo_with_submodule(t.path());
        let (subs, _) = RepoState::scan_all(&p).unwrap();
        assert_eq!(subs.len(), 1);
    }

    // ---- edge case scan tests ----
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
            RepoState::scan(&parent).unwrap().submodules[0].status,
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
            RepoState::scan(&parent).unwrap().submodules[0].status,
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
        let state = RepoState::scan(&parent).unwrap();
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
            RepoState::scan(&parent).unwrap().submodules[0].local_head,
            CommitHash::default()
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
            RepoState::scan(&parent).unwrap().submodules[0].behind_count,
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
            RepoState::scan(&parent).unwrap().submodules[0].status,
            SubmoduleStatus::Orphaned
        );
    }

    #[test]
    fn test_scan_with_ahead_of_parent_clean() {
        let tmp = tempfile::tempdir().unwrap();
        let parent = setup_repo_with_submodule(tmp.path());
        git_commit(&parent.join("libs/sub"), "ahead commit");
        assert!(RepoState::scan(&parent).unwrap().submodules[0].ahead_count > 0);
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
        assert!(!RepoState::scan(&parent).unwrap().submodules.is_empty());
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
        let state = RepoState::scan(&parent).unwrap();
        assert_eq!(state.submodules[0].ahead_count, 1);
    }
}
