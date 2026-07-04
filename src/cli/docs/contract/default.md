# 默认值

`contract.yaml` 可省略所有段，省略的部分走 `Default` 值。无文件时也使用这些默认值 + 自动推断 scope。

## 各维度默认值

| 维度 | 字段 | 默认值 |
|------|------|--------|
| **Stage** | build.command | `None` |
| | test.command | `None` |
| | test.threshold | `70.0` |
| | release.changelog | `"CHANGELOG.md"` |
| | release.pre_publish | `[]` |
| **Platform** | source_control | `Github` |
| | pipeline | `GithubActions` |
| | artifact_registry | `None` |
| **Source** | version.source_type | `Auto` |
| | version.path | `None` |
| **Scope** | language | `Unknown("auto")` |
| | framework | `""` |
| | build_tool | `Unknown("auto")` |
| | registry | `None` |
| | release | `StageRelease::default()` |
| | test_threshold | `None` |
| | ci_workflow | `None` |

## 自动推断时的额外覆盖

无 `contract.yaml` 时，`auto_detect_contract` 会在默认值基础上额外设置：

| 字段 | 默认值 → 推断值 |
|------|----------------|
| build.command | `None` → `Some("cargo build")` |
| test.command | `None` → `Some("cargo test")` |
| scope.registry | `None` → `Crates` |
| scope.build_tool | 按文件类型推断（Cargo.toml → `Cargo`） |
| scope.language | 按文件类型推断（Cargo.toml → `Rust`） |

## 代码引用

默认值定义在以下位置：

- **Toolkit Rust 包**：`packages/toolkit/packages/rust/src/contract/`
  - `stage.rs` — `Stage` / `StageBuild` / `StageTest` / `StageRelease` 的 `Default` impl
  - `platform.rs` — `Platform` 的 `Default` impl（source_control: Github, pipeline: GithubActions, registry: None）
  - `source.rs` — `Source` / `VersionSource` 的 `Default` impl（source_type: Auto）
  - `scope.rs` — `Language` / `BuildTool` 的 `Default` impl（Unknown("auto")）
- **CLI 适配层**：`src/cli/src/contract.rs`
  - `auto_detect_contract()` — 无文件时的智能推断
  - `infer_build_tool()` — 语言→构建工具的映射
