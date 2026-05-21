---
name: devops-release
description: 发布 Git 仓库 Release。必须先写 CHANGELOG 再打 tag，禁止跳步。支持子模块和主仓库两种流程。
---

# devops-release

> **⚠ 硬约束：不执行预检查 → 禁止发布**
> 加载此 Skill 后，必须按下方工作流从头到尾逐行执行命令。
> 标有"必须执行，不可跳过"的步骤是强制性的，AI 不得合并、跳过或提前执行后续步骤。

发布 Git 仓库 Release。

## 规则

- 版本号遵循 semver（MAJOR.MINOR.PATCH）
- **必须先更新 CHANGELOG.md，提交推送，再执行发布**
- 发布前确认工作区干净
- Release notes 只包含对应版本内容
- 发布主仓库前确认所有子模块引用是最新的

## 依赖

- devops-commit: 检查工作区状态
- devops-submodule: 检查子模块状态

## 工作流

### 1. 预检查

**必须执行，不可跳过**

```bash
# 检查工作区状态
git status

# 检查版本号格式（semver）
VERSION="v0.4.0"
if ! [[ "$VERSION" =~ ^v[0-9]+\.[0-9]+\.[0-9]+(-[a-zA-Z0-9.]+)?$ ]]; then
  echo "错误: 版本号格式错误，应为 vX.Y.Z 或 vX.Y.Z-qualifier"
  exit 1
fi

# 检查 CHANGELOG 是否包含目标版本
if ! grep -q "^## \[${VERSION#v}\]" CHANGELOG.md; then
  echo "错误: CHANGELOG.md 未找到 ${VERSION#v} 版本记录"
  echo "请先更新 CHANGELOG.md"
  exit 1
fi

# 提取版本内容测试
NOTES=$(sed -n "/^## \[${VERSION#v}\]/,/^## \[/p" CHANGELOG.md | sed '1d;$d')
if [ -z "$NOTES" ]; then
  echo "错误: 无法提取 ${VERSION#v} 版本内容"
  exit 1
fi

# 检查标签是否已存在
if git tag -l | grep -q "^${VERSION}$"; then
  echo "错误: 标签 $VERSION 已存在"
  exit 1
fi

# 预览 Release Notes
echo "=== Release Notes 预览 ==="
echo "$NOTES"
echo "========================="
```

### 2. 发布前确认

**向用户展示以下信息并请求确认**

```
发布版本: vX.Y.Z

检查结果:
✓ 版本号格式正确
✓ CHANGELOG.md 包含目标版本
✓ Release Notes 提取成功
✓ 标签不存在
✓ 工作区干净

待执行命令:
1. git tag vX.Y.Z
2. git push origin vX.Y.Z
3. gh release create vX.Y.Z --title "vX.Y.Z" --notes "..."

确认发布? (y/n)
```

### 3. 子模块发布 Release

```bash
# 1. 进入子模块目录
cd <子模块路径>

# 2. 执行预检查（步骤 1）

# 3. 创建并推送标签
git tag <version>
git push origin <version>

# 4. 创建 GitHub Release
gh release create <version> \
  --title "v<version>" \
  --notes "$(sed -n "/^## \[${VERSION#v}\]/,/^## \[/p" CHANGELOG.md | sed '1d;$d')" \
  --repo quanttide/<仓库名>
```

### 4. 主仓库发布 Release

```bash
# 1. 创建预发布版本（可选）
gh release create vX.Y.Z-rc.1 \
  --prerelease \
  --title "vX.Y.Z RC" \
  --notes "$(sed -n "/^## \[X.Y.Z\]/,/^## \[/p" CHANGELOG.md | sed '1d;$d')"

# 2. 确认所有子模块已更新
git submodule update --remote
git status

# 3. 更新 CHANGELOG.md

# 4. 提交 CHANGELOG.md
git add CHANGELOG.md && git commit -m "docs: update CHANGELOG for vX.Y.Z"

# 5. 执行预检查（步骤 1）

# 6. 发布前确认（步骤 2）

# 7. 创建标签并推送
git tag <version> && git push origin <version>

# 8. 创建 GitHub Release
gh release create <version> \
  --title "v<version>" \
  --notes "$(sed -n "/^## \[${VERSION#v}\]/,/^## \[/p" CHANGELOG.md | sed '1d;$d')" \
  --repo quanttide/quanttide-founder

# 9. 验证 Release
gh release view <version> --repo quanttide/quanttide-founder
```

### 5. 错误处理和回滚

```bash
# 标签已创建但 Release 失败
git tag -d <version>
git push origin --delete <version> 2>/dev/null || true

# 恢复到发布前状态（如果有提交）
git reset --hard HEAD~1

# 清理预发布版本
gh release delete vX.Y.Z-rc.1 --repo quanttide/quanttide-founder --yes
```

## 常见错误

| 错误 | 原因 | 解决方案 |
|------|------|----------|
| CHANGELOG 缺少版本 | 忘记更新 CHANGELOG.md | 添加版本记录后再发布 |
| 标签已存在 | 重复发布 | 删除旧标签或使用新版本号 |
| 工作区脏 | 有未提交变更 | 提交或暂存变更后再发布 |
| Release Notes 为空 | 版本格式不匹配 | 检查 CHANGELOG 版本标题格式 |
| 子模块未更新 | 子模块有新提交 | 执行 `git submodule update --remote` |

## 预发布检查清单

- [ ] 所有子模块版本已锁定
- [ ] 通过 CI 测试
- [ ] CHANGELOG.md 版本段已验证
- [ ] 执行过 `npm run build` (如适用)
- [ ] 版本号格式正确
- [ ] Release Notes 提取成功
- [ ] 工作区干净

## 输出

### 成功时返回

```
✓ Release vX.Y.Z 创建成功
  标签: vX.Y.Z
  URL: https://github.com/quanttide/quanttide-founder/releases/tag/vX.Y.Z
  提交: <SHA>
```

### 失败时返回

```
✗ Release vX.Y.Z 创建失败
  错误码: <ERROR_CODE>
  原因: <错误描述>
  建议: <解决方案>
```