use std::path::Path;

use crate::contract;
use crate::build::{ScopeInfo, is_working_tree_dirty};

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
        if is_working_tree_dirty(repo_path) { "⚠ 有未提交变更" } else { "✅ 干净" }
    ));
    write!(writer, "{}", o)
}

fn append_root_status(o: &mut String, repo_path: &Path, c: &contract::Contract) {
    let lang = contract::detect_languages(repo_path).into_iter().next()
        .unwrap_or(contract::Language::Unknown(String::new()));
    let root_scope = contract::Scope {
        name: "(root)".into(), dir: ".".into(), language: lang.clone(),
        framework: String::new(), build_tool: contract::BuildTool::Unknown(String::new()),
        registry: contract::Registry::None, release: contract::StageRelease::default(),
        test_threshold: None, ci_workflow: None,
    };
    let vs = load_version_state(repo_path, &root_scope);
    let release = c.scope_release(&root_scope);
    o.push_str(&build_scope_str(&ScopeInfo {
        name: "(root)", dir: repo_path, lang: &lang, c, vs: &vs, release: &release,
    }));
}

fn append_scope_status(o: &mut String, scope: &contract::Scope, repo_path: &Path, c: &contract::Contract) {
    let scope_dir = repo_path.join(&scope.dir);
    if !scope_dir.exists() {
        o.push_str(&format!("  [{}]     ⚠ 目录不存在: {}\n", scope.name, scope.dir));
        return;
    }
    let lang = c.resolve_language(scope, &scope_dir);
    let vs = load_version_state(repo_path, scope);
    let release = c.scope_release(scope);
    o.push_str(&build_scope_str(&ScopeInfo {
        name: &scope.name, dir: &scope_dir, lang: &lang, c, vs: &vs, release: &release,
    }));
}

fn load_version_state(repo_path: &Path, scope: &contract::Scope) -> contract::VersionState {
    contract::verify_version(repo_path, scope).unwrap_or_else(|e| {
        eprintln!("  ⚠ 版本状态检查失败: {}", e);
        contract::VersionState {
            tag_version: None, config_version: None, consistent: false, config_files: vec![],
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
    let config_lines: String = info.vs
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
        crate::build::ci::check_ci(info.name, None),
        crate::build::check::check_syntax(info.lang, info.dir),
        version_line,
        config_lines,
        info.c.platform.artifact_registry,
        crate::build::check::check_dependencies(info.dir),
        info.release.changelog,
    )
}
