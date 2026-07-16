use std::path::Path;

use crate::contract;

pub(crate) mod status;
pub(crate) mod clean;
pub(crate) mod audit;

pub use status::status;
pub use clean::clean;
pub use audit::audit;

/// CI 运行记录。
#[derive(Debug, PartialEq)]
pub(crate) struct CiRun {
    pub conclusion: String,
    pub title: String,
    pub branch: String,
    pub number: String,
}

pub(crate) struct ScopeInfo<'a> {
    name: &'a str,
    dir: &'a Path,
    lang: &'a contract::Language,
    c: &'a contract::Contract,
    vs: &'a contract::VersionState,
    release: &'a contract::StageRelease,
}

/// 检查工作区是否有未提交变更。
pub(crate) fn is_working_tree_dirty(repo_path: &Path) -> bool {
    std::process::Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(repo_path)
        .output()
        .map(|o| !o.stdout.is_empty())
        .unwrap_or(false)
}

/// 解析 CI workflow 名称。ci_workflow 优先，无则按约定 build-{scope}。
pub(crate) fn resolve_workflow(scope: &str, ci_workflow: Option<&str>) -> String {
    match ci_workflow {
        Some(w) => w.to_string(),
        None => format!("build-{}", scope),
    }
}

/// 返回语言对应的构建检查命令和标签，None 表示不支持。
pub(crate) fn check_command(lang: &contract::Language) -> Option<(&'static str, &'static str)> {
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
pub(crate) fn check_manifest_file(lang: &contract::Language) -> Option<&'static str> {
    match lang {
        contract::Language::Rust => Some("Cargo.toml"),
        contract::Language::Python => Some("pyproject.toml"),
        contract::Language::Go => Some("go.mod"),
        contract::Language::Dart => Some("pubspec.yaml"),
        contract::Language::TypeScript => Some("package.json"),
        contract::Language::Unknown(_) => None,
    }
}

/// 检查 scope 目录下的 Cargo.toml 是否有 path 或 git 依赖。
pub(crate) fn check_dependencies(dir: &Path) -> String {
    let cargo_toml = dir.join("Cargo.toml");
    if !cargo_toml.exists() {
        return "—".into();
    }
    let content = match std::fs::read_to_string(&cargo_toml) {
        Ok(c) => c,
        Err(_) => return "⚠ 无法读取".into(),
    };

    // 检查 [dependencies] 和 [dev-dependencies] 段
    let mut in_deps = false;
    let mut issues: Vec<&str> = Vec::new();
    for line in content.lines() {
        let t = line.trim();
        if t.starts_with('[') {
            in_deps = t == "[dependencies]"
                || t.starts_with("[dependencies.")
                || t == "[dev-dependencies]"
                || t.starts_with("[dev-dependencies.");
            continue;
        }
        if !in_deps || t.starts_with('#') || t.is_empty() {
            continue;
        }
        if t.contains("path = \"") && !t.contains("\"\"") {
            issues.push("path");
        }
        if t.contains("git = \"") && !t.contains("rev = \"") {
            issues.push("git (no rev)");
        }
    }

    if issues.is_empty() {
        "✅ crates.io".into()
    } else {
        format!("⚠ {}", issues.join(", "))
    }
}

#[cfg(test)]
mod tests;
