use crate::git::scan::*;
use crate::git::types::*;
use std::path::{Path, PathBuf};

// ===== git operations =====

pub struct GitSubmoduleEditor {
    root: PathBuf,
    offline: bool,
}

impl GitSubmoduleEditor {
    pub fn new(root: PathBuf) -> Self {
        Self {
            root,
            offline: false,
        }
    }

    pub fn set_offline(&mut self, offline: bool) {
        self.offline = offline;
    }

    pub fn fetch_submodule(path: &Path) -> Result<(), ()> {
        let has_remote = std::process::Command::new("git")
            .args(["remote", "get-url", "origin"])
            .current_dir(path)
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);
        if !has_remote {
            return Ok(());
        }
        std::process::Command::new("git")
            .args(["fetch", "origin"])
            .current_dir(path)
            .output()
            .map(|o| if o.status.success() { Ok(()) } else { Err(()) })
            .unwrap_or(Err(()))
    }

    pub fn rebase_submodule(path: &Path) -> Result<(), String> {
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
        if !std::process::Command::new("git")
            .args(["remote", "get-url", "origin"])
            .current_dir(path)
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
        {
            return Ok(());
        }
        let output = std::process::Command::new("git")
            .args(["rebase", &format!("origin/{}", branch)])
            .current_dir(path)
            .output()
            .map_err(|e| format!("git rebase 无法执行: {}", e))?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            if stderr.contains("up to date") || stderr.contains("up-to-date") {
                return Ok(());
            }
            return Err(format!(
                "rebase 冲突，需手动处理：解决冲突后 git rebase --continue，或 git rebase --abort 放弃\n{}",
                stderr
            ));
        }
        Ok(())
    }

    pub fn push_submodule(path: &Path) -> Result<(), String> {
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
        if !std::process::Command::new("git")
            .args(["remote", "get-url", "origin"])
            .current_dir(path)
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
        {
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
                    String::from_utf8_lossy(&o.stdout)
                        .trim()
                        .parse::<i32>()
                        .ok()
                } else {
                    None
                }
            })
            .unwrap_or(0);
        if ahead <= 0 {
            return Ok(());
        }
        std::process::Command::new("git")
            .args(["push", "origin", &branch])
            .current_dir(path)
            .output()
            .map(|o| {
                if o.status.success() {
                    Ok(())
                } else {
                    Err(String::from_utf8_lossy(&o.stderr).trim().to_string())
                }
            })
            .unwrap_or_else(|e| Err(format!("git push 无法执行: {}", e)))
    }

    pub fn update_parent_pointer(
        root: &Path,
        sm_path: &Path,
        name: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let sm_rel = sm_path.to_string_lossy().to_string();
        std::process::Command::new("git")
            .args(["add", &sm_rel])
            .current_dir(root)
            .output()?;
        let out = std::process::Command::new("git")
            .args([
                "commit",
                "-m",
                &format!("chore: 更新子模块 '{}' 指针", name),
            ])
            .current_dir(root)
            .output()?;
        if !out.status.success() {
            let msg = String::from_utf8_lossy(&out.stderr).trim().to_string();
            let msg2 = String::from_utf8_lossy(&out.stdout).trim().to_string();
            let combined = format!("{}\n{}", msg, msg2);
            if !combined.contains("nothing to commit")
                && !combined.contains("no changes")
                && !combined.contains("无文件")
            {
                return Err(msg.into());
            }
        }
        Ok(())
    }

    pub fn push_parent(root: &Path) -> Result<(), String> {
        if !std::process::Command::new("git")
            .args(["remote", "get-url", "origin"])
            .current_dir(root)
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
        {
            return Ok(());
        }
        let branch = std::process::Command::new("git")
            .args(["rev-parse", "--abbrev-ref", "HEAD"])
            .current_dir(root)
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
            return Err("无法检测当前分支".into());
        }
        std::process::Command::new("git")
            .args(["push", "origin", &branch])
            .current_dir(root)
            .output()
            .map(|o| {
                if o.status.success() {
                    Ok(())
                } else {
                    Err(String::from_utf8_lossy(&o.stderr).trim().to_string())
                }
            })
            .unwrap_or_else(|e| Err(format!("git push 无法执行: {}", e)))
    }

    pub fn revert_parent_commit(root: &Path) {
        std::process::Command::new("git")
            .args(["reset", "--hard", "HEAD~1"])
            .current_dir(root)
            .output()
            .ok();
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    fn find_sm_path(&self, name: &str) -> Option<PathBuf> {
        let entries = parse_gitmodules(&self.root);
        entries
            .into_iter()
            .find(|(n, _, _, _)| n == name)
            .map(|(_, p, _, _)| p)
    }

    pub fn sync_to_parent(&self, name: &str) -> Result<(), Box<dyn std::error::Error>> {
        let sm_path = self
            .find_sm_path(name)
            .ok_or_else(|| format!(".gitmodules 中未找到子模块 '{}'", name))?;
        let full_sm_path = self.root.join(&sm_path);

        if full_sm_path.exists() {
            Self::fetch_submodule(&full_sm_path).ok();
            Self::rebase_submodule(&full_sm_path)?;
        }
        Self::push_submodule(&full_sm_path).map_err(|e| format!("子模块 push 失败: {}", e))?;
        Self::update_parent_pointer(&self.root, &sm_path, name)?;
        if let Err(e) = Self::push_parent(&self.root) {
            Self::revert_parent_commit(&self.root);
            return Err(format!("父仓库 push 失败 (已回滚提交): {}", e).into());
        }
        println!("  ✓ {}", name);
        Ok(())
    }

    pub fn sync_all_to_parent(&self) -> Result<(), Box<dyn std::error::Error>> {
        let entries = parse_gitmodules(&self.root);
        let names: Vec<String> = entries.into_iter().map(|(n, _, _, _)| n).collect();
        println!("同步 {} 个子模块", names.len());
        for name in &names {
            match self.sync_to_parent(name) {
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

    // ---- GitSubmoduleEditor ----
    #[test]
    fn test_editor_new_and_root() {
        let e = GitSubmoduleEditor::new(PathBuf::from("/tmp"));
        assert_eq!(e.root(), std::path::Path::new("/tmp"));
    }
    #[test]
    fn test_editor_sync_to_parent() {
        let t = tempfile::tempdir().unwrap();
        let p = setup_repo_with_submodule(t.path());
        let result = GitSubmoduleEditor::new(p).sync_to_parent("libs/sub");
        assert!(result.is_ok(), "sync_to_parent error: {:?}", result);
    }
    #[test]
    fn test_editor_sync_to_parent_nonexistent() {
        let t = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(t.path().join(".git")).unwrap();
        assert!(GitSubmoduleEditor::new(t.path().to_path_buf())
            .sync_to_parent("no-such-module")
            .is_err());
    }
    #[test]
    fn test_editor_sync_all_to_parent() {
        let t = tempfile::tempdir().unwrap();
        let p = setup_repo_with_submodule(t.path());
        assert!(GitSubmoduleEditor::new(p).sync_all_to_parent().is_ok());
    }
    #[test]
    fn test_editor_sync_all_to_parent_no_submodules() {
        let t = tempfile::tempdir().unwrap();
        git_init(t.path());
        git_commit(t.path(), "initial");
        assert!(GitSubmoduleEditor::new(t.path().to_path_buf())
            .sync_all_to_parent()
            .is_ok());
    }
    #[test]
    fn test_editor_status() {
        let t = tempfile::tempdir().unwrap();
        let p = setup_repo_with_submodule(t.path());
        assert!(GitSubmoduleEditor::new(p).status().unwrap().is_empty());
    }
    #[test]
    fn test_editor_status_with_gitmodules_but_no_repo() {
        let t = tempfile::tempdir().unwrap();
        std::fs::write(t.path().join(".gitmodules"), "").unwrap();
        assert!(GitSubmoduleEditor::new(t.path().to_path_buf())
            .status()
            .is_err());
    }

    #[test]
    fn test_editor_sync_with_remote_push() {
        let tmp = tempfile::tempdir().unwrap();
        let bare_sub = tmp.path().join("bare-sub");
        let bare_parent = tmp.path().join("bare-parent");
        for b in [&bare_sub, &bare_parent] {
            Command::new("git")
                .args(["init", "--bare", &b.to_string_lossy()])
                .output()
                .unwrap();
        }
        let sub = tmp.path().join("sub");
        Command::new("git")
            .args(["clone", &bare_sub.to_string_lossy(), &sub.to_string_lossy()])
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
        let parent = tmp.path().join("parent");
        std::fs::create_dir_all(&parent).unwrap();
        git_init(&parent);
        git_commit(&parent, "init parent");
        Command::new("git")
            .args(["remote", "add", "origin", &bare_parent.to_string_lossy()])
            .current_dir(&parent)
            .output()
            .unwrap();
        Command::new("git")
            .args(["submodule", "add", &bare_sub.to_string_lossy(), "libs/sub"])
            .current_dir(&parent)
            .output()
            .unwrap();
        Command::new("git")
            .args(["commit", "-m", "add submodule"])
            .current_dir(&parent)
            .output()
            .unwrap();
        git_commit(&sub, "ahead");
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
        assert!(
            GitSubmoduleEditor::new(parent)
                .sync_to_parent("libs/sub")
                .is_ok(),
            "sync failed"
        );
    }

    #[test]
    fn test_editor_sync_rebase_catches_up() {
        let tmp = tempfile::tempdir().unwrap();
        let bare_sub = tmp.path().join("bare-sub");
        let bare_parent = tmp.path().join("bare-parent");
        for b in [&bare_sub, &bare_parent] {
            Command::new("git")
                .args(["init", "--bare", &b.to_string_lossy()])
                .output()
                .unwrap();
        }
        let sub = tmp.path().join("sub");
        Command::new("git")
            .args(["clone", &bare_sub.to_string_lossy(), &sub.to_string_lossy()])
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
        let init_hash = String::from_utf8_lossy(
            &Command::new("git")
                .args(["rev-parse", "HEAD"])
                .current_dir(&sub)
                .output()
                .unwrap()
                .stdout,
        )
        .trim()
        .to_string();
        let parent = tmp.path().join("parent");
        std::fs::create_dir_all(&parent).unwrap();
        git_init(&parent);
        git_commit(&parent, "init parent");
        Command::new("git")
            .args(["remote", "add", "origin", &bare_parent.to_string_lossy()])
            .current_dir(&parent)
            .output()
            .unwrap();
        Command::new("git")
            .args(["submodule", "add", &bare_sub.to_string_lossy(), "libs/sub"])
            .current_dir(&parent)
            .output()
            .unwrap();
        Command::new("git")
            .args(["commit", "-m", "add submodule"])
            .current_dir(&parent)
            .output()
            .unwrap();
        let sm_path = parent.join("libs/sub");
        assert_eq!(
            String::from_utf8_lossy(
                &Command::new("git")
                    .args(["rev-parse", "HEAD"])
                    .current_dir(&sm_path)
                    .output()
                    .unwrap()
                    .stdout
            )
            .trim()
            .to_string(),
            init_hash,
            "submodule starts at init"
        );
        git_commit(&sub, "remote ahead");
        Command::new("git")
            .args(["push", "origin", "main"])
            .current_dir(&sub)
            .output()
            .unwrap();
        let remote_hash = String::from_utf8_lossy(
            &Command::new("git")
                .args(["rev-parse", "HEAD"])
                .current_dir(&sub)
                .output()
                .unwrap()
                .stdout,
        )
        .trim()
        .to_string();
        assert!(
            GitSubmoduleEditor::new(parent)
                .sync_to_parent("libs/sub")
                .is_ok(),
            "sync failed"
        );
        assert_eq!(
            String::from_utf8_lossy(
                &Command::new("git")
                    .args(["rev-parse", "HEAD"])
                    .current_dir(&sm_path)
                    .output()
                    .unwrap()
                    .stdout
            )
            .trim()
            .to_string(),
            remote_hash,
            "submodule caught up to remote after sync"
        );
    }

    #[test]
    fn test_editor_status_with_dirty_submodule() {
        let t = tempfile::tempdir().unwrap();
        let p = setup_repo_with_submodule(t.path());
        std::fs::write(p.join("libs/sub/new-file"), "content").unwrap();
        let issues = GitSubmoduleEditor::new(p).status().unwrap();
        assert!(!issues.is_empty());
        assert_eq!(issues[0].status, "Dirty");
    }
}
