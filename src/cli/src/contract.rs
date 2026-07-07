/// 契约模块 — 适配层，委托给 `quanttide-devops` toolkit。
pub use quanttide_devops::contract::{
    check_version_consistency, load_or_default, normalize_version, validate_version,
    verify_version, BuildTool, Contract, ContractError, Language, Pipeline, Platform, Registry,
    Scope, SourceControl, Stage, StageBuild, StageRelease, StageTest, VersionState,
};
pub use quanttide_devops::source::config_file::{
    detect_languages, read_config_versions,
};

use std::path::Path;

/// 加载契约。有 `contract.yaml` 则解析，无则自动推测。
pub fn load(repo_path: &Path) -> Contract {
    load_or_default(repo_path)
}

pub fn load_scopes(repo_path: &Path) -> Vec<Scope> {
    load(repo_path).scopes
}

pub fn detect_by_files(dir: &Path) -> Language {
    detect_languages(dir).into_iter().next().unwrap_or(Language::Unknown(String::new()))
}

/// 检查 scope 版本一致性。失败时返回空状态。
pub fn version_status(repo_path: &Path, scope: &Scope) -> VersionState {
    verify_version(repo_path, scope).unwrap_or_else(|e| {
        eprintln!("  ⚠ 版本状态检查失败: {}", e);
        VersionState {
            tag_version: None,
            config_version: None,
            consistent: false,
            config_files: vec![],
        }
    })
}

/// 显示当前契约的完整状态。
pub fn status(repo_path: &Path) {
    let mut stdout = std::io::stdout();
    status_to(&mut stdout, repo_path).ok();
}

/// 写入指定 writer 的版本（可测试）。
pub fn status_to(writer: &mut impl std::io::Write, repo_path: &Path) -> std::io::Result<()> {
    let contract_path = repo_path.join(".quanttide/devops/contract.yaml");
    let exists = contract_path.exists();
    let c = load(repo_path);

    let status = if exists {
        "✅ 已加载"
    } else {
        "⚠ 默认配置"
    };
    let loc = if exists {
        format!("{}", contract_path.display())
    } else {
        "未找到，使用默认契约".into()
    };

    let mut o = format!(
        "契约状态\n{}\n  配置文件:  {}\n  状态:      {}\n\n",
        "-".repeat(40),
        loc,
        status,
    );
    o.push_str(&format!(
        "  Stages:\n    build:    {}\n    test:     {}（阈值 {}%）\n    release:  {}（pre_publish: {:?}）\n\n",
        c.stages.build.command.as_deref().unwrap_or("—"),
        c.stages.test.command.as_deref().unwrap_or("—"),
        c.stages.test.threshold,
        c.stages.release.changelog,
        c.stages.release.pre_publish,
    ));
    o.push_str(&format!(
        "  Platform:\n    source_control:   {:?}\n    pipeline:         {:?}\n    artifact_registry: {}\n\n",
        c.platform.source_control, c.platform.pipeline, c.platform.artifact_registry,
    ));
    o.push_str(&format!(
        "  Sources:\n    version:  {:?} {:?}\n\n",
        c.sources.version.source_type, c.sources.version.path,
    ));
    if c.scopes.is_empty() {
        o.push_str("  Scopes:  0 个\n    未定义 scope\n");
    } else {
        o.push_str(&format!("  Scopes:  {} 个\n", c.scopes.len()));
        for s in &c.scopes {
            o.push_str(&format!(
                "    {:<12} dir: {:<24} {} / {}\n",
                s.name,
                s.dir,
                s.language.as_str(),
                s.build_tool.as_str()
            ));
        }
    }
    let langs = detect_languages(repo_path);
    if !langs.is_empty() {
        o.push_str(&format!(
            "\n  语言:      {}\n",
            langs
                .iter()
                .map(|l| l.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    write!(writer, "{}", o)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmpdir() -> tempfile::TempDir {
        tempfile::tempdir().unwrap()
    }

    #[test]
    fn test_detect_by_files_empty_dir() {
        let d = tmpdir();
        let lang = detect_by_files(d.path());
        assert!(matches!(lang, Language::Unknown(_)));
    }

    #[test]
    fn test_detect_by_files_cargo_toml() {
        let d = tmpdir();
        std::fs::write(d.path().join("Cargo.toml"), "").unwrap();
        let lang = detect_by_files(d.path());
        assert_eq!(lang.as_str(), "rust");
    }

    #[test]
    fn test_detect_by_files_python() {
        let d = tmpdir();
        std::fs::write(d.path().join("pyproject.toml"), "").unwrap();
        let lang = detect_by_files(d.path());
        assert_eq!(lang.as_str(), "python");
    }

    #[test]
    fn test_version_status_no_repo() {
        let d = tmpdir();
        let scope = Scope {
            name: "test".into(),
            dir: ".".into(),
            language: Language::Unknown(String::new()),
            framework: String::new(),
            build_tool: BuildTool::Unknown(String::new()),
            registry: Registry::None,
            release: StageRelease::default(),
            test_threshold: None,
            ci_workflow: None,
        };
        let state = version_status(d.path(), &scope);
        assert!(!state.consistent);
    }

    #[test]
    fn test_load_default_contract() {
        let d = tmpdir();
        let c = load(d.path());
        assert!(c.scopes.is_empty());
    }

    #[test]
    fn test_status_to_empty_dir() {
        let d = tmpdir();
        let mut buf = Vec::new();
        status_to(&mut buf, d.path()).unwrap();
        let output = String::from_utf8_lossy(&buf);
        assert!(output.contains("默认配置"));
        assert!(output.contains("Scopes:"));
    }

    #[test]
    fn test_status_to_with_contract() {
        let d = tmpdir();
        let contract_dir = d.path().join(".quanttide/devops");
        std::fs::create_dir_all(&contract_dir).unwrap();
        std::fs::write(
            contract_dir.join("contract.yaml"),
            "stages:\n  test:\n    threshold: 80\nscopes:\n  cli:\n    dir: src/cli\n",
        )
        .unwrap();
        let mut buf = Vec::new();
        status_to(&mut buf, d.path()).unwrap();
        let output = String::from_utf8_lossy(&buf);
        assert!(output.contains("已加载"));
        assert!(output.contains("cli"));
    }
}
