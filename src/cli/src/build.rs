use std::path::Path;

use crate::contract;

/// 输出当前仓库的构建状态（按 scope）。
pub fn status(repo_path: &Path) {
    let c = contract::load(repo_path);

    println!("构建状态");
    println!("{}", "-".repeat(50));

    if c.scopes.is_empty() {
        let lang = contract::detect_by_files(repo_path);
        let root_scope = contract::Scope {
            name: "(root)".into(),
            dir: ".".into(),
            language: lang.clone(),
            framework: String::new(),
            build_tool: contract::BuildTool::Unknown(String::new()),
            registry: contract::Registry::None,
            release: contract::StageRelease::default(),
            test_threshold: None,
            ci_workflow: None,
        };
        let vs = contract::version_status(repo_path, &root_scope);
        let release = c.scope_release(&root_scope);
        print_scope("(root)", repo_path, &lang, &vs, release, &c, None);
    } else {
        for scope in &c.scopes {
            let scope_dir = repo_path.join(&scope.dir);
            if !scope_dir.exists() {
                println!("  [{}]     ⚠ 目录不存在: {}", scope.name, scope.dir);
                continue;
            }
            let lang = c.resolve_language(scope, &scope_dir);
            let vs = contract::version_status(repo_path, scope);
            let release = c.scope_release(scope);
            print_scope(
                &scope.name,
                &scope_dir,
                &lang,
                &vs,
                release,
                &c,
                scope.ci_workflow.as_deref(),
            );
        }
    }

    let dirty = is_working_tree_dirty(repo_path);
    println!(
        "  {}         {}",
        "工作区".to_string(),
        if dirty {
            "⚠ 有未提交变更"
        } else {
            "✅ 干净"
        }
    );
}

fn print_scope(
    name: &str,
    dir: &Path,
    lang: &contract::Language,
    vs: &contract::VersionStatus,
    release: &contract::StageRelease,
    c: &contract::Contract,
    ci_workflow: Option<&str>,
) {
    println!("  [{:<12}] {}", name, lang.as_str());
    println!("    CI:         {}", check_ci(name, ci_workflow));
    println!("    build:      {}", check_syntax(lang, dir));
    match (&vs.tag_version, &vs.config_version) {
        (Some(t), Some(_)) if vs.consistent => println!("    version:    ✅ {}（一致）", t),
        (Some(t), Some(_)) => println!("    version:    ⚠ {}（配置不一致）", t),
        (Some(t), None) => println!("    version:    tag {}（无配置文件）", t),
        (None, Some(_)) => println!("    version:    有配置版本（无 tag）"),
        (None, None) => println!("    version:    暂无发布"),
    }
    for (fname, ver) in &vs.config_files {
        match (ver, &vs.tag_version) {
            (Some(v), Some(t)) if v == t => {
                println!("      {:<15} {} ✅", format!("{}:", fname), v)
            }
            (Some(v), Some(_)) => println!(
                "      {:<15} {} ❌（期望 {})",
                format!("{}:", fname),
                v,
                vs.tag_version.as_deref().unwrap_or("?")
            ),
            (Some(v), None) => println!("      {:<15} {}（无 tag）", format!("{}:", fname), v),
            (None, _) => println!("      {:<15} （未找到版本字段）", format!("{}:", fname)),
        }
    }
    println!("    registry:   {:?}", c.platform.artifact_registry);
    println!("    changelog:  {}", release.changelog);
}

/// 解析 CI workflow 名称。ci_workflow 优先，无则按约定 build-{scope}。
pub fn resolve_workflow(scope: &str, ci_workflow: Option<&str>) -> String {
    match ci_workflow {
        Some(w) => w.to_string(),
        None => format!("build-{}", scope),
    }
}

fn check_ci(scope: &str, ci_workflow: Option<&str>) -> String {
    let workflow = resolve_workflow(scope, ci_workflow);
    let output = match std::process::Command::new("gh")
        .args([
            "run",
            "list",
            "--limit",
            "1",
            "--workflow",
            &workflow,
            "--json",
            "conclusion,displayTitle,headBranch,number",
        ])
        .output()
    {
        Ok(o) if o.status.success() => o.stdout,
        Ok(_) => return "⚠ 无 CI 运行记录".into(),
        Err(_) => return "⚠ gh CLI 未安装".into(),
    };

    let out = String::from_utf8_lossy(&output);
    // JSON: [{"conclusion":"success","displayTitle":"CI","headBranch":"main","number":42}]
    let conclusion = out
        .split("\"conclusion\":")
        .nth(1)
        .and_then(|s| s.split('"').nth(1))
        .unwrap_or("");
    let title = out
        .split("\"displayTitle\":")
        .nth(1)
        .and_then(|s| s.split('"').nth(1))
        .unwrap_or("");
    let branch = out
        .split("\"headBranch\":")
        .nth(1)
        .and_then(|s| s.split('"').nth(1))
        .unwrap_or("?");
    let number: String = out
        .split("\"number\":")
        .nth(1)
        .map(|s| s.chars().take_while(|c| c.is_ascii_digit()).collect())
        .filter(|s: &String| !s.is_empty())
        .unwrap_or_else(|| "?".into());

    if conclusion.is_empty() {
        return "⚠ 无 CI 运行记录".into();
    }
    match conclusion {
        "success" => format!("✅ {} ({} #{})", title, branch, number),
        "failure" => format!("❌ {} ({} #{})", title, branch, number),
        "cancelled" => format!("🔶 {} 已取消", title),
        s => format!("⏳ {} ({}) - {}", title, branch, s),
    }
}

fn check_syntax(lang: &contract::Language, dir: &Path) -> String {
    let (cmd, args, label) = match lang {
        contract::Language::Rust => {
            let mp = dir.join("Cargo.toml");
            if !mp.exists() {
                return "—".into();
            }
            let mp_s = mp.to_string_lossy().to_string();
            (
                "cargo",
                vec!["check".into(), "--manifest-path".into(), mp_s],
                "cargo check",
            )
        }
        contract::Language::Python => {
            if !dir.join("pyproject.toml").exists() {
                return "—".into();
            }
            ("uv".into(), vec!["check".into()], "uv check")
        }
        contract::Language::Go => {
            if !dir.join("go.mod").exists() {
                return "—".into();
            }
            ("go".into(), vec!["vet".into(), "./...".into()], "go vet")
        }
        contract::Language::Dart => {
            if !dir.join("pubspec.yaml").exists() {
                return "—".into();
            }
            ("dart".into(), vec!["analyze".into()], "dart analyze")
        }
        contract::Language::TypeScript => {
            if !dir.join("package.json").exists() {
                return "—".into();
            }
            (
                "npx".into(),
                vec!["tsc".into(), "--noEmit".into()],
                "tsc --noEmit",
            )
        }
        contract::Language::Unknown(_) => return "⚠ 语言未知，跳过".into(),
    };
    match std::process::Command::new(&cmd)
        .args(&args)
        .current_dir(dir)
        .output()
    {
        Ok(o) if o.status.success() => format!("✅ {} 通过", label),
        Ok(_) => format!("❌ {} 失败", label),
        Err(_) => format!("⚠ {} 未安装", cmd),
    }
}

fn is_working_tree_dirty(repo_path: &Path) -> bool {
    match std::process::Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(repo_path)
        .output()
    {
        Ok(o) => !o.stdout.is_empty(),
        Err(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_print_scope_all_ok() {
        let d = tempfile::tempdir().unwrap();
        let c = contract::load(d.path());
        let vs = contract::VersionStatus {
            tag_version: Some("0.1.0".into()),
            config_version: Some("0.1.0".into()),
            consistent: true,
            config_files: vec![("Cargo.toml".into(), Some("0.1.0".into()))],
        };
        let release = contract::StageRelease::default();
        print_scope(
            "test",
            d.path(),
            &contract::Language::Rust,
            &vs,
            &release,
            &c,
            None,
        );
    }

    #[test]
    fn test_is_working_tree_dirty_empty_repo() {
        let d = tempfile::tempdir().unwrap();
        assert!(!is_working_tree_dirty(d.path()));
    }

    #[test]
    fn test_resolve_workflow_default() {
        assert_eq!(resolve_workflow("cli", None), "build-cli");
        assert_eq!(resolve_workflow("studio", None), "build-studio");
    }

    #[test]
    fn test_resolve_workflow_custom() {
        assert_eq!(resolve_workflow("cli", Some("my-pipeline")), "my-pipeline");
        assert_eq!(resolve_workflow("cli", Some("release-ci")), "release-ci");
    }

    #[test]
    fn test_detect_no_contract_yaml() {
        let d = tempfile::tempdir().unwrap();
        let c = contract::load(d.path());
        assert!(c.scopes.is_empty());
    }
}
