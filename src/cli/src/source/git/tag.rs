use std::collections::HashMap;
use std::path::Path;

pub fn get_latest_tag_for_scope(root: &Path, scope: Option<&str>) -> Option<String> {
    let scope_name = scope.unwrap_or("");
    quanttide_devops::source::git::tag::latest_tag(root, scope_name)
        .ok()
        .flatten()
}

pub fn collect_tags_with_scope(root: &Path) -> HashMap<String, Vec<String>> {
    use quanttide_devops::source::git::tag::{parse_semver_tag, GixTagSource, TagSource};
    let source = GixTagSource::new(root);
    let all = match source.all_tags() {
        Ok(t) => t,
        Err(_) => return HashMap::new(),
    };
    let mut groups: HashMap<String, Vec<(Option<semver::Version>, String)>> = HashMap::new();
    for tag in &all {
        let (scope, _) = tag.split_once('/').unwrap_or(("", tag));
        let scope_name = if scope.is_empty() {
            "(root)".to_string()
        } else {
            scope.to_string()
        };
        groups
            .entry(scope_name)
            .or_default()
            .push((parse_semver_tag(tag), tag.clone()));
    }
    let mut result: HashMap<String, Vec<String>> = HashMap::new();
    for (scope, mut entries) in groups {
        entries.sort_by(|a, b| b.0.cmp(&a.0));
        result.insert(scope, entries.into_iter().map(|(_, t)| t).collect());
    }
    result
}

pub fn parse_tag(tag: &str) -> (Option<String>, &str) {
    if let Some((scope, ver)) = tag.split_once('/') {
        (Some(scope.to_string()), ver)
    } else {
        (None, tag)
    }
}

pub fn parse_version(s: &str) -> Result<(u32, u32, u32, Option<String>, Option<u32>), String> {
    let s = s.strip_prefix('v').unwrap_or(s);
    let (ver_part, pre_part) = s.split_once('-').unwrap_or((s, ""));
    let parts: Vec<&str> = ver_part.split('.').collect();
    if parts.len() != 3 {
        return Err(format!("版本号格式错误: {}，需要 X.Y.Z", s));
    }
    let major = parts[0]
        .parse()
        .map_err(|_| "major 不是数字".to_string())?;
    let minor = parts[1]
        .parse()
        .map_err(|_| "minor 不是数字".to_string())?;
    let patch: u32 = parts[2]
        .parse()
        .map_err(|_| "patch 不是数字".to_string())?;
    let (pre_stage, pre_num) = if pre_part.is_empty() {
        (None, None)
    } else {
        let sp: Vec<&str> = pre_part.split('.').collect();
        let stage = sp.first().map(|s| s.to_string());
        let num = sp.get(1).and_then(|s| s.parse().ok());
        (stage, num)
    };
    Ok((major, minor, patch, pre_stage, pre_num))
}

pub fn build_version(parts: &VersionParts, increment: &str, prerelease: Option<&str>) -> String {
    if let Some(stage) = &parts.pre_stage {
        let next = parts.pre_num.unwrap_or(0) + 1;
        return format!(
            "v{}.{}.{}-{}.{}",
            parts.major, parts.minor, parts.patch, stage, next
        );
    }

    match (increment, prerelease) {
        ("minor", Some(pr)) => format!("v{}.{}.{}-{}.1", parts.major, parts.minor + 1, 0, pr),
        ("minor", None) => format!("v{}.{}.{}", parts.major, parts.minor + 1, 0),
        _ => format!("v{}.{}.{}", parts.major, parts.minor, parts.patch + 1),
    }
}

pub fn apply_scope_prefix(scope: Option<&str>, version: &str) -> String {
    match scope {
        Some(s) if !s.is_empty() && s != "(root)" => format!("{}/{}", s, version),
        _ => version.to_string(),
    }
}

pub struct VersionParts {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
    pub pre_stage: Option<String>,
    pub pre_num: Option<u32>,
}

/// 创建轻量 tag（`git tag <version>`）。已存在则跳过（幂等）。
pub fn create_tag(version: &str, repo_path: &Path) -> bool {
    let repo = match git2::Repository::open(repo_path) {
        Ok(r) => r,
        Err(_) => return false,
    };
    let refname = format!("refs/tags/{}", version);
    if repo.find_reference(&refname).is_ok() {
        return true;
    }
    let head_id = match repo.head().ok().and_then(|h| h.target()) {
        Some(id) => id,
        None => return false,
    };
    let result = repo.reference(&refname, head_id, false, "");
    result.is_ok()
}

fn tag_push_refspec(version: &str) -> String {
    format!("refs/tags/{}", version)
}

/// 推送 tag 到远程（需要网络）。
pub fn push_tag(version: &str, repo_path: &Path) -> Result<(), String> {
    if !crate::source::git::is_git_repo(repo_path) {
        return Err("不是 git 仓库".into());
    }
    if !crate::source::git::git_check(&["remote", "get-url", "origin"], repo_path) {
        return Ok(());
    }
    let output = std::process::Command::new("git")
        .args(["push", "origin", &tag_push_refspec(version)])
        .current_dir(repo_path)
        .output()
        .map_err(|e| format!("执行 git push 失败: {}", e))?;
    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        Err(format!("推送标签失败: {}", stderr))
    }
}

/// 回滚 tag：删除本地和远端 tag。
pub fn rollback_tag(version: &str, repo_path: &Path) {
    let local_ok = delete_local_tag(version, repo_path);
    let remote_ok = delete_remote_tag(version, repo_path);
    if local_ok && remote_ok {
        eprintln!("已回滚标签 {}", version);
    }
}

/// 删除本地 tag（`git tag -d <version>`）。不存在也算成功。
pub fn delete_local_tag(version: &str, repo_path: &Path) -> bool {
    let repo = match git2::Repository::open(repo_path) {
        Ok(r) => r,
        Err(_) => return true,
    };
    let refname = format!("refs/tags/{}", version);
    repo.find_reference(&refname)
        .ok()
        .and_then(|mut r| r.delete().ok())
        .is_some()
}

/// 删除远端 tag（等价于 `git push --delete origin <version>`）。
pub fn delete_remote_tag(version: &str, repo_path: &Path) -> bool {
    let output = std::process::Command::new("git")
        .args(["push", "--delete", "origin", version])
        .current_dir(repo_path)
        .output();
    match output {
        Ok(out) => out.status.success(),
        Err(_) => false,
    }
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

    // ── parse_tag ─────────────────────────────────────────────────

    #[test]
    fn test_parse_tag_scoped() {
        assert_eq!(parse_tag("cli/v0.8.4"), (Some("cli".into()), "v0.8.4"));
    }

    #[test]
    fn test_parse_tag_root() {
        assert_eq!(parse_tag("v0.1.0"), (None, "v0.1.0"));
    }

    #[test]
    fn test_parse_tag_empty() {
        assert_eq!(parse_tag(""), (None, ""));
    }

    #[test]
    fn test_parse_tag_multiple_slashes() {
        assert_eq!(
            parse_tag("scope/v0.1.0-rc.1"),
            (Some("scope".into()), "v0.1.0-rc.1")
        );
    }

    // ── parse_version ─────────────────────────────────────────────

    #[test]
    fn test_parse_version_formal() {
        let (ma, mi, pa, st, nu) = parse_version("0.8.4").unwrap();
        assert_eq!((ma, mi, pa), (0, 8, 4));
        assert!(st.is_none());
        assert!(nu.is_none());
    }

    #[test]
    fn test_parse_version_prerelease() {
        let (ma, mi, pa, st, nu) = parse_version("0.9.0-rc.1").unwrap();
        assert_eq!((ma, mi, pa), (0, 9, 0));
        assert_eq!(st.as_deref(), Some("rc"));
        assert_eq!(nu, Some(1));
    }

    #[test]
    fn test_parse_version_with_v_prefix() {
        let (ma, mi, pa, _, _) = parse_version("v0.8.4").unwrap();
        assert_eq!((ma, mi, pa), (0, 8, 4));
    }

    #[test]
    fn test_parse_version_bad_format() {
        assert!(parse_version("abc").is_err());
        assert!(parse_version("0.1").is_err());
    }

    #[test]
    fn test_parse_version_alpha() {
        let (ma, mi, pa, st, nu) = parse_version("1.0.0-alpha.1").unwrap();
        assert_eq!((ma, mi, pa), (1, 0, 0));
        assert_eq!(st.as_deref(), Some("alpha"));
        assert_eq!(nu, Some(1));
    }

    #[test]
    fn test_parse_version_beta() {
        let (ma, mi, pa, st, nu) = parse_version("0.5.0-beta.2").unwrap();
        assert_eq!((ma, mi, pa), (0, 5, 0));
        assert_eq!(st.as_deref(), Some("beta"));
        assert_eq!(nu, Some(2));
    }

    #[test]
    fn test_parse_version_prerelease_no_number() {
        let (ma, mi, pa, st, nu) = parse_version("1.2.3-rc").unwrap();
        assert_eq!((ma, mi, pa), (1, 2, 3));
        assert_eq!(st.as_deref(), Some("rc"));
        assert_eq!(nu, None);
    }

    #[test]
    fn test_parse_version_non_numeric_parts() {
        assert!(parse_version("a.b.c").is_err());
        assert!(parse_version("1.x.3").is_err());
    }

    // ── build_version ────────────────────────────────────────────

    #[test]
    fn test_build_version_patch() {
        assert_eq!(
            build_version(
                &VersionParts {
                    major: 0,
                    minor: 8,
                    patch: 4,
                    pre_stage: None,
                    pre_num: None
                },
                "patch",
                None
            ),
            "v0.8.5"
        );
    }

    #[test]
    fn test_build_version_minor_rc() {
        assert_eq!(
            build_version(
                &VersionParts {
                    major: 0,
                    minor: 8,
                    patch: 4,
                    pre_stage: None,
                    pre_num: None
                },
                "minor",
                Some("rc")
            ),
            "v0.9.0-rc.1"
        );
    }

    #[test]
    fn test_build_version_prerelease_increment() {
        assert_eq!(
            build_version(
                &VersionParts {
                    major: 0,
                    minor: 9,
                    patch: 0,
                    pre_stage: Some("rc".into()),
                    pre_num: Some(1)
                },
                "patch",
                None
            ),
            "v0.9.0-rc.2"
        );
    }

    #[test]
    fn test_build_version_minor_formal() {
        assert_eq!(
            build_version(
                &VersionParts {
                    major: 0,
                    minor: 8,
                    patch: 4,
                    pre_stage: None,
                    pre_num: None
                },
                "minor",
                None
            ),
            "v0.9.0"
        );
    }

    #[test]
    fn test_build_version_minor_with_alpha() {
        assert_eq!(
            build_version(
                &VersionParts {
                    major: 0,
                    minor: 1,
                    patch: 0,
                    pre_stage: None,
                    pre_num: None
                },
                "minor",
                Some("alpha")
            ),
            "v0.2.0-alpha.1"
        );
    }

    #[test]
    fn test_build_version_no_prerelease_info() {
        assert_eq!(
            build_version(
                &VersionParts {
                    major: 1,
                    minor: 0,
                    patch: 0,
                    pre_stage: None,
                    pre_num: None
                },
                "patch",
                None
            ),
            "v1.0.1"
        );
    }

    #[test]
    fn test_build_version_patch_with_same_stage() {
        assert_eq!(
            build_version(
                &VersionParts {
                    major: 1,
                    minor: 0,
                    patch: 0,
                    pre_stage: Some("beta".into()),
                    pre_num: Some(3)
                },
                "patch",
                None
            ),
            "v1.0.0-beta.4"
        );
    }

    // ── apply_scope_prefix ────────────────────────────────────────

    #[test]
    fn test_apply_scope_prefix_with_scope() {
        assert_eq!(apply_scope_prefix(Some("cli"), "v0.1.0"), "cli/v0.1.0");
    }

    #[test]
    fn test_apply_scope_prefix_root() {
        assert_eq!(apply_scope_prefix(Some("(root)"), "v0.1.0"), "v0.1.0");
    }

    #[test]
    fn test_apply_scope_prefix_none() {
        assert_eq!(apply_scope_prefix(None, "v0.1.0"), "v0.1.0");
    }

    #[test]
    fn test_apply_scope_prefix_empty() {
        assert_eq!(apply_scope_prefix(Some(""), "v0.1.0"), "v0.1.0");
    }

    // ── collect_tags_with_scope ───────────────────────────────────

    fn git_init_detect(path: &std::path::Path) {
        std::process::Command::new("git")
            .args(["init", "-b", "main"])
            .current_dir(path)
            .output()
            .unwrap();
        std::fs::write(path.join(".gitkeep"), "").unwrap();
        std::process::Command::new("git")
            .args(["add", "."])
            .current_dir(path)
            .output()
            .unwrap();
        std::process::Command::new("git")
            .args(["-c", "user.name=t", "-c", "user.email=t@t", "commit", "-m", "init"])
            .current_dir(path)
            .output()
            .unwrap();
    }

    fn git_tag(repo_path: &std::path::Path, tag: &str) {
        std::process::Command::new("git")
            .args(["tag", tag])
            .current_dir(repo_path)
            .output()
            .unwrap();
    }

    #[test]
    fn test_collect_tags_empty_repo() {
        let d = tempfile::tempdir().unwrap();
        git_init_detect(d.path());
        let tags = collect_tags_with_scope(d.path());
        assert!(tags.is_empty());
    }

    #[test]
    fn test_collect_tags_root_only() {
        let d = tempfile::tempdir().unwrap();
        git_init_detect(d.path());
        git_tag(d.path(), "v1.0.0");
        git_tag(d.path(), "v1.1.0");
        let tags = collect_tags_with_scope(d.path());
        assert_eq!(tags.len(), 1);
        assert!(tags.contains_key("(root)"));
        assert_eq!(tags["(root)"], vec!["v1.1.0", "v1.0.0"]);
    }

    #[test]
    fn test_collect_tags_scoped_ordered() {
        let d = tempfile::tempdir().unwrap();
        git_init_detect(d.path());
        git_tag(d.path(), "cli/v0.2.0");
        git_tag(d.path(), "cli/v0.3.0");
        git_tag(d.path(), "cli/v0.1.0");
        let tags = collect_tags_with_scope(d.path());
        assert_eq!(tags.len(), 1);
        assert!(tags.contains_key("cli"));
        assert_eq!(tags["cli"], vec!["cli/v0.3.0", "cli/v0.2.0", "cli/v0.1.0"]);
    }

    #[test]
    fn test_collect_tags_multi_scope_with_prerelease() {
        let d = tempfile::tempdir().unwrap();
        git_init_detect(d.path());
        git_tag(d.path(), "v0.5.0");
        git_tag(d.path(), "cli/v0.2.0-rc.1");
        git_tag(d.path(), "cli/v0.2.0");
        git_tag(d.path(), "sdk/v0.1.0-alpha.1");
        git_tag(d.path(), "sdk/v0.1.0-beta.1");
        git_tag(d.path(), "sdk/v0.1.0");
        let tags = collect_tags_with_scope(d.path());
        assert_eq!(tags.len(), 3);
        assert_eq!(tags["cli"][0], "cli/v0.2.0", "正式版应排首位");
        assert_eq!(tags["cli"][1], "cli/v0.2.0-rc.1");
        assert!(tags["sdk"][0].contains("v0.1.0"), "正式版应排首位");
        assert!(tags["sdk"][1].contains("beta"), "beta 应在 alpha 之前");
        assert!(tags["sdk"][2].contains("alpha"), "alpha 应排最后");
        assert_eq!(tags["(root)"][0], "v0.5.0");
    }

    // ── get_latest_tag_for_scope ───────────────────────────────────

    #[test]
    fn test_get_latest_tag_root_scope() {
        let d = tempfile::tempdir().unwrap();
        git_init_detect(d.path());
        git_tag(d.path(), "v1.0.0");
        git_tag(d.path(), "v2.0.0");
        assert_eq!(
            get_latest_tag_for_scope(d.path(), None).as_deref(),
            Some("v2.0.0")
        );
        assert_eq!(
            get_latest_tag_for_scope(d.path(), Some("(root)")).as_deref(),
            Some("v2.0.0")
        );
    }

    #[test]
    fn test_get_latest_tag_scoped() {
        let d = tempfile::tempdir().unwrap();
        git_init_detect(d.path());
        git_tag(d.path(), "cli/v0.1.0");
        git_tag(d.path(), "cli/v0.2.0");
        git_tag(d.path(), "sdk/v0.5.0");
        assert_eq!(
            get_latest_tag_for_scope(d.path(), Some("cli")).as_deref(),
            Some("cli/v0.2.0")
        );
        assert_eq!(
            get_latest_tag_for_scope(d.path(), Some("sdk")).as_deref(),
            Some("sdk/v0.5.0")
        );
        assert_eq!(
            get_latest_tag_for_scope(d.path(), Some("nosuch")).as_deref(),
            None
        );
    }

    // ── create_tag ────────────────────────────────────────────────

    #[test]
    fn test_create_tag_in_non_git_dir() {
        let d = tempfile::tempdir().unwrap();
        assert!(!create_tag("v1.0.0", d.path()));
    }

    #[test]
    fn test_create_tag_idempotent() {
        let d = tempfile::tempdir().unwrap();
        git_init(d.path());
        git_commit(d.path(), "init");
        assert!(create_tag("v1.0.0", d.path()));
        assert!(create_tag("v1.0.0", d.path()));
    }

    #[test]
    fn test_push_tag_in_non_git_dir() {
        let d = tempfile::tempdir().unwrap();
        let result = push_tag("v1.0.0", d.path());
        assert!(result.is_err());
    }

    #[test]
    fn test_tag_push_refspec_scoped() {
        assert_eq!(tag_push_refspec("cli/v1.0.0"), "refs/tags/cli/v1.0.0");
    }

    #[test]
    fn test_tag_push_refspec_simple() {
        assert_eq!(tag_push_refspec("v1.0.0"), "refs/tags/v1.0.0");
    }

    #[test]
    fn test_tag_push_refspec_root_tag() {
        assert_eq!(tag_push_refspec("v0.1.0"), "refs/tags/v0.1.0");
    }

    #[test]
    fn test_rollback_tag_removes_tag() {
        let d = tempfile::tempdir().unwrap();
        git_init(d.path());
        git_commit(d.path(), "init");
        create_tag("v1.0.0", d.path());
        assert!(git2::Repository::open(d.path())
            .unwrap()
            .find_reference("refs/tags/v1.0.0")
            .is_ok());
        rollback_tag("v1.0.0", d.path());
        assert!(git2::Repository::open(d.path())
            .unwrap()
            .find_reference("refs/tags/v1.0.0")
            .is_err());
    }
}
