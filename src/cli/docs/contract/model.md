# 四维架构

契约的四个维度直接映射到 `Contract` 结构体。

## Contract

```rust
pub struct Contract {
    pub stages: Stage,           // 时序 — 构建/测试/发布各阶段命令和阈值
    pub platform: Platform,      // 载体 — 源码托管/CI/制品库
    pub sources: Source,         // 事实源 — 版本号来源
    pub scopes: Vec<Scope>,      // 上下文 — 多组件映射
}
```

## Stage（时序）

```rust
pub struct Stage {
    pub build: StageBuild,     // command: Option<String>
    pub test: StageTest,       // command, threshold (default: 70.0)
    pub release: StageRelease, // changelog (default: "CHANGELOG.md"), pre_publish
}
```

## Platform（载体）

```rust
pub struct Platform {
    pub source_control: SourceControl,  // Github / Gitlab / Gitee
    pub pipeline: Pipeline,             // GithubActions / GitlabCi / Jenkins
    pub artifact_registry: Registry,    // Crates / PyPI / PubDev / Npm / ...
}
```

## Source（事实源）

```rust
pub struct Source {
    pub version: VersionSource,  // source_type: SourceType, path: Option<String>
}
```

`SourceType`：`Cargo`、`Pyproject`、`TagOnly`、`Pubspec`、`PackageJson`、`Auto`（默认）。

## Scope（上下文）

```rust
pub struct Scope {
    pub name: String,
    pub dir: String,
    pub language: Language,
    pub framework: String,
    pub build_tool: BuildTool,
    pub registry: Registry,
    pub release: StageRelease,
    pub test_threshold: Option<f64>,
    pub ci_workflow: Option<String>,
}
```

## 枚举

### Language

```rust
pub enum Language {
    Rust,              // Cargo.toml
    Python,            // pyproject.toml / requirements.txt
    Go,                // go.mod
    Dart,              // pubspec.yaml
    TypeScript,        // package.json
    Unknown(String),   // 兜底
}
```

检测优先级：`Cargo.toml` > `pyproject.toml` > `go.mod` > `pubspec.yaml` > `package.json`。

### BuildTool

```rust
pub enum BuildTool {
    Cargo,             // Rust
    Uv,                // Python（含 uv/poetry/pdm）
    Go,                // Go
    Flutter,           // Dart/Flutter
    Npm,               // TypeScript/Node（含 pnpm/yarn/bun）
    Unknown(String),   // 兜底
}
```

### Registry

```rust
pub enum Registry {
    Crates,            // crates.io
    PyPI,              // Python Package Index
    PubDev,            // pub.dev
    Npm,               // npm registry
    GitHubReleases,    // GitHub Releases
    Docker,            // Docker Hub / 容器镜像
    None,              // 无配置（默认）
}
```

### SourceType

```rust
pub enum SourceType {
    Cargo,             // Cargo.toml version
    Pyproject,         // pyproject.toml version（PEP 621）
    TagOnly,           // 仅 git tag（Go 项目等）
    Pubspec,           // pubspec.yaml version
    PackageJson,       // package.json version
    Auto,              // 自动检测（默认）
}
```
