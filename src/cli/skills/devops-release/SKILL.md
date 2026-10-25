---
name: devops-release
description: 发布 Git 仓库 Release。使用 qtcloud-devops release publish 自动完成创建 tag、推送、GitHub Release。
---

# devops-release

发布 Git 仓库 Release。

## 规则

- 版本号遵循 semver：`vX.Y.Z` 或 `scope/vX.Y.Z`（如 `cli/v0.5.0`）
- 发布前确认 CHANGELOG.md 已包含目标版本的记录
- Release notes 来自 CHANGELOG.md 对应版本段
- 发布主仓库前确认所有子模块引用是最新的

## 前置条件

- [ ] CHANGELOG.md 已更新，包含目标版本的记录
- [ ] 版本号格式正确（`vX.Y.Z` 或 `scope/vX.Y.Z`）
- [ ] 工作区干净（无未提交变更）
- [ ] 标签不存在（避免重复发布）

## 工作流

### 1. 子模块发布 Release

在子模块目录中执行：

```bash
# 确认 CHANGELOG 已更新
grep "^## \[${VERSION#v}\]" CHANGELOG.md

# 发布（自动校验 → 创建 tag → 推送 → GitHub Release）
qtcloud-devops release publish --version vX.Y.Z

# 返回主仓库，更新子模块引用
cd <主仓库路径>
git add <子模块路径>
git commit -m "chore: update <子模块名> to vX.Y.Z"
git push
```

### 2. 主仓库发布 Release

```bash
# 确认所有子模块已更新
git submodule update --remote
git status

# 确认 CHANGELOG 已更新
grep "^## \[${VERSION#v}\]" CHANGELOG.md

# 发布
qtcloud-devops release publish --version vX.Y.Z
```

### 3. 跳过确认

自动发布场景（如 CI）加 `--yes`：

```bash
qtcloud-devops release publish --version vX.Y.Z --yes
```

## 错误处理和回滚

```bash
# 标签已创建但 Release 失败
git tag -d vX.Y.Z
git push origin --delete vX.Y.Z 2>/dev/null || true

# 恢复到发布前状态（如果有提交）
git reset --hard HEAD~1
```

## 常见错误

| 错误 | 原因 | 解决方案 |
|------|------|----------|
| CHANGELOG 缺少版本 | 忘记更新 CHANGELOG.md | 添加版本记录后再发布 |
| 标签已存在 | 重复发布 | 删除旧标签或使用新版本号 |
| 工作区脏 | 有未提交变更 | 提交或暂存变更后再发布 |

## 输出

### 成功时返回

```
✓ Release vX.Y.Z 创建成功
  标签: vX.Y.Z
  URL: https://github.com/quanttide/<repo>/releases/tag/vX.Y.Z
  提交: <SHA>
```

### 失败时返回

```
✗ Release vX.Y.Z 创建失败
  错误码: <ERROR_CODE>
  原因: <错误描述>
  建议: <解决方案>
```
