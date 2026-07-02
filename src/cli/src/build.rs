use std::path::Path;

use crate::contract;

/// CI 运行记录。
#[derive(Debug, PartialEq)]
struct CiRun {
    conclusion: String,
    title: String,
    branch: String,
    number: String,
}

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

/// 从 `gh run list --json conclusion,displayTitle,headBranch,number` 的输出解析运行记录。
///
/// 输入格式：`[{"conclusion":"success","displayTitle":"CI","headBranch":"main","number":42}]`
/// 返回 None 表示无有效记录（空数组或格式异常）。
fn parse_gh_run_list(output: &str) -> Option<CiRun> {
    let conclusion = output
        .split("\"conclusion\":")
        .nth(1)
        .and_then(|s| s.split('"').nth(1))?;
    if conclusion.is_empty() {
        return None;
    }
    let title = output
        .split("\"displayTitle\":")
        .nth(1)
        .and_then(|s| s.split('"').nth(1))
        .unwrap_or("");
    let branch = output
        .split("\"headBranch\":")
        .nth(1)
        .and_then(|s| s.split('"').nth(1))
        .unwrap_or("?");
    let number: String = output
        .split("\"number\":")
        .nth(1)
        .map(|s| s.chars().take_while(|c| c.is_ascii_digit()).collect())
        .filter(|s: &String| !s.is_empty())
        .unwrap_or_else(|| "?".into());

    Some(CiRun {
        conclusion: conclusion.to_string(),
        title: title.to_string(),
        branch: branch.to_string(),
        number,
    })
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
    match parse_gh_run_list(&out) {
        Some(run) => match run.conclusion.as_str() {
            "success" => format!("✅ {} ({} #{})", run.title, run.branch, run.number),
            "failure" => format!("❌ {} ({} #{})", run.title, run.branch, run.number),
            "cancelled" => format!("🔶 {} 已取消", run.title),
            s => format!("⏳ {} ({}) - {}", run.title, run.branch, s),
        },
        None => "⚠ 无 CI 运行记录".into(),
    }
}

/// 返回语言对应的构建检查命令和标签，None 表示不支持。
fn check_command(lang: &contract::Language) -> Option<(&'static str, &'static str)> {
    match lang {
        contract::Language::Rust => Some(("cargo", "cargo check")),
        contract::Language::Python => Some(("uv", "uv check")),
        contract::Language::Go => Some(("go", "go vet")),
        contract::Language::Dart => Some(("dart", "dart analyze")),
        contract::Language::TypeScript => Some(("npx", "tsc --noEmit")),
        contract::Language::Unknown(_) => None,
    }
}

/// 返回语言对应的清单文件名（存在验证用），None 表示不需要验证。
fn check_manifest_file(lang: &contract::Language) -> Option<&'static str> {
    match lang {
        contract::Language::Rust => Some("Cargo.toml"),
        contract::Language::Python => Some("pyproject.toml"),
        contract::Language::Go => Some("go.mod"),
        contract::Language::Dart => Some("pubspec.yaml"),
        contract::Language::TypeScript => Some("package.json"),
        contract::Language::Unknown(_) => None,
    }
}

/// 构建检查参数（依赖目录，因为 Rust 需要 --manifest-path）。
fn check_args(lang: &contract::Language, dir: &Path) -> Option<Vec<String>> {
    match lang {
        contract::Language::Rust => {
            let mp = dir.join("Cargo.toml");
            Some(vec![
                "check".into(),
                "--manifest-path".into(),
                mp.to_string_lossy().to_string(),
            ])
        }
        contract::Language::Python => Some(vec!["check".into()]),
        contract::Language::Go => Some(vec!["vet".into(), "./...".into()]),
        contract::Language::Dart => Some(vec!["analyze".into()]),
        contract::Language::TypeScript => Some(vec!["tsc".into(), "--noEmit".into()]),
        contract::Language::Unknown(_) => None,
    }
}

fn check_syntax(lang: &contract::Language, dir: &Path) -> String {
    let (cmd, label) = match check_command(lang) {
        Some(x) => x,
        None => return "⚠ 语言未知，跳过".into(),
    };
    if let Some(mf) = check_manifest_file(lang) {
        if !dir.join(mf).exists() {
            return "—".into();
        }
    }
    let args = match check_args(lang, dir) {
        Some(a) => a,
        None => return "⚠ 语言未知，跳过".into(),
    };
    match std::process::Command::new(cmd)
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

    // ── parse_gh_run_list ─────────────────────────────────────

    #[test]
    fn test_parse_gh_run_list_success() {
        let out =
            r#"[{"conclusion":"success","displayTitle":"CI","headBranch":"main","number":42}]"#;
        let run = parse_gh_run_list(out).unwrap();
        assert_eq!(run.conclusion, "success");
        assert_eq!(run.title, "CI");
        assert_eq!(run.branch, "main");
        assert_eq!(run.number, "42");
    }

    #[test]
    fn test_parse_gh_run_list_failure() {
        let out =
            r#"[{"conclusion":"failure","displayTitle":"Build","headBranch":"feat/x","number":7}]"#;
        let run = parse_gh_run_list(out).unwrap();
        assert_eq!(run.conclusion, "failure");
        assert_eq!(run.title, "Build");
        assert_eq!(run.branch, "feat/x");
        assert_eq!(run.number, "7");
    }

    #[test]
    fn test_parse_gh_run_list_cancelled() {
        let out =
            r#"[{"conclusion":"cancelled","displayTitle":"CI","headBranch":"main","number":99}]"#;
        let run = parse_gh_run_list(out).unwrap();
        assert_eq!(run.conclusion, "cancelled");
        assert_eq!(run.number, "99");
    }

    #[test]
    fn test_parse_gh_run_list_empty_array() {
        assert!(parse_gh_run_list("[]").is_none());
    }

    #[test]
    fn test_parse_gh_run_list_empty_stdout() {
        assert!(parse_gh_run_list("").is_none());
    }

    #[test]
    fn test_parse_gh_run_list_no_number() {
        // 一些旧版本 gh 可能不返回 number
        let out = r#"[{"conclusion":"success","displayTitle":"CI","headBranch":"main"}]"#;
        let run = parse_gh_run_list(out).unwrap();
        assert_eq!(run.number, "?");
    }

    #[test]
    fn test_parse_gh_run_list_unknown_conclusion() {
        let out =
            r#"[{"conclusion":"neutral","displayTitle":"Check","headBranch":"main","number":1}]"#;
        let run = parse_gh_run_list(out).unwrap();
        assert_eq!(run.conclusion, "neutral");
        assert_eq!(run.title, "Check");
    }

    // ── check_command ─────────────────────────────────────────

    #[test]
    fn test_check_command_all_languages() {
        assert_eq!(
            check_command(&contract::Language::Rust),
            Some(("cargo", "cargo check"))
        );
        assert_eq!(
            check_command(&contract::Language::Python),
            Some(("uv", "uv check"))
        );
        assert_eq!(
            check_command(&contract::Language::Go),
            Some(("go", "go vet"))
        );
        assert_eq!(
            check_command(&contract::Language::Dart),
            Some(("dart", "dart analyze"))
        );
        assert_eq!(
            check_command(&contract::Language::TypeScript),
            Some(("npx", "tsc --noEmit"))
        );
        assert_eq!(
            check_command(&contract::Language::Unknown("?".into())),
            None
        );
    }

    // ── check_manifest_file ────────────────────────────────────

    #[test]
    fn test_check_manifest_file_all_languages() {
        assert_eq!(
            check_manifest_file(&contract::Language::Rust),
            Some("Cargo.toml")
        );
        assert_eq!(
            check_manifest_file(&contract::Language::Python),
            Some("pyproject.toml")
        );
        assert_eq!(check_manifest_file(&contract::Language::Go), Some("go.mod"));
        assert_eq!(
            check_manifest_file(&contract::Language::Dart),
            Some("pubspec.yaml")
        );
        assert_eq!(
            check_manifest_file(&contract::Language::TypeScript),
            Some("package.json")
        );
        assert_eq!(
            check_manifest_file(&contract::Language::Unknown("?".into())),
            None
        );
    }

    // ── check_args ─────────────────────────────────────────────

    #[test]
    fn test_check_args_rust_includes_manifest_path() {
        let d = tempfile::tempdir().unwrap();
        let args = check_args(&contract::Language::Rust, d.path()).unwrap();
        assert!(args.contains(&"check".to_string()));
        assert!(args.iter().any(|a| a.contains("Cargo.toml")));
    }

    #[test]
    fn test_check_args_unknown_returns_none() {
        let d = tempfile::tempdir().unwrap();
        assert!(check_args(&contract::Language::Unknown("?".into()), d.path()).is_none());
    }
}
