use std::path::{Path, PathBuf};

use crate::commands::{HealthIssue, SubmoduleEditor};
use crate::model::code::{RepoState, SubmoduleStatus};

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
}

impl GitSubmoduleEditor {
    fn fetch_submodule(path: &std::path::Path) -> Result<(), ()> {
        let has_remote = std::process::Command::new("git")
            .args(["remote", "get-url", "origin"])
            .current_dir(path)
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);
        if !has_remote { return Ok(()); }
        let output = std::process::Command::new("git")
            .args(["fetch", "origin"])
            .current_dir(path)
            .output()
            .map_err(|_| ())?;
        if output.status.success() { Ok(()) } else { Err(()) }
    }

    fn push_submodule(path: &std::path::Path) -> Result<(), String> {
        if !path.exists() {
            return Ok(());
        }
        let branch = std::process::Command::new("git")
            .args(["rev-parse", "--abbrev-ref", "HEAD"])
            .current_dir(path)
            .output()
            .ok()
            .and_then(|o| {
                if o.status.success() {
                    Some(String::from_utf8_lossy(&o.stdout).trim().to_string())
                } else {
                    None
                }
            })
            .unwrap_or_default();
        if branch.is_empty() || branch == "HEAD" {
            return Ok(());
        }
        let has_remote = std::process::Command::new("git")
            .args(["remote", "get-url", "origin"])
            .current_dir(path)
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);
        if !has_remote {
            return Ok(());
        }
        let tracking = format!("origin/{}", branch);
        let ahead = std::process::Command::new("git")
            .args(["rev-list", "--count", &format!("{}..{}", tracking, branch)])
            .current_dir(path)
            .output()
            .ok()
            .and_then(|o| {
                if o.status.success() {
                    String::from_utf8_lossy(&o.stdout).trim().parse::<i32>().ok()
                } else {
                    None
                }
            })
            .unwrap_or(0);
        if ahead <= 0 {
            return Ok(());
        }
        let output = std::process::Command::new("git")
            .args(["push", "origin", &branch])
            .current_dir(path)
            .output()
            .map_err(|e| format!("git push 无法执行: {}", e))?;
        if output.status.success() {
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(stderr.trim().to_string())
        }
    }

    fn update_parent_pointer(
        repo: &git2::Repository,
        sm_path: &std::path::Path,
        name: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut index = repo.index()?;
        index.add_path(sm_path)?;
        index.write()?;
        let tree_id = index.write_tree()?;
        let tree = repo.find_tree(tree_id)?;
        let head = repo.head()?;
        let parent = head.peel_to_commit()?;
        let signature = repo.signature()?;
        repo.commit(
            Some("HEAD"),
            &signature,
            &signature,
            &format!("chore: 更新子模块 '{}' 指针", name),
            &tree,
            &[&parent],
        )?;
        Ok(())
    }

    fn push_parent(repo: &git2::Repository, root: &std::path::Path) -> Result<(), String> {
        let has_remote = std::process::Command::new("git")
            .args(["remote", "get-url", "origin"])
            .current_dir(root)
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);
        if !has_remote {
            return Ok(());
        }
        let branch = repo
            .head()
            .ok()
            .and_then(|r| r.shorthand().map(|s| s.to_string()))
            .unwrap_or_default();
        if branch.is_empty() {
            return Err("无法检测当前分支".into());
        }
        let output = std::process::Command::new("git")
            .args(["push", "origin", &branch])
            .current_dir(root)
            .output()
            .map_err(|e| format!("git push 无法执行: {}", e))?;
        if output.status.success() {
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(stderr.trim().to_string())
        }
    }

    fn revert_parent_commit(root: &std::path::Path) {
        std::process::Command::new("git")
            .args(["reset", "--hard", "HEAD~1"])
            .current_dir(root)
            .output()
            .ok();
    }
}

impl SubmoduleEditor for GitSubmoduleEditor {
    fn root(&self) -> &Path {
        &self.root
    }

    fn sync_to_parent(&self, name: &str) -> Result<(), Box<dyn std::error::Error>> {
        let repo = git2::Repository::open(&self.root)?;
        let sm = repo.find_submodule(name)?;
        let sm_path = sm.path();
        let full_sm_path = self.root.join(sm_path);

        // 1. fetch 远端，确保推送时基于最新状态
        if full_sm_path.exists() {
            Self::fetch_submodule(&full_sm_path).ok();
        }

        // 2. push 子模块本地提交到远端
        Self::push_submodule(&full_sm_path).map_err(|e| format!("子模块 push 失败: {}", e))?;

        // 3. 更新父仓库指针并提交
        Self::update_parent_pointer(&repo, sm_path, name)?;

        // 4. push 父仓库；失败时回滚第 3 步的提交
        if let Err(e) = Self::push_parent(&repo, &self.root) {
            Self::revert_parent_commit(&self.root);
            return Err(format!("父仓库 push 失败 (已回滚提交): {}", e).into());
        }

        println!("  ✓ {}", name);
        Ok(())
    }

    fn sync_all_to_parent(&self) -> Result<(), Box<dyn std::error::Error>> {
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

    fn status(&self) -> Result<Vec<HealthIssue>, Box<dyn std::error::Error>> {
        // RepoState::scan() 内部对每个子模块执行 fetch，确保 remote_head 实时
        let state = RepoState::scan(&self.root)?;
        let mut issues = Vec::new();
        for sm in &state.submodules {
            if sm.status != SubmoduleStatus::Clean {
                let (description, action) = describe_issue(&sm.status);
                issues.push(HealthIssue {
                    submodule_name: sm.name.clone(),
                    status: sm.status.clone(),
                    description,
                    suggested_action: action,
                });
            }
        }
        Ok(issues)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;

    fn git_init(repo_path: &std::path::Path) {
        Command::new("git")
            .args(["init"])
            .current_dir(repo_path)
            .output()
            .unwrap();
        Command::new("git")
            .args(["config", "user.email", "test@test.com"])
            .current_dir(repo_path)
            .output()
            .unwrap();
        Command::new("git")
            .args(["config", "user.name", "Test"])
            .current_dir(repo_path)
            .output()
            .unwrap();
    }

    fn git_commit(repo_path: &std::path::Path, msg: &str) {
        std::fs::write(repo_path.join("file"), msg).unwrap();
        Command::new("git")
            .args(["add", "."])
            .current_dir(repo_path)
            .output()
            .unwrap();
        Command::new("git")
            .args(["commit", "-m", msg])
            .current_dir(repo_path)
            .output()
            .unwrap();
    }

    fn setup_repo_with_submodule(tmp: &std::path::Path) -> std::path::PathBuf {
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

    // ---- describe_issue ----

    #[test]
    fn test_describe_issue_ahead_of_parent() {
        let (desc, action) = describe_issue(&SubmoduleStatus::AheadOfParent);
        assert!(desc.contains("领先"));
        assert!(action.contains("sync"));
    }

    #[test]
    fn test_describe_issue_behind_remote() {
        let (desc, action) = describe_issue(&SubmoduleStatus::BehindRemote);
        assert!(desc.contains("落后"));
        assert!(action.contains("update"));
    }

    #[test]
    fn test_describe_issue_detached() {
        let (desc, action) = describe_issue(&SubmoduleStatus::Detached);
        assert!(desc.contains("游离"));
        assert!(action.contains("checkout"));
    }

    #[test]
    fn test_describe_issue_dirty() {
        let (desc, action) = describe_issue(&SubmoduleStatus::Dirty);
        assert!(desc.contains("修改"));
        assert!(action.contains("提交") || action.contains("stash"));
    }

    #[test]
    fn test_describe_issue_orphaned() {
        let (desc, action) = describe_issue(&SubmoduleStatus::Orphaned);
        assert!(desc.contains("不存在"));
        assert!(action.contains("手动"));
    }

    #[test]
    fn test_describe_issue_uninitialized() {
        let (desc, action) = describe_issue(&SubmoduleStatus::Uninitialized);
        assert!(desc.contains("初始化"));
        assert!(action.contains("init"));
    }

    #[test]
    #[should_panic(expected = "unreachable")]
    fn test_describe_issue_clean_panics() {
        describe_issue(&SubmoduleStatus::Clean);
    }

    // ---- GitSubmoduleEditor ----

    #[test]
    fn test_editor_new_and_root() {
        let p = PathBuf::from("/tmp/test-editor");
        let editor = GitSubmoduleEditor::new(p.clone());
        assert_eq!(editor.root(), p);
    }

    #[test]
    fn test_editor_sync_to_parent() {
        let tmp = tempfile::tempdir().unwrap();
        let parent = setup_repo_with_submodule(tmp.path());
        let editor = GitSubmoduleEditor::new(parent);
        let result = editor.sync_to_parent("libs/sub");
        assert!(result.is_ok());
    }

    #[test]
    fn test_editor_sync_to_parent_nonexistent() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(tmp.path().join(".git")).unwrap();
        let editor = GitSubmoduleEditor::new(tmp.path().to_path_buf());
        let result = editor.sync_to_parent("no-such-module");
        assert!(result.is_err());
    }

    #[test]
    fn test_editor_sync_all_to_parent() {
        let tmp = tempfile::tempdir().unwrap();
        let parent = setup_repo_with_submodule(tmp.path());
        let editor = GitSubmoduleEditor::new(parent);
        let result = editor.sync_all_to_parent();
        assert!(result.is_ok());
    }

    #[test]
    fn test_editor_sync_all_to_parent_no_submodules() {
        let tmp = tempfile::tempdir().unwrap();
        git_init(tmp.path());
        git_commit(tmp.path(), "initial");
        let editor = GitSubmoduleEditor::new(tmp.path().to_path_buf());
        let result = editor.sync_all_to_parent();
        assert!(result.is_ok());
    }

    #[test]
    fn test_editor_status() {
        let tmp = tempfile::tempdir().unwrap();
        let parent = setup_repo_with_submodule(tmp.path());
        let editor = GitSubmoduleEditor::new(parent);
        let issues = editor.status().unwrap();
        // initially the submodule should be clean
        assert!(issues.is_empty());
    }

    #[test]
    fn test_editor_status_with_gitmodules_but_no_repo() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join(".gitmodules"), "").unwrap();
        let editor = GitSubmoduleEditor::new(tmp.path().to_path_buf());
        let result = editor.status();
        assert!(result.is_err());
    }

    #[test]
    fn test_editor_sync_with_remote_push() {
        let tmp = tempfile::tempdir().unwrap();
        let bare_sub = tmp.path().join("bare-sub");
        let bare_parent = tmp.path().join("bare-parent");
        for b in [&bare_sub, &bare_parent] {
            Command::new("git")
                .args(["init", "--bare", &b.to_string_lossy()])
                .output().unwrap();
        }
        let sub = tmp.path().join("sub");
        Command::new("git")
            .args(["clone", &bare_sub.to_string_lossy(), &sub.to_string_lossy()])
            .current_dir(tmp.path()).output().unwrap();
        git_init(&sub);
        git_commit(&sub, "init");
        Command::new("git")
            .args(["push", "origin", "main"]).current_dir(&sub).output().unwrap();
        let parent = tmp.path().join("parent");
        std::fs::create_dir_all(&parent).unwrap();
        git_init(&parent);
        git_commit(&parent, "init parent");
        Command::new("git")
            .args(["remote", "add", "origin", &bare_parent.to_string_lossy()])
            .current_dir(&parent).output().unwrap();
        Command::new("git")
            .args(["submodule", "add", &bare_sub.to_string_lossy(), "libs/sub"])
            .current_dir(&parent).output().unwrap();
        Command::new("git")
            .args(["commit", "-m", "add submodule"])
            .current_dir(&parent).output().unwrap();
        git_commit(&sub, "ahead");
        Command::new("git")
            .args(["push", "origin", "main"]).current_dir(&sub).output().unwrap();
        Command::new("git")
            .args(["fetch", "origin"])
            .current_dir(&parent.join("libs/sub")).output().unwrap();
        let editor = GitSubmoduleEditor::new(parent);
        let result = editor.sync_to_parent("libs/sub");
        assert!(result.is_ok(), "sync failed: {:?}", result.as_ref().err());
    }

    #[test]
    fn test_editor_status_with_dirty_submodule() {
        let tmp = tempfile::tempdir().unwrap();
        let parent = setup_repo_with_submodule(tmp.path());
        let sm_path = parent.join("libs/sub");
        std::fs::write(sm_path.join("new-file"), "content").unwrap();
        let editor = GitSubmoduleEditor::new(parent);
        let issues = editor.status().unwrap();
        assert!(!issues.is_empty());
        assert_eq!(issues[0].status, SubmoduleStatus::Dirty);
    }
}

pub(crate) fn describe_issue(status: &SubmoduleStatus) -> (String, String) {
    match status {
        SubmoduleStatus::AheadOfParent => (
            "本地领先于父仓库记录".into(),
            "运行 sync_to_parent 更新父仓库指针".into(),
        ),
        SubmoduleStatus::BehindRemote => (
            "远程有更新，本地落后".into(),
            "运行 update 获取最新代码".into(),
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
