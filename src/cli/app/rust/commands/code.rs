use std::path::{Path, PathBuf};

use crate::commands::{HealthIssue, SubmoduleEditor};
use crate::model::{RepoState, SubmoduleStatus};

pub struct GitSubmoduleEditor {
    root: PathBuf,
}

impl GitSubmoduleEditor {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
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

        let mut index = repo.index()?;
        index.add_path(sm_path)?;
        index.write()?;

        let tree_id = index.write_tree()?;
        let tree = repo.find_tree(tree_id)?;
        let head = repo.head()?;
        let parent = head.peel_to_commit()?;
        let signature = git2::Signature::now("kse", "kse@local")?;
        repo.commit(
            Some("HEAD"),
            &signature,
            &signature,
            &format!("chore: 更新子模块 '{}' 指针", name),
            &tree,
            &[&parent],
        )?;
        println!("已同步子模块 '{}' 到父仓库", name);

        // 推送当前分支到 origin（使用系统 git 命令，避免 git2 SSL 兼容问题）
        let branch = repo
            .head()
            .ok()
            .and_then(|r| r.shorthand().map(|s| s.to_string()))
            .unwrap_or_default();
        if !branch.is_empty() {
            let output = std::process::Command::new("git")
                .args(["push", "origin", &branch])
                .current_dir(&self.root)
                .output()
                .map_err(|e| format!("无法执行 git push: {}", e))?;
            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                eprintln!("警告: git push 失败: {}", stderr.trim());
            } else {
                println!("已推送 '{}' 到 origin", branch);
            }
        }
        Ok(())
    }

    fn sync_all_to_parent(&self) -> Result<(), Box<dyn std::error::Error>> {
        let repo = git2::Repository::open(&self.root)?;
        let submodules = repo.submodules()?;
        for sm in submodules.iter() {
            let name = sm.name().unwrap_or("unknown").to_string();
            match self.sync_to_parent(&name) {
                Ok(()) => {}
                Err(e) => eprintln!("警告: 同步子模块 '{}' 失败: {}", name, e),
            }
        }
        Ok(())
    }

    fn retire_submodule(&self, name: &str) -> Result<(), Box<dyn std::error::Error>> {
        let repo = git2::Repository::open(&self.root)?;
        let sm = repo.find_submodule(name)?;
        let sm_path = sm.path().to_path_buf();

        let result = std::process::Command::new("git")
            .args(["submodule", "deinit", "-f", name])
            .current_dir(&self.root)
            .output();
        match result {
            Err(e) => eprintln!("警告: git submodule deinit 无法执行: {} (继续处理)", e),
            Ok(out) if !out.status.success() => {
                let stderr = String::from_utf8_lossy(&out.stderr);
                eprintln!("警告: git submodule deinit 失败: {} (继续处理)", stderr.trim());
            }
            _ => {}
        }

        let gitmodules_path = self.root.join(".gitmodules");
        if gitmodules_path.exists() {
            let content = std::fs::read_to_string(&gitmodules_path)?;
            let mut new_content = String::new();
            let mut skip = false;
            let in_submodule_alt = format!("[submodule \"{}\"]", name);
            for line in content.lines() {
                if line.trim() == in_submodule_alt {
                    skip = true;
                    continue;
                }
                if skip && line.trim_start().starts_with('[') {
                    skip = false;
                }
                if !skip {
                    new_content.push_str(line);
                    new_content.push('\n');
                }
            }
            std::fs::write(&gitmodules_path, new_content)?;
        }
        let mut index = repo.index()?;
        index.remove_path(&sm_path)?;
        index.write()?;

        println!("已退役子模块 '{}'", name);
        Ok(())
    }

    fn status(&self) -> Result<Vec<HealthIssue>, Box<dyn std::error::Error>> {
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
    fn test_editor_retire_submodule() {
        let tmp = tempfile::tempdir().unwrap();
        let parent = setup_repo_with_submodule(tmp.path());
        let editor = GitSubmoduleEditor::new(parent.clone());
        let result = editor.retire_submodule("libs/sub");
        assert!(result.is_ok());
        // verify .gitmodules no longer has the submodule entry
        let gitmodules = parent.join(".gitmodules");
        assert!(!gitmodules.exists()
            || !std::fs::read_to_string(&gitmodules)
                .unwrap()
                .contains("libs/sub"));
    }

    #[test]
    fn test_editor_retire_submodule_nonexistent() {
        let tmp = tempfile::tempdir().unwrap();
        git_init(tmp.path());
        git_commit(tmp.path(), "initial");
        let editor = GitSubmoduleEditor::new(tmp.path().to_path_buf());
        let result = editor.retire_submodule("no-such-module");
        assert!(result.is_err());
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
