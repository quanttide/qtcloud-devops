/// 契约模块 — 基于 `quanttide-devops` toolkit 的适配层。
pub use quanttide_devops::contract::{
    detect_language_by_files, normalize_version, read_all_config_versions, validate_version,
    BuildTool, Contract, Language, Pipeline, Platform, Registry, Scope, SourceControl, SourceType,
    StageBuild, StageRelease, StageTest, VersionSource,
};
pub use quanttide_devops::source::git::{GitSourceError, VersionStatus};

use std::path::Path;

// ═══════════════════════════════════════════════════════════════════════
// 加载（保留向后兼容的行为）
// ═══════════════════════════════════════════════════════════════════════

pub fn load(repo_path: &Path) -> Contract {
    match quanttide_devops::contract::load(repo_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("  ℹ contract.yaml: {}，使用默认契约", e);
            Contract::default()
        }
    }
}

pub fn load_scopes(repo_path: &Path) -> Vec<Scope> {
    load(repo_path).scopes
}

pub fn detect_by_files(dir: &Path) -> Language {
    detect_language_by_files(dir)
}

// ═══════════════════════════════════════════════════════════════════════
// 版本状态（适配 toolkit 的 Result → 旧签名）
// ═══════════════════════════════════════════════════════════════════════

/// 检查 scope 版本一致性。失败时返回空的 VersionStatus。
pub fn version_status(repo_path: &Path, scope: &Scope) -> VersionStatus {
    quanttide_devops::source::git::version_status(repo_path, scope).unwrap_or_else(|e| {
        eprintln!("  ⚠ 版本状态检查失败: {}", e);
        VersionStatus {
            tag_version: None,
            config_version: None,
            consistent: false,
            config_files: vec![],
        }
    })
}

/// 显示当前契约的完整状态（调试/查看用）。
pub fn status(repo_path: &Path) {
    let mut stdout = std::io::stdout();
    status_to(&mut stdout, repo_path).ok();
}

/// 写入指定 writer 的版本（可测试）。
pub fn status_to(writer: &mut impl std::io::Write, repo_path: &Path) -> std::io::Result<()> {
    let contract_path = repo_path.join(".quanttide/devops/contract.yaml");
    let exists = contract_path.exists();

    let c = load(repo_path);

    writeln!(writer, "契约状态")?;
    writeln!(writer, "{}", "-".repeat(40))?;

    if exists {
        writeln!(writer, "  配置文件:  {}", contract_path.display())?;
        writeln!(writer, "  状态:      ✅ 已加载")?;
    } else {
        writeln!(writer, "  配置文件:  未找到，使用默认契约")?;
        writeln!(writer, "  状态:      ⚠ 默认配置")?;
    }
    writeln!(writer)?;

    // Stages
    writeln!(writer, "  Stages:")?;
    let b = &c.stages.build;
    writeln!(
        writer,
        "    build:    {}",
        b.command.as_deref().unwrap_or("—")
    )?;
    let t = &c.stages.test;
    writeln!(
        writer,
        "    test:     {}（阈值 {}%）",
        t.command.as_deref().unwrap_or("—"),
        t.threshold
    )?;
    let r = &c.stages.release;
    writeln!(
        writer,
        "    release:  {}（pre_publish: {:?}）",
        r.changelog, r.pre_publish
    )?;
    writeln!(writer)?;

    // Platform
    writeln!(writer, "  Platform:")?;
    writeln!(
        writer,
        "    source_control:   {:?}",
        c.platform.source_control
    )?;
    writeln!(writer, "    pipeline:         {:?}", c.platform.pipeline)?;
    writeln!(
        writer,
        "    artifact_registry: {}",
        c.platform.artifact_registry
    )?;
    writeln!(writer)?;

    // Sources
    writeln!(writer, "  Sources:")?;
    writeln!(
        writer,
        "    version:  {:?} {:?}",
        c.sources.version.source_type, c.sources.version.path
    )?;
    writeln!(writer)?;

    // Scopes
    writeln!(writer, "  Scopes:  {} 个", c.scopes.len())?;
    if c.scopes.is_empty() {
        writeln!(writer, "    未定义 scope")?;
    } else {
        for s in &c.scopes {
            writeln!(
                writer,
                "    {:<12} dir: {:<24} {} / {}",
                s.name,
                s.dir,
                s.language.as_str(),
                s.build_tool.as_str()
            )?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_status_git_error_returns_fallback() {
        let scope = Scope {
            name: "test".into(),
            dir: ".".into(),
            language: Language::Rust,
            framework: String::new(),
            build_tool: BuildTool::Cargo,
            registry: Registry::None,
            release: StageRelease::default(),
            test_threshold: None,
            ci_workflow: None,
        };
        let vs = version_status(Path::new("/nonexistent"), &scope);
        assert!(!vs.consistent);
        assert_eq!(vs.tag_version, None);
        assert_eq!(vs.config_version, None);
        assert!(vs.config_files.is_empty());
    }

    #[test]
    fn test_status_to_with_contract() {
        let d = tempfile::tempdir().unwrap();
        // 创建 contract.yaml
        let contract_dir = d.path().join(".quanttide/devops");
        std::fs::create_dir_all(&contract_dir).unwrap();
        std::fs::write(
            contract_dir.join("contract.yaml"),
            "scopes:\n  test:\n    dir: .\n    language: rust\n",
        )
        .unwrap();
        let mut buf = Vec::new();
        status_to(&mut buf, d.path()).unwrap();
        let out = String::from_utf8_lossy(&buf);
        assert!(out.contains("✅ 已加载"));
        assert!(out.contains("test"));
        assert!(out.contains("rust"));
    }

    #[test]
    fn test_status_to_without_contract() {
        let d = tempfile::tempdir().unwrap();
        let mut buf = Vec::new();
        status_to(&mut buf, d.path()).unwrap();
        let out = String::from_utf8_lossy(&buf);
        assert!(out.contains("⚠ 默认配置"));
        assert!(out.contains("未定义 scope"));
    }
}
