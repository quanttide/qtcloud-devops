# 契约配置

## YAML 格式

契约配置文件位于 `.quanttide/devops/contract.yaml`。

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

| 场景 | 生效值 |
|------|--------|
| `scope.test_threshold = Some(90)` | 覆盖全局 `stages.test.threshold` |
| `scope.test_threshold = None` | 使用 `stages.test.threshold`（默认 70.0） |
| `scope.release.changelog = "src/cli/CHANGELOG.md"` | 只覆盖 changelog，pre_publish 走全局 |
| `scope.ci_workflow = None` | 按 `build-{scope}` 约定推导 |

## 公共 API

```rust
pub fn load(repo_path: &Path) -> Contract
pub fn load_scopes(repo_path: &Path) -> Vec<Scope>
pub fn status(repo_path: &Path)
pub fn status_to(writer: &mut impl Write, repo_path: &Path) -> io::Result<()>
pub fn version_status(repo_path: &Path, scope: &Scope) -> VersionStatus
pub fn detect_by_files(dir: &Path) -> Language
```
