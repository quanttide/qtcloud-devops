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
