/// 契约模块。按照四维架构（Stages / Platforms / Sources / Scopes）设计。
///
/// 参考：docs/essay/contract/index.md — 契约化 DevOps 建模
use std::path::Path;

// ═══════════════════════════════════════════════════════════════════════
// 四维架构模型
// ═══════════════════════════════════════════════════════════════════════

/// 完整契约。
#[derive(Debug)]
pub struct Contract {
    /// 生命周期阶段配置。
    pub stages: Stages,
    /// 外部治理载体配置。
    pub platforms: Platforms,
    /// 事实源配置。
    pub sources: Sources,
    /// 作用域列表。
    pub scopes: Vec<Scope>,
}

/// Stages（时序维度）：定义价值流的节拍。
///
/// 不规定"怎么做"，只规定"什么时候检查什么"。
#[derive(Debug, Clone)]
pub struct Stages {
    /// 构建阶段配置。
    pub build: StageBuild,
    /// 测试阶段配置。
    pub test: StageTest,
    /// 发布阶段配置。
    pub release: StageRelease,
}

impl Default for Stages {
    fn default() -> Self {
        Self {
            build: StageBuild { command: None },
            test: StageTest {
                command: None,
                threshold: 70.0,
            },
            release: StageRelease {
                changelog: "CHANGELOG.md".into(),
                pre_publish: Vec::new(),
            },
        }
    }
}

#[derive(Debug, Clone)]
pub struct StageBuild {
    pub command: Option<String>,
}

#[derive(Debug, Clone)]
pub struct StageTest {
    pub command: Option<String>,
    pub threshold: f64,
}

#[derive(Debug, Clone)]
pub struct StageRelease {
    pub changelog: String,
    pub pre_publish: Vec<String>,
}

/// Platforms（载体维度）：定义能力的空间。
///
/// 指 GitHub、Kubernetes、Artifactory 等外部治理载体。负责"外部合规"。
///
/// 当前默认值锁定为 `github + github_actions`。
/// 如需支持 GitLab、自建 CI 等，将默认值改为可配置即可。
#[derive(Debug, Clone)]
pub struct Platforms {
    /// 源代码管理平台。
    pub source_control: String,
    /// CI/CD 平台。
    pub ci: String,
    /// 制品库。
    pub artifact_registry: Registry,
}

impl Default for Platforms {
    fn default() -> Self {
        Self {
            source_control: "github".into(),
            ci: "github_actions".into(),
            artifact_registry: Registry::None,
        }
    }
}

/// Sources（事实源维度）：定义真相的本质。
///
/// 指 Git（代码源）、配置文件（版本源）等核心内容引擎。负责"内在完整"。
#[derive(Debug, Clone)]
pub struct Sources {
    /// 版本号来源。
    pub version: VersionSource,
}

impl Default for Sources {
    fn default() -> Self {
        Self {
            version: VersionSource {
                source_type: SourceType::Auto,
                path: None,
            },
        }
    }
}

#[derive(Debug, Clone)]
pub struct VersionSource {
    pub source_type: SourceType,
    pub path: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SourceType {
    /// cargo（Cargo.toml）
    Cargo,
    /// pyproject.toml（PEP 621 / Poetry / PDM）
    Pyproject,
    /// 不从配置文件读版本，只从 git tag 读
    TagOnly,
    /// pubspec.yaml（Dart/Flutter）
    Pubspec,
    /// Node/TypeScript（package.json）
    PackageJson,
    /// 自动检测
    Auto,
}

/// Scopes（上下文维度）：定义规则的边界。
///
/// 通过 scope 为不同组件挂载不同的 Stages、Platforms、Sources 组合。
#[derive(Debug, Clone)]
pub struct Scope {
    pub name: String,
    pub dir: String,
    /// 语言与框架信息（属于 Sources 维度，但按 scope 声明）。
    pub language: Language,
    pub framework: String,
    pub build_tool: BuildTool,
    /// 该 scope 的制品库（覆盖全局 Platforms）。
    pub registry: Registry,
    /// 该 scope 的发布配置（覆盖全局 Stages.release）。
    pub release: StageRelease,
    /// 该 scope 的测试阈值（覆盖全局 Stages.test.threshold）。
    pub test_threshold: Option<f64>,
    /// CI workflow 名称。未设置时按 build-{scope} 约定推导。
    pub ci_workflow: Option<String>,
}

// ── 辅枚举 ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum Language {
    Rust,
    Python,
    Go,
    Dart,
    TypeScript,
    Unknown(String),
}

impl Language {
    pub fn is_supported(&self) -> bool {
        !matches!(self, Language::Unknown(_))
    }

    pub fn name(&self) -> &str {
        match self {
            Language::Rust => "Rust",
            Language::Python => "Python",
            Language::Go => "Go",
            Language::Dart => "Dart",
            Language::TypeScript => "TypeScript",
            Language::Unknown(s) => s,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum BuildTool {
    Cargo,
    Uv,
    Go,
    Flutter,
    Npm,
    Unknown(String),
}

impl BuildTool {
    pub fn is_supported(&self) -> bool {
        !matches!(self, BuildTool::Unknown(_))
    }

    pub fn name(&self) -> &str {
        match self {
            BuildTool::Cargo => "cargo",
            BuildTool::Uv => "uv",
            BuildTool::Go => "go build",
            BuildTool::Flutter => "flutter build",
            BuildTool::Npm => "npm",
            BuildTool::Unknown(s) => s,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Registry {
    Crates,
    PyPI,
    PubDev,
    Npm,
    GitHubReleases,
    Docker,
    None,
}

impl Registry {
    pub fn name(&self) -> &str {
        match self {
            Registry::Crates => "crates.io",
            Registry::PyPI => "PyPI",
            Registry::PubDev => "pub.dev",
            Registry::Npm => "npm",
            Registry::GitHubReleases => "GitHub Releases",
            Registry::Docker => "Docker",
            Registry::None => "无",
        }
    }
}

/// 版本一致性状态。
#[derive(Debug)]
pub struct VersionStatus {
    pub tag_version: Option<String>,
    pub config_version: Option<String>,
    pub consistent: bool,
}

// ═══════════════════════════════════════════════════════════════════════
// 加载与解析
// ═══════════════════════════════════════════════════════════════════════

/// 从 `.quanttide/devops/contract.yaml` 加载完整契约。
pub fn load(repo_path: &Path) -> Contract {
    let path = repo_path.join(".quanttide/devops/contract.yaml");
    let content = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(_) => {
            eprintln!("  ℹ contract.yaml 不存在，使用默认契约");
            return default_contract();
        }
    };
    parse(&content)
}

fn parse(content: &str) -> Contract {
    // 尝试新格式（四维架构）
    if let Ok(parsed) = serde_yaml::from_str::<ContractYaml>(content) {
        return parsed.into_contract();
    }
    if serde_yaml::from_str::<serde_yaml::Value>(content).is_ok() {
        eprintln!("⚠ contract.yaml: 无法按新格式解析，使用默认值");
    }
    default_contract()
}

fn default_contract() -> Contract {
    Contract {
        stages: Stages::default(),
        platforms: Platforms::default(),
        sources: Sources::default(),
        scopes: Vec::new(),
    }
}

// ═══════════════════════════════════════════════════════════════════════
// 便捷访问
// ═══════════════════════════════════════════════════════════════════════

/// 获取 scope 的发布配置（scope 级覆盖 → 全局默认）。
pub fn scope_release<'a>(contract: &'a Contract, scope: &'a Scope) -> &'a StageRelease {
    let has_custom =
        !scope.release.pre_publish.is_empty() || scope.release.changelog != "CHANGELOG.md";
    if has_custom {
        &scope.release
    } else {
        &contract.stages.release
    }
}

/// 获取 scope 的测试阈值。
pub fn scope_test_threshold(contract: &Contract, scope: &Scope) -> f64 {
    scope
        .test_threshold
        .unwrap_or(contract.stages.test.threshold)
}

// ═══════════════════════════════════════════════════════════════════════
// 语言检测
// ═══════════════════════════════════════════════════════════════════════

pub fn resolve_language(scope: &Scope, scope_dir: &Path) -> Language {
    match &scope.language {
        Language::Unknown(_) => detect_by_files(scope_dir),
        lang => lang.clone(),
    }
}

pub fn detect_by_files(dir: &Path) -> Language {
    if dir.join("Cargo.toml").exists() {
        Language::Rust
    } else if dir.join("pyproject.toml").exists() || dir.join("requirements.txt").exists() {
        Language::Python
    } else if dir.join("go.mod").exists() {
        Language::Go
    } else if dir.join("pubspec.yaml").exists() {
        Language::Dart
    } else if dir.join("package.json").exists() {
        Language::TypeScript
    } else {
        Language::Unknown("无法识别".into())
    }
}

// ═══════════════════════════════════════════════════════════════════════
// 版本状态
// ═══════════════════════════════════════════════════════════════════════

pub fn version_status(repo_path: &Path, scope: &Scope) -> VersionStatus {
    let tag_version = latest_tag_for_scope(repo_path, &scope.name);
    let scope_dir = repo_path.join(&scope.dir);
    let config_version = read_config_version(&scope_dir, &scope.language);
    let consistent = match (&tag_version, &config_version) {
        (Some(t), Some(c)) => t == c,
        (None, None) => true,
        _ => false,
    };
    VersionStatus {
        tag_version,
        config_version,
        consistent,
    }
}

fn latest_tag_for_scope(repo_path: &Path, scope_name: &str) -> Option<String> {
    let output = std::process::Command::new("git")
        .args(["tag", "--sort=-version:refname"])
        .current_dir(repo_path)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let prefix = format!("{}/", scope_name);
    let tags: Vec<&str> = std::str::from_utf8(&output.stdout)
        .ok()?
        .lines()
        .filter(|t| t.starts_with(&prefix) || !t.contains('/'))
        .collect();
    let scoped = tags.iter().find(|t| t.starts_with(&prefix));
    match scoped {
        Some(t) => Some(normalize_version(t)),
        None => tags.first().map(|t| normalize_version(t)),
    }
}

fn normalize_version(version: &str) -> String {
    let after_scope = version.split('/').last().unwrap_or(version);
    after_scope
        .strip_prefix('v')
        .unwrap_or(after_scope)
        .to_string()
}

fn read_config_version(dir: &Path, lang: &Language) -> Option<String> {
    let filename = match lang {
        Language::Rust => "Cargo.toml",
        Language::Python => "pyproject.toml",
        Language::TypeScript => "package.json",
        Language::Dart => "pubspec.yaml",
        _ => return None,
    };
    let path = dir.join(filename);
    let content = std::fs::read_to_string(path).ok()?;
    for line in content.lines() {
        let t = line.trim();
        if t.starts_with("version = \"") {
            if let Some(v) = t.strip_prefix("version = \"") {
                if let Some(end) = v.find('"') {
                    return Some(v[..end].to_string());
                }
            }
        }
        if t.starts_with("\"version\":") {
            if let Some(rest) = t.strip_prefix("\"version\":") {
                let v = rest.trim().trim_matches(',').trim_matches('"');
                if !v.is_empty() {
                    return Some(v.to_string());
                }
            }
        }
    }
    None
}

// ═══════════════════════════════════════════════════════════════════════
// YAML 数据结构（私有）
// ═══════════════════════════════════════════════════════════════════════

#[derive(Debug, serde::Deserialize)]
struct ContractYaml {
    #[serde(default)]
    stages: Option<StagesYaml>,
    #[serde(default)]
    platforms: Option<PlatformsYaml>,
    #[serde(default)]
    sources: Option<SourcesYaml>,
    #[serde(default)]
    scopes: Option<std::collections::BTreeMap<String, ScopeYaml>>,
}

#[derive(Debug, serde::Deserialize)]
struct StagesYaml {
    #[serde(default)]
    build: Option<BuildYaml>,
    #[serde(default)]
    test: Option<TestYaml>,
    #[serde(default)]
    release: Option<ReleaseYaml>,
}

#[derive(Debug, serde::Deserialize)]
struct BuildYaml {
    command: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
struct TestYaml {
    command: Option<String>,
    #[serde(default)]
    threshold: Option<f64>,
}

#[derive(Debug, serde::Deserialize)]
struct ReleaseYaml {
    #[serde(default)]
    changelog: Option<String>,
    #[serde(default)]
    pre_publish: Option<Vec<String>>,
}

#[derive(Debug, serde::Deserialize)]
struct PlatformsYaml {
    #[serde(default)]
    source_control: Option<String>,
    #[serde(default)]
    ci: Option<String>,
    #[serde(default)]
    artifact_registry: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
struct SourcesYaml {
    #[serde(default)]
    version: Option<VersionSourceYaml>,
}

#[derive(Debug, serde::Deserialize)]
struct VersionSourceYaml {
    #[serde(rename = "type")]
    source_type: Option<String>,
    path: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
struct ScopeYaml {
    dir: String,
    #[serde(default)]
    language: Option<String>,
    #[serde(default)]
    framework: Option<String>,
    #[serde(default)]
    build_tool: Option<String>,
    #[serde(default)]
    registry: Option<String>,
    #[serde(default)]
    release: Option<ReleaseYaml>,
    #[serde(default)]
    test_threshold: Option<f64>,
    #[serde(default)]
    ci_workflow: Option<String>,
}

impl ContractYaml {
    fn into_contract(self) -> Contract {
        let stages = self
            .stages
            .map(|s| Stages {
                build: StageBuild {
                    command: s.build.and_then(|b| b.command),
                },
                test: StageTest {
                    command: s.test.as_ref().and_then(|t| t.command.clone()),
                    threshold: s.test.as_ref().and_then(|t| t.threshold).unwrap_or(70.0),
                },
                release: s
                    .release
                    .map(|r| StageRelease {
                        changelog: r.changelog.unwrap_or_else(|| "CHANGELOG.md".into()),
                        pre_publish: r.pre_publish.unwrap_or_default(),
                    })
                    .unwrap_or_default(),
            })
            .unwrap_or_default();

        let platforms = self
            .platforms
            .map(|p| Platforms {
                source_control: p.source_control.unwrap_or_else(|| "github".into()),
                ci: p.ci.unwrap_or_else(|| "github_actions".into()),
                artifact_registry: parse_registry(p.artifact_registry.as_deref()),
            })
            .unwrap_or_default();

        let sources = self
            .sources
            .map(|s| Sources {
                version: s
                    .version
                    .map(|v| VersionSource {
                        source_type: parse_source_type(v.source_type.as_deref()),
                        path: v.path,
                    })
                    .unwrap_or_default(),
            })
            .unwrap_or_default();

        let scopes = self
            .scopes
            .unwrap_or_default()
            .into_iter()
            .map(|(name, cfg)| {
                let lang = match cfg.language.as_deref() {
                    Some("rust") => Language::Rust,
                    Some("python") => Language::Python,
                    Some("go") => Language::Go,
                    Some("dart") => Language::Dart,
                    Some("typescript") | Some("ts") | Some("node") => Language::TypeScript,
                    Some(other) => Language::Unknown(other.into()),
                    None => Language::Unknown("auto".into()),
                };
                let build_tool = match cfg.build_tool.as_deref() {
                    Some("cargo") => BuildTool::Cargo,
                    Some("uv") => BuildTool::Uv,
                    Some("go") => BuildTool::Go,
                    Some("flutter") => BuildTool::Flutter,
                    Some("npm") => BuildTool::Npm,
                    Some(other) => BuildTool::Unknown(other.into()),
                    None => BuildTool::Unknown("auto".into()),
                };
                let release = cfg
                    .release
                    .map(|r| StageRelease {
                        changelog: r.changelog.unwrap_or_else(|| "CHANGELOG.md".into()),
                        pre_publish: r.pre_publish.unwrap_or_default(),
                    })
                    .unwrap_or_default();
                Scope {
                    name,
                    dir: cfg.dir,
                    language: lang,
                    framework: cfg.framework.unwrap_or_default(),
                    build_tool,
                    registry: parse_registry(cfg.registry.as_deref()),
                    release,
                    test_threshold: cfg.test_threshold,
                    ci_workflow: cfg.ci_workflow.clone(),
                }
            })
            .collect();

        Contract {
            stages,
            platforms,
            sources,
            scopes,
        }
    }
}

fn parse_registry(s: Option<&str>) -> Registry {
    match s {
        Some("crates") => Registry::Crates,
        Some("pypi") => Registry::PyPI,
        Some("pubdev") => Registry::PubDev,
        Some("npm") => Registry::Npm,
        Some("github") | Some("github_releases") => Registry::GitHubReleases,
        Some("docker") => Registry::Docker,
        _ => Registry::None,
    }
}

fn parse_source_type(s: Option<&str>) -> SourceType {
    match s {
        Some("cargo") => SourceType::Cargo,
        Some("pyproject") => SourceType::Pyproject,
        Some("tag") => SourceType::TagOnly,
        Some("pubspec") => SourceType::Pubspec,
        Some("package.json") | Some("node") | Some("typescript") => SourceType::PackageJson,
        _ => SourceType::Auto,
    }
}

impl Default for StageRelease {
    fn default() -> Self {
        Self {
            changelog: "CHANGELOG.md".into(),
            pre_publish: Vec::new(),
        }
    }
}

impl Default for VersionSource {
    fn default() -> Self {
        Self {
            source_type: SourceType::Auto,
            path: None,
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════
// 向下兼容 API
// ═══════════════════════════════════════════════════════════════════════

/// 快速加载 scope 列表（简化版，兼容旧调用方）。
pub fn load_scopes(repo_path: &Path) -> Vec<Scope> {
    load(repo_path).scopes
}

/// 快速检测语言（简化版，兼容旧调用方）。
pub fn detect_language(dir: &Path) -> Language {
    detect_by_files(dir)
}

/// 根据当前工作目录查找匹配的 scope。
///
/// 按 dir 路径前缀最长匹配。例如当前在 `src/cli/sub` 时，
/// `cli` scope（dir: `src/cli`）比 root scope（dir: `.`）优先级高。
pub fn find_scope_by_path<'a>(scopes: &'a [Scope], current_dir: &Path) -> Option<&'a Scope> {
    let current_str = current_dir.to_string_lossy();
    scopes
        .iter()
        .filter(|s| current_str.starts_with(&s.dir) || s.dir == ".")
        .max_by_key(|s| s.dir.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── 新格式：四维架构 ──────────────────────────────────────────────

    #[test]
    fn test_load_new_format_full() {
        let d = tempfile::tempdir().unwrap();
        let dir = d.path().join(".quanttide/devops");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("contract.yaml"),
            r#"
stages:
  build:
    command: cargo build --release
  test:
    command: cargo test
    threshold: 80
  release:
    changelog: CHANGELOG.md
    pre_publish:
      - scripts/preflight.sh

platforms:
  source_control: github
  ci: github_actions
  artifact_registry: crates

sources:
  version:
    type: cargo
    path: Cargo.toml

scopes:
  cli:
    dir: src/cli
    language: rust
    framework: clap
    build_tool: cargo
    registry: crates
  studio:
    dir: src/studio
    language: dart
    framework: flutter
    build_tool: flutter
    registry: pubdev
    release:
      changelog: src/studio/CHANGELOG.md
"#,
        )
        .unwrap();

        let c = load(d.path());

        // Stages
        assert_eq!(
            c.stages.build.command.as_deref(),
            Some("cargo build --release")
        );
        assert_eq!(c.stages.test.threshold, 80.0);
        assert_eq!(c.stages.release.changelog, "CHANGELOG.md");
        assert_eq!(c.stages.release.pre_publish.len(), 1);

        // Platforms
        assert_eq!(c.platforms.source_control, "github");
        assert_eq!(c.platforms.artifact_registry, Registry::Crates);

        // Sources
        assert_eq!(c.sources.version.source_type, SourceType::Cargo);

        // Scopes
        assert_eq!(c.scopes.len(), 2);
        assert_eq!(c.scopes[0].name, "cli");
        assert_eq!(c.scopes[0].language, Language::Rust);
        assert_eq!(c.scopes[0].registry, Registry::Crates);
        assert_eq!(c.scopes[1].name, "studio");
        assert_eq!(c.scopes[1].language, Language::Dart);
        assert_eq!(c.scopes[1].release.changelog, "src/studio/CHANGELOG.md");
    }

    #[test]
    fn test_load_new_format_minimal() {
        let d = tempfile::tempdir().unwrap();
        let dir = d.path().join(".quanttide/devops");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("contract.yaml"),
            "scopes:\n  cli:\n    dir: src/cli\n",
        )
        .unwrap();

        let c = load(d.path());
        assert_eq!(c.scopes.len(), 1);
        assert_eq!(c.scopes[0].name, "cli");
        // 未声明的用默认值
        assert_eq!(c.stages.test.threshold, 70.0);
        assert_eq!(c.platforms.source_control, "github");
    }

    #[test]
    fn test_load_no_file() {
        let d = tempfile::tempdir().unwrap();
        let c = load(d.path());
        assert!(c.scopes.is_empty());
    }

    // ── 便捷函数 ────────────────────────────────────────────────────

    #[test]
    fn test_resolve_language_declared() {
        let s = Scope {
            name: "cli".into(),
            dir: ".".into(),
            language: Language::Rust,
            framework: String::new(),
            build_tool: BuildTool::Cargo,
            registry: Registry::Crates,
            release: StageRelease::default(),
            test_threshold: None,
            ci_workflow: None,
        };
        assert_eq!(resolve_language(&s, Path::new("/tmp")), Language::Rust);
    }

    #[test]
    fn test_scope_test_threshold_custom() {
        let mut c = default_contract();
        c.stages.test.threshold = 70.0;
        let s = Scope {
            name: "cli".into(),
            dir: ".".into(),
            language: Language::Rust,
            framework: String::new(),
            build_tool: BuildTool::Cargo,
            registry: Registry::Crates,
            release: StageRelease::default(),
            test_threshold: Some(90.0),
            ci_workflow: None,
        };
        assert_eq!(scope_test_threshold(&c, &s), 90.0);
    }

    #[test]
    fn test_scope_test_threshold_global() {
        let mut c = default_contract();
        c.stages.test.threshold = 70.0;
        let s = Scope {
            name: "cli".into(),
            dir: ".".into(),
            language: Language::Rust,
            framework: String::new(),
            build_tool: BuildTool::Cargo,
            registry: Registry::Crates,
            release: StageRelease::default(),
            test_threshold: None,
            ci_workflow: None,
        };
        assert_eq!(scope_test_threshold(&c, &s), 70.0);
    }

    // ── 语言检测 ────────────────────────────────────────────────────

    #[test]
    fn test_detect_by_files_rust() {
        let d = tempfile::tempdir().unwrap();
        std::fs::write(d.path().join("Cargo.toml"), "").unwrap();
        assert_eq!(detect_by_files(d.path()), Language::Rust);
    }

    #[test]
    fn test_detect_by_files_unknown() {
        let d = tempfile::tempdir().unwrap();
        assert!(matches!(detect_by_files(d.path()), Language::Unknown(_)));
    }

    // ── 版本号 ──────────────────────────────────────────────────────

    #[test]
    fn test_normalize_version_v_prefix() {
        assert_eq!(normalize_version("v1.2.3"), "1.2.3");
    }

    #[test]
    fn test_normalize_version_scoped() {
        assert_eq!(normalize_version("cli/v0.1.0"), "0.1.0");
    }

    #[test]
    fn test_read_config_version_cargo() {
        let d = tempfile::tempdir().unwrap();
        std::fs::write(
            d.path().join("Cargo.toml"),
            "[package]\nname = \"foo\"\nversion = \"0.1.0\"\n",
        )
        .unwrap();
        let v = read_config_version(d.path(), &Language::Rust);
        assert_eq!(v.as_deref(), Some("0.1.0"));
    }

    // ── 边缘测试 ────────────────────────────────────────────────────

    #[test]
    fn test_unknown_language_in_yaml() {
        let content =
            "stages:\n  test:\n    threshold: 70\nscopes:\n  ziggy:\n    dir: src/ziggy\n    language: zig\n";
        let c = parse(content);
        assert_eq!(c.scopes.len(), 1);
        assert_eq!(c.scopes[0].language, Language::Unknown("zig".into()));
    }

    #[test]
    fn test_normalize_version_rc() {
        assert_eq!(normalize_version("v1.0.0-rc.1"), "1.0.0-rc.1");
    }

    #[test]
    fn test_normalize_version_strips_v_only() {
        assert_eq!(normalize_version("v0.0.1"), "0.0.1");
        assert_eq!(normalize_version("0.0.1"), "0.0.1");
    }

    #[test]
    fn test_normalize_version_scoped_with_rc() {
        assert_eq!(normalize_version("cli/v1.0.0-rc.1"), "1.0.0-rc.1");
    }

    #[test]
    fn test_find_scope_by_path_exact_match() {
        let scopes = vec![
            Scope {
                name: "root".into(),
                dir: ".".into(),
                language: Language::Unknown("auto".into()),
                ..scope_default()
            },
            Scope {
                name: "cli".into(),
                dir: "src/cli".into(),
                language: Language::Rust,
                ..scope_default()
            },
        ];
        let found = find_scope_by_path(&scopes, Path::new("src/cli"));
        assert_eq!(found.map(|s| s.name.as_str()), Some("cli"));
    }

    #[test]
    fn test_find_scope_by_path_subdir() {
        let scopes = vec![
            Scope {
                name: "root".into(),
                dir: ".".into(),
                language: Language::Unknown("auto".into()),
                ..scope_default()
            },
            Scope {
                name: "cli".into(),
                dir: "src/cli".into(),
                language: Language::Rust,
                ..scope_default()
            },
        ];
        let found = find_scope_by_path(&scopes, Path::new("src/cli/sub/foo"));
        assert_eq!(found.map(|s| s.name.as_str()), Some("cli"));
    }

    #[test]
    fn test_find_scope_by_path_root_fallback() {
        let scopes = vec![
            Scope {
                name: "root".into(),
                dir: ".".into(),
                language: Language::Unknown("auto".into()),
                ..scope_default()
            },
            Scope {
                name: "cli".into(),
                dir: "src/cli".into(),
                language: Language::Rust,
                ..scope_default()
            },
        ];
        let found = find_scope_by_path(&scopes, Path::new("docs"));
        assert_eq!(found.map(|s| s.name.as_str()), Some("root"));
    }

    #[test]
    fn test_find_scope_by_path_no_match() {
        let scopes = vec![];
        let found = find_scope_by_path(&scopes, Path::new("src/cli"));
        assert!(found.is_none());
    }

    fn scope_default() -> Scope {
        Scope {
            name: String::new(),
            dir: ".".into(),
            language: Language::Unknown("auto".into()),
            framework: String::new(),
            build_tool: BuildTool::Unknown("auto".into()),
            registry: Registry::None,
            release: StageRelease::default(),
            test_threshold: None,
            ci_workflow: None,
        }
    }
}
