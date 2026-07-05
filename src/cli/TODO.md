# TODO — quanttide-devops 0.2 迁移

## 确认的 API 变更

| 0.1.5 | 0.2.1 |
|-------|-------|
| `VersionStatus` | `VersionState` |
| `version_status(repo, scope)` | `verify_version(repo, scope)` |
| `detect_language_by_files` | `config_file::detect_language` |
| `read_all_config_versions` | `config_file::read_config_versions` |
| `source::git::*` (整个模块) | `source::git_tag` + `source::config_file` |
| `GitSourceError` | 移除（`verify_version` 返回 `Box<dyn Error>`） |

## 步骤

- [x] `Cargo.toml`: `quanttide-devops = "0.2.1"`
- [x] 适配 `contract.rs` API 变更
- [x] `cargo test` 全量通过（419 passed）
- [ ] USTC mirror 同步后切回 registry 依赖
