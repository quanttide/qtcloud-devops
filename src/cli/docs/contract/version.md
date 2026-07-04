# 版本一致性检查

```rust
pub fn version_status(repo_path: &Path, scope: &Scope) -> VersionStatus
```

## 对比维度

| 来源 | 读取方式 | 格式 |
|------|---------|------|
| **tag 版本** | `git tag --sort=-version:refname`，按 scope 前缀过滤 | `vX.Y.Z`、`scope/vX.Y.Z` |
| **配置版本** | 按语言读取配置文件中的 `version` 字段 | `X.Y.Z` |

两者比较前均经过 `normalize_version` 去 `v` 前缀和 scope 前缀。

## 一致性判定

| tag | config | 判定 |
|-----|--------|------|
| 有 | 有，且相同 | ✅ 一致 |
| 有 | 有，但不同 | ❌ 不一致 |
| 有 | 无 | ❌ 配置缺失 |
| 无 | 有 | ❌ 未打 tag |
| 无 | 无 | ✅ 一致（空仓库） |

## 测试覆盖

- git 异常降级（`/nonexistent` 目录返回空结构）
- scoped tag vs 根 tag 的过滤
- v 前缀剥离后的比较
