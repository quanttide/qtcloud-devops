# TODO — quanttide-devops 0.2 迁移

## 确认的 API 变更

| 0.1.5 | 0.2.1 |
|-------|-------|
| `VersionStatus` | `VersionState` |
| `version_status(repo, scope)` | `verify_version(repo, scope)` |

（其他变更待 cargo build 发现）

## 步骤

- [ ] `Cargo.toml`: `quanttide-devops = "0.2.1"`
- [ ] 适配 `contract.rs` API 变更

- [ ] `cargo test` 全量通过
