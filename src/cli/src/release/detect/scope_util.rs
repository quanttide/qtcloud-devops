use std::collections::HashMap;
use std::path::Path;

use super::tag_util::collect_tags_with_scope;
use super::DetectError;

pub(super) fn detect_project_type(root: &Path) -> &'static str {
    let indicators = [
        root.join("src").is_dir(),
        root.join("Cargo.toml").exists(),
        root.join("package.json").exists(),
        root.join("pyproject.toml").exists(),
        root.join("setup.py").exists(),
        root.join("go.mod").exists(),
        root.join("packages").is_dir(),
        root.join("apps").is_dir(),
    ];
    if indicators.iter().any(|&x| x) {
        "code"
    } else {
        "docs"
    }
}

pub(super) fn detect_single_scope(root: &Path) -> Result<Option<String>, DetectError> {
    let scopes = crate::contract::load_scopes(root);
    let changed_paths = get_changed_paths_since_last_tag(root)?;

    let mut hits: HashMap<String, usize> = HashMap::new();
    for path in &changed_paths {
        for scope in &scopes {
            if path.starts_with(scope.dir.trim_start_matches('/')) || path.contains(&scope.dir) {
                *hits.entry(scope.name.clone()).or_insert(0) += 1;
            }
        }
    }

    let best = hits.iter().max_by_key(|(_, c)| *c);
    if let Some((name, _)) = best {
        return Ok(Some(name.clone()));
    }

    let all_tags = collect_tags_with_scope(root);
    let scoped: Vec<&String> = all_tags.keys().filter(|k| *k != "(root)").collect();
    if scoped.len() == 1 {
        return Ok(Some(scoped[0].clone()));
    }
    if scoped.len() > 1 {
        let names: Vec<&str> = scoped.iter().map(|s| s.as_str()).collect();
        return Err(DetectError::Other(format!(
            "多个 scope 有变更: {:?}，请用 -v 指定",
            names
        )));
    }

    Ok(None)
}

fn get_changed_paths_since_last_tag(root: &Path) -> Result<Vec<String>, DetectError> {
    let tags = collect_tags_with_scope(root);
    let latest_tag = tags
        .iter()
        .filter(|(k, _)| *k != "(root)")
        .find_map(|(_, v)| v.first())
        .or_else(|| tags.get("(root)").and_then(|v| v.first()));

    let range = match latest_tag {
        Some(tag) => format!("{}..HEAD", tag),
        None => return Ok(vec![]),
    };

    let output = super::git_output(&["diff", "--name-only", &range], root).unwrap_or_default();
    Ok(output.lines().map(|s| s.to_string()).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_project_type_code_with_src() {
        let d = tempfile::tempdir().unwrap();
        std::process::Command::new("git")
            .args(["init", "-b", "main"])
            .current_dir(d.path())
            .output()
            .unwrap();
        std::fs::write(d.path().join(".gitkeep"), "").unwrap();
        std::process::Command::new("git")
            .args(["add", "."])
            .current_dir(d.path())
            .output()
            .unwrap();
        std::process::Command::new("git")
            .args(["-c", "user.name=t", "-c", "user.email=t@t", "commit", "-m", "init"])
            .current_dir(d.path())
            .output()
            .unwrap();
        std::fs::create_dir(d.path().join("src")).unwrap();
        assert_eq!(detect_project_type(d.path()), "code");
    }

    #[test]
    fn test_detect_project_type_code_with_cargo() {
        let d = tempfile::tempdir().unwrap();
        std::fs::write(d.path().join("Cargo.toml"), "").unwrap();
        assert_eq!(detect_project_type(d.path()), "code");
    }

    #[test]
    fn test_detect_project_type_docs() {
        let d = tempfile::tempdir().unwrap();
        assert_eq!(detect_project_type(d.path()), "docs");
    }

    #[test]
    fn test_detect_project_type_no_workdir() {
        let d = tempfile::tempdir().unwrap();
        assert_eq!(detect_project_type(d.path()), "docs");
    }

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

    fn git_commit_file(repo_path: &std::path::Path, path: &str, content: &str) {
        std::fs::write(repo_path.join(path), content).unwrap();
        std::process::Command::new("git")
            .args(["add", path])
            .current_dir(repo_path)
            .output()
            .unwrap();
        std::process::Command::new("git")
            .args(["-c", "user.name=t", "-c", "user.email=t@t", "commit", "-m", &format!("update {path}")])
            .current_dir(repo_path)
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
    fn test_get_changed_paths_no_tag() {
        let d = tempfile::tempdir().unwrap();
        git_init_detect(d.path());
        git_commit_file(d.path(), "new.txt", "hello");
        let paths = get_changed_paths_since_last_tag(d.path()).unwrap();
        assert!(paths.is_empty(), "无 tag 时无法 diff，应返回空");
    }

    #[test]
    fn test_get_changed_paths_after_tag() {
        let d = tempfile::tempdir().unwrap();
        git_init_detect(d.path());
        git_commit_file(d.path(), "initial.txt", "initial");
        git_tag(d.path(), "v1.0.0");
        git_commit_file(d.path(), "added.txt", "added");
        git_commit_file(d.path(), "modified.txt", "modified");
        let paths = get_changed_paths_since_last_tag(d.path()).unwrap();
        assert!(paths.contains(&"added.txt".to_string()));
        assert!(paths.contains(&"modified.txt".to_string()));
        assert!(!paths.contains(&"initial.txt".to_string()), "tag 前的文件不应出现");
    }

    #[test]
    fn test_get_changed_paths_no_new_commits() {
        let d = tempfile::tempdir().unwrap();
        git_init_detect(d.path());
        git_tag(d.path(), "v1.0.0");
        let paths = get_changed_paths_since_last_tag(d.path()).unwrap();
        assert!(paths.is_empty(), "无新提交应返回空");
    }

    #[test]
    fn test_detect_single_scope_no_changes() {
        let d = tempfile::tempdir().unwrap();
        git_init_detect(d.path());
        let scope = detect_single_scope(d.path()).unwrap();
        assert_eq!(scope, None, "无 tag 无 contract 应返回 None");
    }

    #[test]
    fn test_detect_single_scope_fallback_to_tags() {
        let d = tempfile::tempdir().unwrap();
        git_init_detect(d.path());
        std::fs::create_dir_all(d.path().join("packages/cli")).unwrap();
        git_commit_file(d.path(), "packages/cli/readme.md", "cli");
        git_tag(d.path(), "cli/v0.1.0");
        git_commit_file(d.path(), "readme.md", "root");
        let scope = detect_single_scope(d.path()).unwrap();
        assert_eq!(scope.as_deref(), Some("cli"), "唯一有 tag 的 scope");
    }

    #[test]
    fn test_detect_single_scope_root_tag() {
        let d = tempfile::tempdir().unwrap();
        git_init_detect(d.path());
        git_commit_file(d.path(), "file.txt", "content");
        git_tag(d.path(), "v1.0.0");
        let scope = detect_single_scope(d.path()).unwrap();
        assert_eq!(scope, None, "只有 root tag 应返回 None");
    }
}
