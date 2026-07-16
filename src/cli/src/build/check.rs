use std::path::Path;

use crate::contract;

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

/// 构建检查参数（依赖目录，因为 Rust 需要 --manifest-path）。
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

pub(crate) fn check_syntax(lang: &contract::Language, dir: &Path) -> String {
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
