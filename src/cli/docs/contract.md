# Contract 模块设计文档

契约（Contract）是四维 DevOps 治理模型在代码中的映射，定义在 `src/contract.rs`。

## 加载流程

```
load(repo_path)
  │
  ├─ .quanttide/devops/contract.yaml 存在?
  │   ├─ Y → serde 解析 → Contract
  │   └─ N → auto_detect_contract(repo_path)
  │           ├─ 扫描 src/*/ packages/*/ apps/*/ 下项目配置文件
  │           │   有匹配文件 → 每个子目录生成一个 Scope
  │           │   无匹配     → scope 为空
  │           ├─ 根目录语言可识别? → 插入 (root) scope
  │           └─ 返回 Contract（stages/platform/scopes 均默认值）
  │
  └─ 返回 Contract
```

### 无 contract.yaml 时的自动推断规则

扫描 `src/*`、`packages/*`、`apps/*` 下的一级子目录：

| 标志文件 | 推断语言 | 推断构建工具 |
|----------|---------|------------|
| `Cargo.toml` | `Rust` | `Cargo` |
| `pyproject.toml` / `requirements.txt` | `Python` | `Uv` |
| `go.mod` | `Go` | `Go` |
| `pubspec.yaml` | `Dart` | `Flutter` |
| `package.json` | `TypeScript` | `Npm` |
| 无匹配 | 跳过（不生成 scope） | — |

根目录也按同样规则检测，检测到语言时生成 `(root)` scope（优先级最低）。

## 四维架构

```rust
pub struct Contract {
    pub stages: Stage,           // 时序 — 构建/测试/发布各阶段的命令和阈值
    pub platform: Platform,      // 载体 — 源码托管/CI/制品库
    pub sources: Source,         // 事实源 — 版本号来源
    pub scopes: Vec<Scope>,      // 上下文 — 多组件映射
}
```

### Stage（时序）

```rust
pub struct Stage {
    pub build: StageBuild,       // command: Option<String>
    pub test: StageTest,         // command, threshold (default: 70.0)
    pub release: StageRelease,   // changelog (default: "CHANGELOG.md"), pre_publish
}
```

### Platform（载体）

```rust
pub struct Platform {
    pub source_control: SourceControl,   // Github / Gitlab / Gitee
    pub pipeline: Pipeline,              // GithubActions / GitlabCi / Jenkins
    pub artifact_registry: Registry,     // Crates / PyPI / PubDev / Npm / ...
}
```

### Source（事实源）

```rust
pub struct Source {
    pub version: VersionSource,  // source_type: SourceType, path: Option<String>
}
```

`SourceType` 枚举：`Cargo`、`Pyproject`、`TagOnly`、`Pubspec`、`PackageJson`、`Auto`（默认）。

### Scope（上下文）

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

## 枚举定义

### Language — 编程语言

```rust
pub enum Language {
    Rust,              // Cargo.toml
    Python,            // pyproject.toml / requirements.txt
    Go,                // go.mod
    Dart,              // pubspec.yaml
    TypeScript,        // package.json
    Unknown(String),   // 兜底，携带原始字符串
}
```

检测优先级：`Cargo.toml` > `pyproject.toml` > `go.mod` > `pubspec.yaml` > `package.json`。

### BuildTool — 构建工具

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

### Registry — 制品库

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

### SourceType — 版本号来源

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

## 公共 API

```rust
pub fn load(repo_path: &Path) -> Contract
pub fn load_scopes(repo_path: &Path) -> Vec<Scope>
pub fn status(repo_path: &Path)
pub fn status_to(writer: &mut impl Write, repo_path: &Path) -> io::Result<()>
pub fn version_status(repo_path: &Path, scope: &Scope) -> VersionStatus
pub fn detect_by_files(dir: &Path) -> Language
```

## YAML 格式（contract.yaml）

```yaml
stages:
  build:
    command: cargo build --release
  test:
    command: cargo test
    threshold: 80
  release:
    changelog: CHANGELOG.md
    pre_publish:
      - cargo publish

platform:
  source_control: github
  pipeline: github_actions
  artifact_registry: crates

sources:
  version:
    type: cargo

scopes:
  cli:
    dir: src/cli
    language: rust
    build_tool: cargo
    registry: crates
    test_threshold: 90
    ci_workflow: build-cli
```

所有段均可省略，省略的部分走 `Default` 值。

## 覆盖语义（浅覆盖）

Scope 有值用 scope 的，没有则用全局。不做深度合并。

- `scope.test_threshold = Some(90)` → 覆盖全局 `stages.test.threshold`
- `scope.test_threshold = None` → 使用 `stages.test.threshold`（默认 70.0）
- `scope.release.changelog = "src/cli/CHANGELOG.md"` → 只覆盖 changelog，pre_publish 走全局
- `scope.ci_workflow = None` → 按 `build-{scope}` 约定推导

## 版本一致性检查

```rust
pub fn version_status(repo_path: &Path, scope: &Scope) -> VersionStatus
```

- **tag 版本**：`git tag --sort=-version:refname`，按 scope 前缀过滤
- **配置版本**：按语言读取配置文件中的 `version` 字段
- **一致性**：两者都存在时比较；都为空视为一致；一个为空视为不一致

## 测试策略

269 个测试覆盖：

1. **契约加载**：有 contract.yaml / 无文件 / 自动推断
2. **格式修复**：v 前缀、大小写、非标准版本头/分类、混合格式
3. **清理**：done 条目、空版本、后缀版本 cascade、文件不存在
4. **发布**：scoped 版本、monorepo 子目录、自动更新版本号
5. **版本一致性**：git 异常降级、tag vs 配置比较
6. **CHANGELOG**：自动生成、追加、repo_path ≠ scope_dir
7. **路径解析**：契约映射、回退子目录名、无 scope

## 参考

- 四维架构理论：`docs/essay/contract/index.md`
- Toolkit 模型：`packages/toolkit/packages/rust/src/contract/`
