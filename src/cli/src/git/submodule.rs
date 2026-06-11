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
        Self::scan_with_options(root, false)
    }

    pub fn scan_offline(root: &std::path::Path) -> Result<Self, Box<dyn std::error::Error>> {
        Self::scan_with_options(root, true)
    }

    fn scan_with_options(root: &std::path::Path, offline: bool) -> Result<Self, Box<dyn std::error::Error>> {
        let repo = match git2::Repository::open(root) {
            Ok(r) => r,
            Err(e) => return Err(format!("无法打开 Git 仓库 '{}': {}", root.display(), e).into()),
        };
        let gitmodules_path = root.join(".gitmodules");

        let submodules = if gitmodules_path.exists() {
            let mut git_submodules = repo.submodules()?;
            git_submodules.sort_by(|a, b| a.name().cmp(&b.name()));
            git_submodules
                .iter()
                .map(|sm| Self::scan_single_submodule(root, sm, &repo, offline))
                .collect::<Result<Vec<_>, _>>()?
        } else {
            Vec::new()
        };

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

    fn scan_single_submodule(
        root: &std::path::Path,
        sm: &git2::Submodule,
        repo: &git2::Repository,
        offline: bool,
    ) -> Result<Submodule, Box<dyn std::error::Error>> {
        let name = sm.name().unwrap_or("unknown").to_string();
        let sm_path = sm.path();
        let full_sm_path = root.join(sm_path);
        let url = sm.url().unwrap_or("").to_string();
        let branch = sm.branch().unwrap_or("main").to_string();

        let raw_status = repo.submodule_status(&name, git2::SubmoduleIgnore::None)?;
        let is_uninitialized = raw_status.is_wd_uninitialized();
        let head_oid = sm.head_id().unwrap_or_else(git2::Oid::zero);
        let parent_pointer = CommitHash(head_oid.to_string());

        let (local_head, remote_head, is_detached, ahead_count, behind_count, is_orphaned, remote_unreachable) =
            Self::scan_submodule_remote_state(&full_sm_path, &branch, &parent_pointer, is_uninitialized, offline);

        let is_dirty = !is_uninitialized
            && ahead_count == 0
            && (raw_status.is_wd_modified()
                || raw_status.is_index_modified()
                || raw_status.is_wd_untracked());

        let status = Self::determine_submodule_status(
            is_uninitialized, is_dirty, is_detached, is_orphaned,
            remote_unreachable, ahead_count, behind_count, &local_head, &parent_pointer,
        );

        Ok(Submodule {
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
        })
    }

    fn scan_submodule_remote_state(
        full_sm_path: &std::path::Path, branch: &str, parent_pointer: &CommitHash, is_uninitialized: bool, offline: bool,
    ) -> (CommitHash, CommitHash, bool, usize, usize, bool, bool) {
        if is_uninitialized {
            return Self::default_submodule_state();
        }
        let Ok(sub_repo) = git2::Repository::open(full_sm_path) else {
            return Self::default_submodule_state();
        };
        let (local, detached) = Self::open_submodule_head(&sub_repo);
        if !offline {
            Self::fetch_submodule_remote(&sub_repo);
        }
        let (remote, unreachable) = Self::resolve_submodule_remote(&sub_repo, branch);
        let (ahead, behind, orphaned) = Self::compute_submodule_diff(&sub_repo, &local, parent_pointer, &remote, unreachable);
        (local, remote, detached, ahead, behind, orphaned, unreachable)
    }

    fn default_submodule_state() -> (CommitHash, CommitHash, bool, usize, usize, bool, bool) {
        (CommitHash::default(), CommitHash::default(), false, 0, 0, false, false)
    }

    fn open_submodule_head(sub_repo: &git2::Repository) -> (CommitHash, bool) {
        let local = sub_repo.head().ok().and_then(|r| r.target()).map(|o| CommitHash(o.to_string())).unwrap_or_default();
        let detached = sub_repo.head().ok().map(|r| !r.is_branch()).unwrap_or(false);
        (local, detached)
    }

    fn fetch_submodule_remote(sub_repo: &git2::Repository) {
        if let Ok(mut sub_remote) = sub_repo.find_remote("origin") {
            let mut fetch_opts = git2::FetchOptions::new();
            fetch_opts.download_tags(git2::AutotagOption::None);
            let mut callbacks = git2::RemoteCallbacks::new();
            callbacks.transfer_progress(|_| true);
            fetch_opts.remote_callbacks(callbacks);
            let _ = sub_remote.fetch(&["+refs/heads/*:refs/remotes/origin/*"], Some(&mut fetch_opts), None);
        }
    }

    fn resolve_submodule_remote(sub_repo: &git2::Repository, branch: &str) -> (CommitHash, bool) {
        sub_repo.find_reference(&format!("refs/remotes/origin/{}", branch)).ok()
            .and_then(|r| r.target())
            .map(|o| (CommitHash(o.to_string()), false))
            .unwrap_or_else(|| (CommitHash::default(), true))
    }

    fn compute_submodule_diff(
        sub_repo: &git2::Repository, local: &CommitHash, parent_pointer: &CommitHash,
        remote: &CommitHash, unreachable: bool,
    ) -> (usize, usize, bool) {
        let ahead = count_between_opt(sub_repo, parse_oid(parent_pointer), parse_oid(local));
        let behind = if unreachable { 0 } else { count_between_opt(sub_repo, parse_oid(local), parse_oid(remote)) };
        let orphaned = if !unreachable && remote != &CommitHash::default() && parent_pointer != remote {
            let (p, r) = (parse_oid(parent_pointer), parse_oid(remote));
            match (p, r) {
                (Some(p_oid), Some(r_oid)) => sub_repo.merge_base(r_oid, p_oid).map(|base| base != p_oid).unwrap_or(true),
                _ => false,
            }
        } else { false };
        (ahead, behind, orphaned)
    }

    fn determine_submodule_status(
        is_uninitialized: bool, is_dirty: bool, is_detached: bool, is_orphaned: bool,
        remote_unreachable: bool, ahead_count: usize, behind_count: usize,
        local_head: &CommitHash, parent_pointer: &CommitHash,
    ) -> SubmoduleStatus {
        if is_uninitialized { return SubmoduleStatus::Uninitialized; }
        if is_dirty { return SubmoduleStatus::Dirty; }
        if is_detached { return SubmoduleStatus::Detached; }
        if is_orphaned && !remote_unreachable { return SubmoduleStatus::Orphaned; }
        if (remote_unreachable && local_head != parent_pointer) || (ahead_count > 0 && behind_count == 0) {
            return SubmoduleStatus::AheadOfParent;
        }
        if behind_count > 0 && !remote_unreachable { return SubmoduleStatus::BehindRemote; }
        SubmoduleStatus::Clean
    }

    pub fn scan_all(root: &std::path::Path) -> Result<(Vec<Submodule>, AggregateStatus), Box<dyn std::error::Error>> {
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
        let mut clean = 0; let mut ahead = 0; let mut behind = 0;
        let mut detached = 0; let mut dirty = 0; let mut orphaned = 0; let mut uninit = 0;
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
        AggregateStatus { total: submodules.len(), clean, ahead_of_parent: ahead, behind_remote: behind, detached, dirty, orphaned, uninitialized: uninit }
    }
}

// ===== git operations =====

pub struct GitSubmoduleEditor {
    root: PathBuf,
    offline: bool,
}

impl GitSubmoduleEditor {
    pub fn new(root: PathBuf) -> Self {
        Self { root, offline: false }
    }

    pub fn set_offline(&mut self, offline: bool) {
        self.offline = offline;
    }

    pub fn fetch_submodule(path: &std::path::Path) -> Result<(), ()> {
        let has_remote = std::process::Command::new("git")
            .args(["remote", "get-url", "origin"]).current_dir(path).output()
            .map(|o| o.status.success()).unwrap_or(false);
        if !has_remote { return Ok(()); }
        std::process::Command::new("git").args(["fetch", "origin"]).current_dir(path).output()
            .map(|o| if o.status.success() { Ok(()) } else { Err(()) }).unwrap_or(Err(()))
    }

    pub fn rebase_submodule(path: &std::path::Path) -> Result<(), String> {
        if !path.exists() { return Ok(()); }
        let branch = std::process::Command::new("git").args(["rev-parse", "--abbrev-ref", "HEAD"])
            .current_dir(path).output().ok()
            .and_then(|o| if o.status.success() { Some(String::from_utf8_lossy(&o.stdout).trim().to_string()) } else { None })
            .unwrap_or_default();
        if branch.is_empty() || branch == "HEAD" { return Ok(()); }
        if !std::process::Command::new("git").args(["remote", "get-url", "origin"])
            .current_dir(path).output().map(|o| o.status.success()).unwrap_or(false) { return Ok(()); }
        let output = std::process::Command::new("git")
            .args(["rebase", &format!("origin/{}", branch)])
            .current_dir(path).output()
            .map_err(|e| format!("git rebase 无法执行: {}", e))?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            if stderr.contains("up to date") || stderr.contains("up-to-date") {
                return Ok(());
            }
            return Err(format!("rebase 冲突，需手动处理：解决冲突后 git rebase --continue，或 git rebase --abort 放弃\n{}", stderr));
        }
        Ok(())
    }

    pub fn push_submodule(path: &std::path::Path) -> Result<(), String> {
        if !path.exists() { return Ok(()); }
        let branch = std::process::Command::new("git").args(["rev-parse", "--abbrev-ref", "HEAD"])
            .current_dir(path).output().ok()
            .and_then(|o| if o.status.success() { Some(String::from_utf8_lossy(&o.stdout).trim().to_string()) } else { None })
            .unwrap_or_default();
        if branch.is_empty() || branch == "HEAD" { return Ok(()); }
        if !std::process::Command::new("git").args(["remote", "get-url", "origin"])
            .current_dir(path).output().map(|o| o.status.success()).unwrap_or(false) { return Ok(()); }
        let tracking = format!("origin/{}", branch);
        let ahead = std::process::Command::new("git").args(["rev-list", "--count", &format!("{}..{}", tracking, branch)])
            .current_dir(path).output().ok()
            .and_then(|o| if o.status.success() { String::from_utf8_lossy(&o.stdout).trim().parse::<i32>().ok() } else { None })
            .unwrap_or(0);
        if ahead <= 0 { return Ok(()); }
        std::process::Command::new("git").args(["push", "origin", &branch]).current_dir(path).output()
            .map(|o| if o.status.success() { Ok(()) } else { Err(String::from_utf8_lossy(&o.stderr).trim().to_string()) })
            .unwrap_or_else(|e| Err(format!("git push 无法执行: {}", e)))
    }

    pub fn update_parent_pointer(repo: &git2::Repository, sm_path: &std::path::Path, name: &str) -> Result<(), Box<dyn std::error::Error>> {
        let mut index = repo.index()?;
        index.add_path(sm_path)?; index.write()?;
        let tree_id = index.write_tree()?;
        let tree = repo.find_tree(tree_id)?;
        let head = repo.head()?;
        let parent = head.peel_to_commit()?;
        let signature = repo.signature()?;
        repo.commit(Some("HEAD"), &signature, &signature, &format!("chore: 更新子模块 '{}' 指针", name), &tree, &[&parent])?;
        Ok(())
    }

    pub fn push_parent(repo: &git2::Repository, root: &std::path::Path) -> Result<(), String> {
        if !std::process::Command::new("git").args(["remote", "get-url", "origin"])
            .current_dir(root).output().map(|o| o.status.success()).unwrap_or(false) { return Ok(()); }
        let branch = repo.head().ok().and_then(|r| r.shorthand().map(|s| s.to_string())).unwrap_or_default();
        if branch.is_empty() { return Err("无法检测当前分支".into()); }
        std::process::Command::new("git").args(["push", "origin", &branch]).current_dir(root).output()
            .map(|o| if o.status.success() { Ok(()) } else { Err(String::from_utf8_lossy(&o.stderr).trim().to_string()) })
            .unwrap_or_else(|e| Err(format!("git push 无法执行: {}", e)))
    }

    pub fn revert_parent_commit(root: &std::path::Path) {
        std::process::Command::new("git").args(["reset", "--hard", "HEAD~1"]).current_dir(root).output().ok();
    }

    pub fn root(&self) -> &std::path::Path {
        &self.root
    }

    pub fn sync_to_parent(&self, name: &str) -> Result<(), Box<dyn std::error::Error>> {
        let repo = git2::Repository::open(&self.root)?;
        let sm = repo.find_submodule(name)?;
        let sm_path = sm.path();
        let full_sm_path = self.root.join(sm_path);

        if full_sm_path.exists() {
            Self::fetch_submodule(&full_sm_path).ok();
            Self::rebase_submodule(&full_sm_path)?;
        }
        Self::push_submodule(&full_sm_path).map_err(|e| format!("子模块 push 失败: {}", e))?;
        Self::update_parent_pointer(&repo, sm_path, name)?;
        if let Err(e) = Self::push_parent(&repo, &self.root) {
            Self::revert_parent_commit(&self.root);
            return Err(format!("父仓库 push 失败 (已回滚提交): {}", e).into());
        }
        println!("  ✓ {}", name);
        Ok(())
    }

    pub fn sync_all_to_parent(&self) -> Result<(), Box<dyn std::error::Error>> {
        let repo = git2::Repository::open(&self.root)?;
        let submodules = repo.submodules()?;
        println!("同步 {} 个子模块", submodules.len());
        for sm in submodules.iter() {
            let name = sm.name().unwrap_or("unknown").to_string();
            match self.sync_to_parent(&name) {
                Ok(()) => {}
                Err(e) => println!("  {:<35} ✗ 失败: {}", name, e),
            }
        }
        Ok(())
    }

    pub fn status(&self) -> Result<Vec<HealthIssue>, Box<dyn std::error::Error>> {
        let state = RepoState::scan(&self.root)?;
        let mut issues = Vec::new();
        for sm in &state.submodules {
            if sm.status != SubmoduleStatus::Clean {
                let (description, action) = describe_issue(&sm.status);
                issues.push(HealthIssue {
                    submodule_name: sm.name.clone(),
                    status: format!("{:?}", sm.status),
                    description,
                    suggested_action: action,
                });
            }
        }
        Ok(issues)
    }
}

fn parse_oid(h: &CommitHash) -> Option<git2::Oid> {
    git2::Oid::from_str(&h.0).ok()
}

fn count_between_opt(repo: &git2::Repository, from: Option<git2::Oid>, to: Option<git2::Oid>) -> usize {
    let (Some(from), Some(to)) = (from, to) else { return 0; };
    if from == to { return 0; }
    let mut walk = match repo.revwalk() { Ok(w) => w, Err(_) => return 0, };
    if walk.push(to).is_err() || walk.hide(from).is_err() { return 0; }
    walk.count()
}

#[derive(Debug, Clone)]
pub struct HealthIssue {
    pub submodule_name: String,
    pub status: String,
    pub description: String,
    pub suggested_action: String,
}

fn describe_issue(status: &SubmoduleStatus) -> (String, String) {
    match status {
        SubmoduleStatus::AheadOfParent => ("本地领先于父仓库记录".into(), "运行 sync_to_parent 更新父仓库指针".into()),
        SubmoduleStatus::BehindRemote => ("远程有更新，本地落后".into(), "运行 code sync 获取最新代码".into()),
        SubmoduleStatus::Detached => ("处于游离 HEAD 状态".into(), "运行 checkout_branch 切换到跟踪分支".into()),
        SubmoduleStatus::Dirty => ("有未提交的修改".into(), "提交或 stash 当前修改".into()),
        SubmoduleStatus::Orphaned => ("父仓库记录的 commit 在远程已不存在".into(), "需手动干预".into()),
        SubmoduleStatus::Uninitialized => ("尚未初始化".into(), "运行 init 初始化子模块".into()),
        SubmoduleStatus::Clean => unreachable!(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;

    fn git_init(path: &std::path::Path) {
        Command::new("git").args(["init", "-b", "main"]).current_dir(path).output().unwrap();
        Command::new("git").args(["config", "user.email", "test@test.com"]).current_dir(path).output().unwrap();
        Command::new("git").args(["config", "user.name", "Test"]).current_dir(path).output().unwrap();
    }

    fn git_commit(path: &std::path::Path, msg: &str) {
        std::fs::write(path.join("file"), msg).unwrap();
        Command::new("git").args(["add", "."]).current_dir(path).output().unwrap();
        Command::new("git").args(["commit", "-m", msg]).current_dir(path).output().unwrap();
    }

    fn setup_repo_with_submodule(tmp: &std::path::Path) -> std::path::PathBuf {
        let parent = tmp.join("parent");
        let sub = tmp.join("sub");
        std::fs::create_dir_all(&sub).unwrap(); git_init(&sub); git_commit(&sub, "init sub");
        std::fs::create_dir_all(&parent).unwrap(); git_init(&parent); git_commit(&parent, "init parent");
        Command::new("git").args(["submodule", "add", &sub.to_string_lossy(), "libs/sub"]).current_dir(&parent).output().unwrap();
        Command::new("git").args(["commit", "-m", "add submodule"]).current_dir(&parent).output().unwrap();
        parent
    }

    // ---- SubmoduleStatus tests ----
    #[test] fn test_status_priority_ordering() {
        assert!(SubmoduleStatus::Dirty.priority() < SubmoduleStatus::Clean.priority());
        assert!(SubmoduleStatus::Orphaned.priority() < SubmoduleStatus::BehindRemote.priority());
    }
    #[test] fn test_clean_is_lowest_priority() {
        for s in &[SubmoduleStatus::Dirty, SubmoduleStatus::Orphaned, SubmoduleStatus::Detached, SubmoduleStatus::Uninitialized, SubmoduleStatus::BehindRemote, SubmoduleStatus::AheadOfParent] {
            assert!(s.priority() < SubmoduleStatus::Clean.priority());
        }
    }
    #[test] fn test_all_priorities_are_unique() {
        let p: Vec<u8> = [SubmoduleStatus::Dirty, SubmoduleStatus::Orphaned, SubmoduleStatus::Detached, SubmoduleStatus::Uninitialized, SubmoduleStatus::BehindRemote, SubmoduleStatus::AheadOfParent, SubmoduleStatus::Clean].iter().map(|s| s.priority()).collect();
        let mut s = p.clone(); s.sort(); s.dedup();
        assert_eq!(p.len(), s.len());
    }
    #[test] fn test_status_debug_output() { assert_eq!(format!("{:?}", SubmoduleStatus::Clean), "Clean"); }
    #[test] fn test_status_clone_eq() { assert_eq!(SubmoduleStatus::Dirty, SubmoduleStatus::Dirty); }

    // ---- determine_submodule_status ----
    fn dh() -> CommitHash { CommitHash::default() }
    fn h(s: &str) -> CommitHash { CommitHash(s.to_string()) }
    #[test] fn test_determine_status_uninitialized() { assert_eq!(RepoState::determine_submodule_status(true, false, false, false, false, 0, 0, &dh(), &dh()), SubmoduleStatus::Uninitialized); }
    #[test] fn test_determine_status_dirty() { assert_eq!(RepoState::determine_submodule_status(false, true, false, false, false, 0, 0, &dh(), &dh()), SubmoduleStatus::Dirty); }
    #[test] fn test_determine_status_detached() { assert_eq!(RepoState::determine_submodule_status(false, false, true, false, false, 0, 0, &dh(), &dh()), SubmoduleStatus::Detached); }
    #[test] fn test_determine_status_orphaned() { assert_eq!(RepoState::determine_submodule_status(false, false, false, true, false, 0, 0, &dh(), &dh()), SubmoduleStatus::Orphaned); }
    #[test] fn test_determine_status_ahead_of_parent() {
        assert_eq!(RepoState::determine_submodule_status(false, false, false, false, true, 0, 0, &h("abc"), &dh()), SubmoduleStatus::AheadOfParent);
        assert_eq!(RepoState::determine_submodule_status(false, false, false, false, false, 5, 0, &dh(), &dh()), SubmoduleStatus::AheadOfParent);
        assert_eq!(RepoState::determine_submodule_status(false, false, false, false, false, 5, 3, &dh(), &dh()), SubmoduleStatus::BehindRemote);
    }
    #[test] fn test_determine_status_behind_remote() {
        assert_eq!(RepoState::determine_submodule_status(false, false, false, false, false, 0, 3, &dh(), &dh()), SubmoduleStatus::BehindRemote);
        assert_eq!(RepoState::determine_submodule_status(false, false, false, false, true, 0, 3, &dh(), &dh()), SubmoduleStatus::Clean);
    }
    #[test] fn test_determine_status_clean() { assert_eq!(RepoState::determine_submodule_status(false, false, false, false, false, 0, 0, &dh(), &dh()), SubmoduleStatus::Clean); }

    // ---- CommitHash ----
    #[test] fn test_commit_hash_display_truncates() { assert_eq!(CommitHash("abcdef1234567890".to_string()).to_string(), "abcdef1"); }
    #[test] fn test_commit_hash_display_short() { assert_eq!(CommitHash("abc".to_string()).to_string(), "abc"); }
    #[test] fn test_commit_hash_display_empty() { assert_eq!(CommitHash(String::new()).to_string(), ""); }
    #[test] fn test_commit_hash_equality() { assert_eq!(CommitHash("abc".to_string()), CommitHash("abc".to_string())); }
    #[test] fn test_commit_hash_default() { assert_eq!(CommitHash::default().0, "0000000000000000000000000000000000000000"); }
    #[test] fn test_commit_hash_clone() { let a = CommitHash("deadbeef".to_string()); assert_eq!(a, a.clone()); }

    // ---- Submodule ----
    #[test] fn test_submodule_builder() {
        let sm = Submodule { name: "test".into(), path: PathBuf::from("libs/test"), url: "https://example.com/test.git".into(), tracked_branch: "main".into(), parent_pointer: CommitHash("aaa".into()), local_head: CommitHash("bbb".into()), remote_head: CommitHash("ccc".into()), status: SubmoduleStatus::BehindRemote, ahead_count: 0, behind_count: 3, remote_unreachable: false };
        assert_eq!(sm.name, "test");
    }

    // ---- AggregateStatus ----
    #[test] fn test_aggregate_status_default() { assert_eq!(AggregateStatus::default().total, 0); }
    #[test] fn test_aggregate_status_from_submodules() {
        let sm = |s| Submodule { name: String::new(), path: PathBuf::new(), url: String::new(), tracked_branch: "main".into(), parent_pointer: CommitHash::default(), local_head: CommitHash::default(), remote_head: CommitHash::default(), status: s, ahead_count: 0, behind_count: 0, remote_unreachable: false };
        let agg = AggregateStatus::from_submodules(&[sm(SubmoduleStatus::Clean), sm(SubmoduleStatus::Dirty), sm(SubmoduleStatus::Orphaned)]);
        assert_eq!(agg.total, 3); assert_eq!(agg.clean, 1); assert_eq!(agg.dirty, 1); assert_eq!(agg.orphaned, 1);
    }
    #[test] fn test_aggregate_status_all_variants() {
        let sm = |s| Submodule { name: String::new(), path: PathBuf::new(), url: String::new(), tracked_branch: "main".into(), parent_pointer: CommitHash::default(), local_head: CommitHash::default(), remote_head: CommitHash::default(), status: s, ahead_count: 0, behind_count: 0, remote_unreachable: false };
        let agg = AggregateStatus::from_submodules(&[sm(SubmoduleStatus::Clean), sm(SubmoduleStatus::AheadOfParent), sm(SubmoduleStatus::BehindRemote), sm(SubmoduleStatus::Detached), sm(SubmoduleStatus::Dirty), sm(SubmoduleStatus::Orphaned), sm(SubmoduleStatus::Uninitialized)]);
        assert_eq!(agg.total, 7); assert_eq!(agg.clean, 1);
    }

    // ---- parse_oid / count_between ----
    #[test] fn test_parse_oid_valid() { assert!(parse_oid(&CommitHash("a1b2c3d4e5f6a7b8c9d0e1f2a3b4c5d6e7f8a9b0".into())).is_some()); }
    #[test] fn test_parse_oid_invalid() { assert!(parse_oid(&CommitHash("not-a-hex-string".into())).is_none()); }
    #[test] fn test_parse_oid_empty() { assert!(parse_oid(&CommitHash(String::new())).is_none()); }
    #[test] fn test_count_between_opt_both_none() { let t = tempfile::tempdir().unwrap(); git_init(t.path()); let r = git2::Repository::open(t.path()).unwrap(); assert_eq!(count_between_opt(&r, None, None), 0); }
    #[test] fn test_count_between_opt_equal_oids() { let t = tempfile::tempdir().unwrap(); git_init(t.path()); git_commit(t.path(), "c1"); let r = git2::Repository::open(t.path()).unwrap(); let h = r.head().unwrap().target().unwrap(); assert_eq!(count_between_opt(&r, Some(h), Some(h)), 0); }
    #[test] fn test_count_between_opt_from_to() { let t = tempfile::tempdir().unwrap(); git_init(t.path()); git_commit(t.path(), "c1"); let r = git2::Repository::open(t.path()).unwrap(); let c1 = r.head().unwrap().target().unwrap(); git_commit(t.path(), "c2"); let c2 = r.head().unwrap().target().unwrap(); assert_eq!(count_between_opt(&r, Some(c1), Some(c2)), 1); }
    #[test] fn test_count_between_opt_revwalk_fail() { let t = tempfile::tempdir().unwrap(); git_init(t.path()); let r = git2::Repository::open(t.path()).unwrap(); let o = git2::Oid::from_str("a1b2c3d4e5f6a7b8c9d0e1f2a3b4c5d6e7f8a9b0").ok(); assert_eq!(count_between_opt(&r, o, o), 0); }

    // ---- scan tests ----
    #[test] fn test_scan_no_gitmodules() { assert!(RepoState::scan(&tempfile::tempdir().unwrap().path()).is_err()); }
    #[test] fn test_scan_git_repo_but_no_submodules() { let t = tempfile::tempdir().unwrap(); git_init(t.path()); git_commit(t.path(), "initial"); assert_eq!(RepoState::scan(t.path()).unwrap().total, 0); }
    #[test] fn test_scan_non_git_directory() { let t = tempfile::tempdir().unwrap(); std::fs::write(t.path().join(".gitmodules"), "").unwrap(); assert!(RepoState::scan(t.path()).is_err()); }
    #[test] fn test_scan_with_submodule() { let t = tempfile::tempdir().unwrap(); let p = setup_repo_with_submodule(t.path()); let s = RepoState::scan(&p).unwrap(); assert_eq!(s.total, 1); assert_eq!(s.submodules[0].name, "libs/sub"); }
    #[test] fn test_scan_all_no_gitmodules() { assert!(RepoState::scan_all(&tempfile::tempdir().unwrap().path()).is_err()); }
    #[test] fn test_scan_all_with_submodule() { let t = tempfile::tempdir().unwrap(); let p = setup_repo_with_submodule(t.path()); let (subs, _) = RepoState::scan_all(&p).unwrap(); assert_eq!(subs.len(), 1); }
    #[test] fn test_repo_state_empty() { let s = RepoState { root_path: PathBuf::from("/tmp"), submodules: vec![], total: 0, clean_count: 0, needs_attention: vec![] }; assert_eq!(s.total, 0); }

    // ---- GitSubmoduleEditor ----
    #[test] fn test_editor_new_and_root() { let e = GitSubmoduleEditor::new(PathBuf::from("/tmp")); assert_eq!(e.root(), std::path::Path::new("/tmp")); }
    #[test] fn test_editor_sync_to_parent() { let t = tempfile::tempdir().unwrap(); let p = setup_repo_with_submodule(t.path()); assert!(GitSubmoduleEditor::new(p).sync_to_parent("libs/sub").is_ok()); }
    #[test] fn test_editor_sync_to_parent_nonexistent() { let t = tempfile::tempdir().unwrap(); std::fs::create_dir_all(t.path().join(".git")).unwrap(); assert!(GitSubmoduleEditor::new(t.path().to_path_buf()).sync_to_parent("no-such-module").is_err()); }
    #[test] fn test_editor_sync_all_to_parent() { let t = tempfile::tempdir().unwrap(); let p = setup_repo_with_submodule(t.path()); assert!(GitSubmoduleEditor::new(p).sync_all_to_parent().is_ok()); }
    #[test] fn test_editor_sync_all_to_parent_no_submodules() { let t = tempfile::tempdir().unwrap(); git_init(t.path()); git_commit(t.path(), "initial"); assert!(GitSubmoduleEditor::new(t.path().to_path_buf()).sync_all_to_parent().is_ok()); }
    #[test] fn test_editor_status() { let t = tempfile::tempdir().unwrap(); let p = setup_repo_with_submodule(t.path()); assert!(GitSubmoduleEditor::new(p).status().unwrap().is_empty()); }
    #[test] fn test_editor_status_with_gitmodules_but_no_repo() { let t = tempfile::tempdir().unwrap(); std::fs::write(t.path().join(".gitmodules"), "").unwrap(); assert!(GitSubmoduleEditor::new(t.path().to_path_buf()).status().is_err()); }

    #[test] fn test_editor_sync_with_remote_push() {
        let tmp = tempfile::tempdir().unwrap();
        let bare_sub = tmp.path().join("bare-sub");
        let bare_parent = tmp.path().join("bare-parent");
        for b in [&bare_sub, &bare_parent] { Command::new("git").args(["init", "--bare", &b.to_string_lossy()]).output().unwrap(); }
        let sub = tmp.path().join("sub");
        Command::new("git").args(["clone", &bare_sub.to_string_lossy(), &sub.to_string_lossy()]).current_dir(tmp.path()).output().unwrap();
        git_init(&sub); git_commit(&sub, "init"); Command::new("git").args(["push", "origin", "main"]).current_dir(&sub).output().unwrap();
        let parent = tmp.path().join("parent");
        std::fs::create_dir_all(&parent).unwrap(); git_init(&parent); git_commit(&parent, "init parent");
        Command::new("git").args(["remote", "add", "origin", &bare_parent.to_string_lossy()]).current_dir(&parent).output().unwrap();
        Command::new("git").args(["submodule", "add", &bare_sub.to_string_lossy(), "libs/sub"]).current_dir(&parent).output().unwrap();
        Command::new("git").args(["commit", "-m", "add submodule"]).current_dir(&parent).output().unwrap();
        git_commit(&sub, "ahead"); Command::new("git").args(["push", "origin", "main"]).current_dir(&sub).output().unwrap();
        Command::new("git").args(["fetch", "origin"]).current_dir(&parent.join("libs/sub")).output().unwrap();
        assert!(GitSubmoduleEditor::new(parent).sync_to_parent("libs/sub").is_ok(), "sync failed");
    }

    #[test] fn test_editor_sync_rebase_catches_up() {
        let tmp = tempfile::tempdir().unwrap();
        let bare_sub = tmp.path().join("bare-sub");
        let bare_parent = tmp.path().join("bare-parent");
        for b in [&bare_sub, &bare_parent] { Command::new("git").args(["init", "--bare", &b.to_string_lossy()]).output().unwrap(); }
        let sub = tmp.path().join("sub");
        Command::new("git").args(["clone", &bare_sub.to_string_lossy(), &sub.to_string_lossy()]).current_dir(tmp.path()).output().unwrap();
        git_init(&sub); git_commit(&sub, "init");
        Command::new("git").args(["push", "origin", "main"]).current_dir(&sub).output().unwrap();
        let init_hash = String::from_utf8_lossy(&Command::new("git").args(["rev-parse", "HEAD"]).current_dir(&sub).output().unwrap().stdout).trim().to_string();
        let parent = tmp.path().join("parent");
        std::fs::create_dir_all(&parent).unwrap(); git_init(&parent); git_commit(&parent, "init parent");
        Command::new("git").args(["remote", "add", "origin", &bare_parent.to_string_lossy()]).current_dir(&parent).output().unwrap();
        Command::new("git").args(["submodule", "add", &bare_sub.to_string_lossy(), "libs/sub"]).current_dir(&parent).output().unwrap();
        Command::new("git").args(["commit", "-m", "add submodule"]).current_dir(&parent).output().unwrap();
        let sm_path = parent.join("libs/sub");
        assert_eq!(
            String::from_utf8_lossy(&Command::new("git").args(["rev-parse", "HEAD"]).current_dir(&sm_path).output().unwrap().stdout).trim().to_string(),
            init_hash, "submodule starts at init"
        );
        git_commit(&sub, "remote ahead");
        Command::new("git").args(["push", "origin", "main"]).current_dir(&sub).output().unwrap();
        let remote_hash = String::from_utf8_lossy(&Command::new("git").args(["rev-parse", "HEAD"]).current_dir(&sub).output().unwrap().stdout).trim().to_string();
        assert!(GitSubmoduleEditor::new(parent).sync_to_parent("libs/sub").is_ok(), "sync failed");
        assert_eq!(
            String::from_utf8_lossy(&Command::new("git").args(["rev-parse", "HEAD"]).current_dir(&sm_path).output().unwrap().stdout).trim().to_string(),
            remote_hash, "submodule caught up to remote after sync"
        );
    }

    #[test] fn test_editor_status_with_dirty_submodule() {
        let t = tempfile::tempdir().unwrap(); let p = setup_repo_with_submodule(t.path());
        std::fs::write(p.join("libs/sub/new-file"), "content").unwrap();
        let issues = GitSubmoduleEditor::new(p).status().unwrap();
        assert!(!issues.is_empty()); assert_eq!(issues[0].status, "Dirty");
    }

    // ---- describe_issue ----
    #[test] fn test_describe_issue_ahead_of_parent() { let (d, a) = describe_issue(&SubmoduleStatus::AheadOfParent); assert!(d.contains("领先")); assert!(a.contains("sync")); }
    #[test] fn test_describe_issue_behind_remote() { let (d, a) = describe_issue(&SubmoduleStatus::BehindRemote); assert!(d.contains("落后")); assert!(a.contains("sync")); }
    #[test] fn test_describe_issue_detached() { let (d, a) = describe_issue(&SubmoduleStatus::Detached); assert!(d.contains("游离")); assert!(a.contains("checkout")); }
    #[test] fn test_describe_issue_dirty() { let (d, a) = describe_issue(&SubmoduleStatus::Dirty); assert!(d.contains("修改")); }
    #[test] fn test_describe_issue_orphaned() { let (d, a) = describe_issue(&SubmoduleStatus::Orphaned); assert!(d.contains("不存在")); }
    #[test] fn test_describe_issue_uninitialized() { let (d, a) = describe_issue(&SubmoduleStatus::Uninitialized); assert!(d.contains("初始化")); }
    #[test] #[should_panic(expected = "unreachable")] fn test_describe_issue_clean_panics() { describe_issue(&SubmoduleStatus::Clean); }

    // ---- edge case scan tests ----
    #[test] fn test_scan_with_uninitialized_submodule() {
        let tmp = tempfile::tempdir().unwrap(); let parent = tmp.path().join("parent");
        std::fs::create_dir_all(&parent).unwrap(); git_init(&parent); git_commit(&parent, "init");
        let sub = tmp.path().join("sub"); std::fs::create_dir_all(&sub).unwrap(); git_init(&sub); git_commit(&sub, "init");
        Command::new("git").args(["submodule", "add", &sub.to_string_lossy(), "libs/sub"]).current_dir(&parent).output().unwrap();
        Command::new("git").args(["commit", "-m", "add submodule"]).current_dir(&parent).output().unwrap();
        Command::new("git").args(["submodule", "deinit", "-f", "libs/sub"]).current_dir(&parent).output().unwrap();
        assert_eq!(RepoState::scan(&parent).unwrap().submodules[0].status, SubmoduleStatus::Uninitialized);
    }

    #[test] fn test_scan_with_detached_submodule() {
        let tmp = tempfile::tempdir().unwrap(); let parent = setup_repo_with_submodule(tmp.path());
        let sm_path = parent.join("libs/sub");
        let hash = String::from_utf8_lossy(&Command::new("git").args(["rev-parse", "HEAD"]).current_dir(&sm_path).output().unwrap().stdout).trim().to_string();
        Command::new("git").args(["checkout", "--detach", &hash]).current_dir(&sm_path).output().unwrap();
        assert_eq!(RepoState::scan(&parent).unwrap().submodules[0].status, SubmoduleStatus::Detached);
    }

    #[test] fn test_scan_with_ahead_via_remote_unreachable() {
        let tmp = tempfile::tempdir().unwrap(); let parent = setup_repo_with_submodule(tmp.path());
        let sm_path = parent.join("libs/sub");
        std::fs::write(sm_path.join("new-file"), "content").unwrap();
        Command::new("git").args(["add", "."]).current_dir(&sm_path).output().unwrap();
        Command::new("git").args(["commit", "-m", "ahead commit"]).current_dir(&sm_path).output().unwrap();
        Command::new("git").args(["remote", "remove", "origin"]).current_dir(&sm_path).output().unwrap();
        let state = RepoState::scan(&parent).unwrap();
        assert_eq!(state.submodules[0].status, SubmoduleStatus::AheadOfParent);
    }

    #[test] fn test_scan_with_subrepo_open_error() {
        let tmp = tempfile::tempdir().unwrap(); let parent = setup_repo_with_submodule(tmp.path());
        let sm_git = parent.join("libs/sub/.git");
        if sm_git.is_dir() { std::fs::remove_dir_all(&sm_git).unwrap(); } else { std::fs::remove_file(&sm_git).unwrap(); }
        assert_eq!(RepoState::scan(&parent).unwrap().submodules[0].local_head, CommitHash::default());
    }

    #[test] fn test_scan_with_behind_remote() {
        let tmp = tempfile::tempdir().unwrap(); let parent = tmp.path().join("parent"); let sub = tmp.path().join("sub"); let bare = tmp.path().join("bare");
        std::fs::create_dir_all(&bare).unwrap(); Command::new("git").args(["init", "--bare", &bare.to_string_lossy()]).current_dir(tmp.path()).output().unwrap();
        Command::new("git").args(["clone", &bare.to_string_lossy(), &sub.to_string_lossy()]).current_dir(tmp.path()).output().unwrap();
        git_init(&sub); git_commit(&sub, "init"); Command::new("git").args(["push", "origin", "main"]).current_dir(&sub).output().unwrap();
        std::fs::create_dir_all(&parent).unwrap(); git_init(&parent); git_commit(&parent, "init parent");
        Command::new("git").args(["submodule", "add", &sub.to_string_lossy(), "libs/sub"]).current_dir(&parent).output().unwrap();
        Command::new("git").args(["commit", "-m", "add submodule"]).current_dir(&parent).output().unwrap();
        git_commit(&sub, "remote ahead"); Command::new("git").args(["push", "origin", "main"]).current_dir(&sub).output().unwrap();
        Command::new("git").args(["fetch", "origin"]).current_dir(&parent.join("libs/sub")).output().unwrap();
        assert_eq!(RepoState::scan(&parent).unwrap().submodules[0].behind_count, 1);
    }

    #[test] fn test_scan_with_orphaned_submodule() {
        let tmp = tempfile::tempdir().unwrap(); let parent = setup_repo_with_submodule(tmp.path());
        let sm_path = parent.join("libs/sub");
        Command::new("git").args(["remote", "remove", "origin"]).current_dir(&sm_path).output().unwrap();
        let ref_dir = parent.join(".git/modules/libs/sub/refs/remotes/origin");
        std::fs::create_dir_all(&ref_dir).unwrap();
        std::fs::write(ref_dir.join("main"), "1111111111111111111111111111111111111111\n").unwrap();
        assert_eq!(RepoState::scan(&parent).unwrap().submodules[0].status, SubmoduleStatus::Orphaned);
    }

    #[test] fn test_scan_with_ahead_of_parent_clean() {
        let tmp = tempfile::tempdir().unwrap(); let parent = setup_repo_with_submodule(tmp.path());
        git_commit(&parent.join("libs/sub"), "ahead commit");
        assert!(RepoState::scan(&parent).unwrap().submodules[0].ahead_count > 0);
    }

    #[test] fn test_orphaned_parse_oid_failure() {
        let tmp = tempfile::tempdir().unwrap(); let parent = setup_repo_with_submodule(tmp.path());
        let ref_dir = parent.join(".git/modules/libs/sub/refs/remotes/origin");
        if !ref_dir.exists() { std::fs::create_dir_all(&ref_dir).unwrap(); }
        std::fs::write(ref_dir.join("main"), "not-a-valid-oid\n").unwrap();
        assert!(!RepoState::scan(&parent).unwrap().submodules.is_empty());
    }

    #[test] fn test_ahead_of_parent_via_ahead_count() {
        let tmp = tempfile::tempdir().unwrap(); let parent = setup_repo_with_submodule(tmp.path());
        let sm_path = parent.join("libs/sub");
        Command::new("git").args(["remote", "remove", "origin"]).current_dir(&sm_path).output().unwrap();
        std::fs::write(sm_path.join("new-file"), "content").unwrap();
        Command::new("git").args(["add", "."]).current_dir(&sm_path).output().unwrap();
        Command::new("git").args(["commit", "-m", "ahead"]).current_dir(&sm_path).output().unwrap();
        let state = RepoState::scan(&parent).unwrap();
        assert_eq!(state.submodules[0].ahead_count, 1);
    }
}
