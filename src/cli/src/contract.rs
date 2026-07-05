/// 契约模块 — 适配层，委托给 `quanttide-devops` toolkit。
pub use quanttide_devops::contract::{
    check_version_consistency, load_or_default, normalize_version, validate_version,
    verify_version, BuildTool, Contract, ContractError, Language, Pipeline, Platform, Registry,
    Scope, SourceControl, Stage, StageBuild, StageRelease, StageTest, VersionState,
};
pub use quanttide_devops::source::config_file::{
    detect_language, detect_languages, read_config_versions,
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
    detect_language(dir)
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

    writeln!(writer, "  Stages:")?;
    writeln!(
        writer,
        "    build:    {}",
        c.stages.build.command.as_deref().unwrap_or("—")
    )?;
    writeln!(
        writer,
        "    test:     {}（阈值 {}%）",
        c.stages.test.command.as_deref().unwrap_or("—"),
        c.stages.test.threshold
    )?;
    writeln!(
        writer,
        "    release:  {}（pre_publish: {:?}）",
        c.stages.release.changelog, c.stages.release.pre_publish
    )?;
    writeln!(writer)?;

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

    writeln!(writer, "  Sources:")?;
    writeln!(
        writer,
        "    version:  {:?} {:?}",
        c.sources.version.source_type, c.sources.version.path
    )?;
    writeln!(writer)?;

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

    let langs = detect_languages(repo_path);
    if !langs.is_empty() {
        writeln!(writer)?;
        writeln!(
            writer,
            "  语言:      {}",
            langs
                .iter()
                .map(|l| l.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        )?;
    }
    Ok(())
}
