/// 契约模块 — 基于 `quanttide-devops` toolkit 的适配层。
pub use quanttide_devops::contract::{
    check_version_consistency, normalize_version, validate_version, verify_version, BuildTool,
    Contract, Language, Pipeline, Platform, Registry, Scope, SourceControl, Stage, StageBuild,
    StageRelease, StageTest, VersionState,
};
pub use quanttide_devops::source::config_file::{detect_language, read_config_versions};

use std::path::Path;

// ═══════════════════════════════════════════════════════════════════════
// 加载（保留向后兼容的行为）
// ═══════════════════════════════════════════════════════════════════════

pub fn load(repo_path: &Path) -> Contract {
    match quanttide_devops::contract::load(repo_path) {
        Ok(c) => c,
        Err(_) => auto_detect_contract(repo_path),
    }
}

/// 无 contract.yaml 时自动推测仓库结构生成契约。
fn auto_detect_contract(repo_path: &Path) -> Contract {
    let root_lang = detect_language(repo_path);
    let mut scopes: Vec<Scope> = Vec::new();

    // 扫描常见 scope 子目录
    for base in &["src", "packages", "apps"] {
        let base_dir = repo_path.join(base);
        if !base_dir.is_dir() {
            continue;
        }
        if let Ok(entries) = std::fs::read_dir(&base_dir) {
            for entry in entries.flatten() {
                let sub = entry.path();
                if !sub.is_dir() {
                    continue;
                }
                let name = sub
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default();
                let sub_lang = detect_language(&sub);
                if matches!(sub_lang, Language::Unknown(_)) {
                    continue;
                }
                let dir = format!("{}/{}", base, name);
                scopes.push(Scope {
                    name,
                    dir,
                    language: sub_lang.clone(),
                    framework: String::new(),
                    build_tool: infer_build_tool(&sub_lang),
                    registry: Registry::Crates,
                    release: StageRelease::default(),
                    test_threshold: None,
                    ci_workflow: None,
                });
            }
        }
    }

    // 根目录 scope（优先级最低，find_scope_by_path 以 dir 长度排序）
    if !matches!(root_lang, Language::Unknown(_)) {
        scopes.insert(
            0,
            Scope {
                name: "(root)".into(),
                dir: ".".into(),
                language: root_lang.clone(),
                framework: String::new(),
                build_tool: infer_build_tool(&root_lang),
                registry: Registry::Crates,
                release: StageRelease::default(),
                test_threshold: None,
                ci_workflow: None,
            },
        );
    }

    Contract {
        stages: Stage {
            build: StageBuild {
                command: Some("cargo build".into()),
            },
            test: StageTest {
                command: Some("cargo test".into()),
                ..StageTest::default()
            },
            release: StageRelease {
                changelog: "CHANGELOG.md".into(),
                pre_publish: Vec::new(),
            },
        },
        scopes,
        ..Contract::default()
    }
}

fn infer_build_tool(lang: &Language) -> BuildTool {
    match lang {
        Language::Rust => BuildTool::Cargo,
        Language::Python => BuildTool::Uv,
        Language::Go => BuildTool::Go,
        Language::Dart => BuildTool::Flutter,
        Language::TypeScript => BuildTool::Npm,
        Language::Unknown(_) => BuildTool::Unknown("auto".into()),
    }
}

pub fn load_scopes(repo_path: &Path) -> Vec<Scope> {
    load(repo_path).scopes
}

pub fn detect_by_files(dir: &Path) -> Language {
    detect_language(dir)
}

/// 检测目录下的所有语言（不限于优先级最高的一个）。
pub fn detect_all_languages(dir: &Path) -> Vec<String> {
    let mut langs = Vec::new();
    if dir.join("Cargo.toml").exists() {
        langs.push("rust".into());
    }
    if dir.join("pyproject.toml").exists() || dir.join("requirements.txt").exists() {
        langs.push("python".into());
    }
    if dir.join("go.mod").exists() {
        langs.push("go".into());
    }
    if dir.join("pubspec.yaml").exists() {
        langs.push("dart".into());
    }
    if dir.join("package.json").exists() {
        langs.push("typescript".into());
    }
    langs
}

// ═══════════════════════════════════════════════════════════════════════
// 版本状态（适配 toolkit 的 Result → 旧签名）
// ═══════════════════════════════════════════════════════════════════════

/// 检查 scope 版本一致性。失败时返回空的 VersionStatus。
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

    // 语言汇总
    let mut langs: Vec<String> = Vec::new();
    for s in &c.scopes {
        let scope_dir = repo_path.join(&s.dir);
        let mut scope_langs = detect_all_languages(&scope_dir);
        langs.append(&mut scope_langs);
    }
    langs.sort();
    langs.dedup();
    if !langs.is_empty() {
        writeln!(writer)?;
        writeln!(writer, "  语言:      {}", langs.join(", "))?;
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

    #[test]
    fn test_detect_all_languages_empty() {
        let d = tempfile::tempdir().unwrap();
        let langs = detect_all_languages(d.path());
        assert!(langs.is_empty(), "空目录应返回空");
    }

    #[test]
    fn test_detect_all_languages_multi() {
        let d = tempfile::tempdir().unwrap();
        std::fs::write(d.path().join("Cargo.toml"), "").unwrap();
        std::fs::write(d.path().join("pyproject.toml"), "").unwrap();
        std::fs::write(d.path().join("package.json"), "{}").unwrap();
        let mut langs = detect_all_languages(d.path());
        langs.sort();
        assert_eq!(langs, vec!["python", "rust", "typescript"]);
    }

    #[test]
    fn test_detect_all_languages_edge() {
        let d = tempfile::tempdir().unwrap();
        std::fs::write(d.path().join("go.mod"), "").unwrap();
        std::fs::write(d.path().join("pubspec.yaml"), "").unwrap();
        std::fs::write(d.path().join("requirements.txt"), "").unwrap();
        let mut langs = detect_all_languages(d.path());
        langs.sort();
        assert!(langs.contains(&"go".to_string()));
        assert!(langs.contains(&"dart".to_string()));
        assert!(langs.contains(&"python".to_string()));
    }

    #[test]
    fn test_auto_detect_with_packages() {
        let d = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(d.path().join("packages/foo")).unwrap();
        std::fs::write(d.path().join("packages/foo/Cargo.toml"), "[package]\n").unwrap();
        std::fs::create_dir_all(d.path().join("src/cli")).unwrap();
        std::fs::write(d.path().join("src/cli/Cargo.toml"), "[package]\n").unwrap();
        let c = auto_detect_contract(d.path());
        assert_eq!(c.scopes.len(), 2, "应有 2 个 scope");
        let names: Vec<&str> = c.scopes.iter().map(|s| s.name.as_str()).collect();
        assert!(names.contains(&"foo"));
        assert!(names.contains(&"cli"));
    }

    #[test]
    fn test_auto_detect_empty_repo() {
        let d = tempfile::tempdir().unwrap();
        let c = auto_detect_contract(d.path());
        assert!(c.scopes.is_empty(), "空目录 scopes 应为空");
    }

    #[test]
    fn test_auto_detect_root_only() {
        let d = tempfile::tempdir().unwrap();
        std::fs::write(d.path().join("Cargo.toml"), "[package]\n").unwrap();
        let c = auto_detect_contract(d.path());
        assert!(!c.scopes.is_empty());
        assert!(c.scopes.iter().any(|s| s.name == "(root)"));
    }
}
