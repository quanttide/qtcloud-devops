use std::path::Path;

use crate::build::{is_working_tree_dirty, CiRun, ScopeInfo};
use crate::contract;

/// 输出当前仓库的构建状态（按 scope）。
pub fn status(repo_path: &Path) {
    let mut stdout = std::io::stdout();
    status_to(&mut stdout, repo_path).ok();
}

pub fn status_to(writer: &mut impl std::io::Write, repo_path: &Path) -> std::io::Result<()> {
    let c = contract::load(repo_path);
    let mut o = format!("构建状态\n{}\n", "-".repeat(50));

    if c.scopes.is_empty() {
        append_root_status(&mut o, repo_path, &c);
    } else {
        for scope in &c.scopes {
            append_scope_status(&mut o, scope, repo_path, &c);
        }
    }

    o.push_str(&format!(
        "  工作区         {}\n",
        if is_working_tree_dirty(repo_path) {
            "⚠ 有未提交变更"
        } else {
            "✅ 干净"
        }
    ));
    write!(writer, "{}", o)
}

fn append_root_status(o: &mut String, repo_path: &Path, c: &contract::Contract) {
    let lang = contract::detect_languages(repo_path)
        .into_iter()
        .next()
        .unwrap_or(contract::Language::Unknown(String::new()));
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
    let vs = load_version_state(repo_path, &root_scope);
    let release = c.scope_release(&root_scope);
    o.push_str(&build_scope_str(&ScopeInfo {
        name: "(root)",
        dir: repo_path,
        lang: &lang,
        c,
        vs: &vs,
        release: &release,
    }));
}

fn append_scope_status(
    o: &mut String,
    scope: &contract::Scope,
    repo_path: &Path,
    c: &contract::Contract,
) {
    let scope_dir = repo_path.join(&scope.dir);
    if !scope_dir.exists() {
        o.push_str(&format!(
            "  [{}]     ⚠ 目录不存在: {}\n",
            scope.name, scope.dir
        ));
        return;
    }
    let lang = c.resolve_language(scope, &scope_dir);
    let vs = load_version_state(repo_path, scope);
    let release = c.scope_release(scope);
    o.push_str(&build_scope_str(&ScopeInfo {
        name: &scope.name,
        dir: &scope_dir,
        lang: &lang,
        c,
        vs: &vs,
        release: &release,
    }));
}

fn load_version_state(repo_path: &Path, scope: &contract::Scope) -> contract::VersionState {
    contract::verify_version(repo_path, scope).unwrap_or_else(|e| {
        eprintln!("  ⚠ 版本状态检查失败: {}", e);
        contract::VersionState {
            tag_version: None,
            config_version: None,
            consistent: false,
            config_files: vec![],
        }
    })
}

/// 构建 scope 的状态字符串。
pub(crate) fn build_scope_str(info: &ScopeInfo) -> String {
    let version_line = match (&info.vs.tag_version, &info.vs.config_version) {
        (Some(t), Some(_)) if info.vs.consistent => format!("    version:    ✅ {}（一致）", t),
        (Some(t), Some(_)) => format!("    version:    ⚠ {}（配置不一致）", t),
        (Some(t), None) => format!("    version:    tag {}（无配置文件）", t),
        (None, Some(_)) => "    version:    有配置版本（无 tag）".into(),
        (None, None) => "    version:    暂无发布".into(),
    };
    let config_lines: String = info
        .vs
        .config_files
        .iter()
        .map(|(fname, ver)| match (ver, &info.vs.tag_version) {
            (Some(v), Some(t)) if v == t => {
                format!("      {:<15} {} ✅\n", format!("{}:", fname), v)
            }
            (Some(v), Some(_)) => format!(
                "      {:<15} {} ❌（期望 {})\n",
                format!("{}:", fname),
                v,
                info.vs.tag_version.as_deref().unwrap_or("?")
            ),
            (Some(v), None) => format!("      {:<15} {}（无 tag）\n", format!("{}:", fname), v),
            (None, _) => format!("      {:<15} （未找到版本字段）\n", format!("{}:", fname)),
        })
        .collect::<String>();

    format!(
        "  [{:<12}] {}\n    CI:         {}\n    build:      {}\n{}\n{}    registry:   {:?}\n    deps:       {}\n    changelog:  {}\n",
        info.name, info.lang.as_str(),
        check_ci(info.name, None),
        check_syntax(info.lang, info.dir),
        version_line,
        config_lines,
        info.c.platform.artifact_registry,
        crate::build::check_dependencies(info.dir),
        info.release.changelog,
    )
}

// ── CI helpers (used by build_scope_str) ─────────────────────────

fn check_ci(scope: &str, ci_workflow: Option<&str>) -> String {
    let workflow = crate::build::resolve_workflow(scope, ci_workflow);
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

/// 从 `gh run list` 的输出解析运行记录。
pub(crate) fn parse_gh_run_list(output: &str) -> Option<CiRun> {
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

// ── Syntax check helpers (used by build_scope_str) ───────────────

fn check_syntax(lang: &contract::Language, dir: &Path) -> String {
    let (cmd, label) = match crate::build::check_command(lang) {
        Some(x) => x,
        None => return "⚠ 语言未知，跳过".into(),
    };
    if let Some(mf) = crate::build::check_manifest_file(lang) {
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

pub(crate) fn check_args(lang: &contract::Language, dir: &Path) -> Option<Vec<String>> {
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
