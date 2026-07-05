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
