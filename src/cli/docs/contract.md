# Contract 模块设计文档

> 枚举命名约定：按**文件/工具/行为**命名，不按语言或运行时。
> 例外：`BuildTool::Uv` 保留原名，因 `uv` 是其官方项目名，且用户群体已认知。

## 枚举定义

### `Language` — 编程语言

```rust
pub enum Language {
    Rust,              // Cargo.toml 存在
    Python,            // pyproject.toml 或 requirements.txt 存在
    Go,                // go.mod 存在
    Dart,              // pubspec.yaml 存在
    TypeScript,        // package.json 存在
    Unknown(String),   // 无法识别，携带原始标识符以便调试
}
```

检测策略（`detect_by_files`）：按优先级检查根目录下的标志文件，`Cargo.toml` > `pyproject.toml` > `go.mod` > `pubspec.yaml` > `package.json`。无匹配时返回 `Unknown`。

### `BuildTool` — 构建工具

```rust
pub enum BuildTool {
    Cargo,             // Rust 项目
    Uv,                // Python 项目（uv，兼容 pip CLI）
    Go,                // Go 项目（go build）
    Flutter,           // Dart/Flutter 项目（flutter build）
    Npm,               // Node/TypeScript 项目（npm run build）
    Unknown(String),   // 无法识别
}
```

### `Registry` — 制品库

```rust
pub enum Registry {
    Crates,            // crates.io
    PyPI,              // Python Package Index
    PubDev,            // pub.dev（Dart/Flutter）
    Npm,               // npm registry
    GitHubReleases,    // GitHub Releases（不通过包管理器分发时使用）
    Docker,            // Docker Hub / 容器镜像仓库
    None,              // 无制品库 / 暂未配置
}
```

### `SourceType` — 版本号来源

```rust
pub enum SourceType {
    Cargo,             // 从 Cargo.toml 的 version 字段读取
    Pyproject,         // 从 pyproject.toml 的 version 字段读取（PEP 621）
    TagOnly,           // 不从配置文件读版本，只从 git tag 读取（如 Go 项目）
    Pubspec,           // 从 pubspec.yaml 的 version 字段读取（Dart/Flutter）
    PackageJson,       // 从 package.json 的 version 字段读取
    Auto,              // 自动检测（默认值）
}
```

所有枚举遵循**兜底原则**：`Unknown` / `None` 变体保证任何未知输入不会导致崩溃。

## 现状

`src/contract.rs` 已实现，20 个测试全部通过。四维契约模型已落地，替换了原来散落在 `publish.rs` 和 `status.rs` 中的手写 YAML 解析。

| 文件 | 旧实现（已删除） | 替换方式 |
|------|-----------------|---------|
| `release/publish.rs` | `resolve_scope_dir()` — 35 行按行扫描 YAML | `contract::load_scopes()` 查找 version 前缀对应 scope |
| `release/status.rs` | `read_contract_scopes()` — 33 行手写状态机 | `contract::load_scopes()` → HashMap 转换 |

**收益**：消灭 68 行重复 YAML 解析，消除 `unused variable` 警告，四维架构可供后续命令直接复用。

## 核心结构

```rust
pub struct Contract {
    pub stages: Stages,         // 时序维度
    pub platforms: Platforms,   // 载体维度
    pub sources: Sources,       // 事实源维度
    pub scopes: Vec<Scope>,     // 上下文维度
}

pub struct Stages {
    pub build: StageBuild,
    pub test: StageTest,        // threshold: f64
    pub release: StageRelease,  // changelog, pre_publish
}

pub struct Platforms {
    pub source_control: String,        // 默认 "github"
    pub ci: String,                    // 默认 "github_actions"
    pub artifact_registry: Registry,   // Crates/PyPI/...
}

pub struct Sources {
    pub version: VersionSource,  // source_type: SourceType, path: Option<String>
}

pub struct Scope {
    pub name: String,
    pub dir: String,
    pub language: Language,
    pub framework: String,
    pub build_tool: BuildTool,
    pub registry: Registry,
    pub release: StageRelease,
    pub test_threshold: Option<f64>,
}
```

## 公共 API

```rust
// 加载
pub fn load(repo_path: &Path) -> Contract

// 便捷访问
pub fn scope_release(contract, scope) -> &StageRelease
pub fn scope_test_threshold(contract, scope) -> f64
pub fn find_scope_by_path(scopes, current_dir) -> Option<&Scope>

// 语言检测
pub fn resolve_language(scope, scope_dir) -> Language
pub fn detect_by_files(dir) -> Language
pub trait Language::is_supported() -> bool
pub trait BuildTool::is_supported() -> bool

// 版本状态
pub fn version_status(repo_path, scope) -> VersionStatus

// 向下兼容
pub fn load_scopes(repo_path) -> Vec<Scope>
pub fn detect_language(dir) -> Language
```

## YAML 格式

新格式（完整四维架构）：

```yaml
stages:
  test:
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
    build_tool: cargo
    registry: crates
  studio:
    dir: src/studio
    language: dart
    build_tool: flutter
    registry: pubdev
    release:
      changelog: src/studio/CHANGELOG.md
```

旧格式兼容：

```yaml
scopes:
  cli: src/cli
  studio: src/studio
```

## 两阶段解析

```
YAML 文件
   │
   ├─ 尝试 serde::from_str<ContractYaml>（新格式）
   │    成功 → into_contract() → Contract
   │    失败 → ─┐
   │            ├─ YAML 语法合法 → eprintln 警告"无法按新格式解析"
   │            │
   ▼            ▼
   ├─ 尝试 serde::from_str<OldContractYaml>（旧格式）
   │    成功 → into_contract() → Contract（自动语言检测）
   │    失败 → eprintln 警告"建议迁移到新格式"
   │
   └─ 都失败 → default_contract()
               eprintln 警告"使用默认值"
```

### 错误反馈

每条解析路径都有 `eprintln` 输出：

| 场景 | 输出 |
|------|------|
| contract.yaml 不存在 | `ℹ contract.yaml 不存在，使用默认契约` |
| YAML 语法合法但新格式不匹配 | `⚠ contract.yaml: 无法按新格式解析，尝试旧格式...` |
| 旧格式匹配成功 | `⚠ contract.yaml: 检测到旧格式，建议迁移到新格式` |
| 全部失败 | `⚠ contract.yaml: 无法按任何格式解析，使用默认值` |

## 版本一致性检查

```rust
pub fn version_status(repo_path: &Path, scope: &Scope) -> VersionStatus
```

- **tag 版本**：`git tag --sort=-version:refname`，按 scope 前缀过滤，`normalize_version` 去 `v` 前缀
- **配置版本**：按语言读取配置文件中的 `version` 字段
- **一致性**：两者都存在时比较；都为空视为一致；一个为空视为不一致

## 覆盖语义（浅覆盖）

Scope 级有值就用 scope 的，没有就用全局的。不是深度合并。

- `scope.test_threshold = Some(90)` → 覆盖全局 `stages.test.threshold`
- `scope.test_threshold = None` → 使用 `stages.test.threshold`（默认 70.0）
- `scope.release.changelog = "src/cli/CHANGELOG.md"` → 只覆盖 changelog，`pre_publish` 走全局

## 依赖

```toml
[dependencies]
serde = { version = "1", features = ["derive"] }   # 已有
serde_yaml = "0.9"                                   # 新建 contract 时新增
```

## 测试策略

共 20 个测试，覆盖：

1. **新格式解析**：完整四维架构 YAML、最小 YAML
2. **旧格式兼容**：`scopes: { cli: src/cli }`
3. **文件缺失**：无 contract.yaml 时返回默认值
4. **便捷函数**：`scope_test_threshold` 自定义/全局、`resolve_language` 声明优先
5. **语言检测**：Rust（有 Cargo.toml）、Unknown（空目录）
6. **版本号**：`v` 前缀、scope 前缀、RC 版本、scope+RC 混合
7. **边缘测试**：Unknown 语言 YAML、路径匹配 4 场景（精确/子目录/回退/无匹配）

## 参考

- 实验室原型：`examples/default/src/contract.rs`（41 测试）
- 设计文档：`examples/default/docs/contract.md`
- 理论来源：`docs/essay/contract/index.md`
