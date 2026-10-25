---
name: devops-audit
description: 审查仓库状态、CHANGELOG、版本一致性等，支持发布前检查、代码审查、文档审查等多场景。
---

# devops-audit

统一审查仓库状态，为多种工作流程提供前置检查。

## 功能

- 验证 CHANGELOG.md 版本连续性
- 检查未追踪文件
- 验证子模块状态
- 检查标签与代码一致性
- 验证版本号格式

## 使用场景

- 发布前验证
- 定期健康检查
- CI/CD 流水线检查

## 验证项

### 1. 版本号格式验证

```bash
validate_version_format() {
  local VERSION="$1"
  
  if [[ "$VERSION" =~ ^v[0-9]+\.[0-9]+\.[0-9]+(-[a-zA-Z0-9.]+)?$ ]]; then
    echo "✓ 版本号格式正确: $VERSION"
    return 0
  else
    echo "✗ 版本号格式错误: $VERSION"
    echo "  预期格式: vX.Y.Z 或 vX.Y.Z-qualifier"
    echo "  示例: v1.0.0, v0.4.0-alpha.1"
    return 1
  fi
}
```

### 2. CHANGELOG 验证

```bash
validate_changelog() {
  local VERSION="$1"
  local VERSION_NUM="${VERSION#v}"  # 移除 v 前缀
  
  # 检查 CHANGELOG 文件存在
  if [ ! -f "CHANGELOG.md" ]; then
    echo "✗ CHANGELOG.md 文件不存在"
    return 1
  fi
  
  # 检查目标版本存在
  if ! grep -q "^## \[${VERSION_NUM}\]" CHANGELOG.md; then
    echo "✗ CHANGELOG.md 未找到版本 [${VERSION_NUM}]"
    echo "  已有版本:"
    grep "^## \[" CHANGELOG.md | head -5
    return 1
  fi
  
  # 提取版本内容
  local NOTES=$(sed -n "/^## \[${VERSION_NUM}\]/,/^## \[/p" CHANGELOG.md | sed '1d;$d')
  
  if [ -z "$NOTES" ]; then
    echo "✗ 版本 [${VERSION_NUM}] 内容为空"
    return 1
  fi
  
  echo "✓ CHANGELOG.md 版本 [${VERSION_NUM}] 验证通过"
  echo "  内容预览:"
  echo "$NOTES" | head -3
  return 0
}
```

### 3. 工作区状态验证

```bash
validate_working_directory() {
  local STATUS=$(git status --porcelain)
  
  if [ -z "$STATUS" ]; then
    echo "✓ 工作区干净"
    return 0
  else
    echo "✗ 工作区有未提交变更:"
    echo "$STATUS"
    return 1
  fi
}
```

### 4. 子模块状态验证

```bash
validate_submodules() {
  # 检查是否有子模块
  if [ ! -f ".gitmodules" ]; then
    echo "✓ 无子模块"
    return 0
  fi
  
  # 检查子模块初始化状态
  local INIT_STATUS=$(git submodule status | grep -c "^-")
  if [ "$INIT_STATUS" -gt 0 ]; then
    echo "✗ 有 $INIT_STATUS 个子模块未初始化"
    git submodule status | grep "^-"
    return 1
  fi
  
  # 检查子模块更新状态
  local UPDATE_STATUS=$(git submodule status | grep -c "^+")
  if [ "$UPDATE_STATUS" -gt 0 ]; then
    echo "⚠ 有 $UPDATE_STATUS 个子模块有新提交"
    git submodule status | grep "^+"
    echo "  建议: git submodule update --remote"
  fi
  
  echo "✓ 子模块状态验证通过"
  return 0
}
```

### 5. 标签验证

```bash
validate_tag() {
  local VERSION="$1"
  
  if git tag -l | grep -q "^${VERSION}$"; then
    echo "✗ 标签 $VERSION 已存在"
    echo "  现有标签:"
    git tag -l | grep "^v" | tail -5
    return 1
  else
    echo "✓ 标签 $VERSION 不存在，可以创建"
    return 0
  fi
}
```

### 6. 远程仓库验证

```bash
validate_remote() {
  local REMOTE="$1"
  
  if ! git remote | grep -q "^${REMOTE}$"; then
    echo "✗ 远程仓库 '$REMOTE' 不存在"
    git remote -v
    return 1
  fi
  
  # 检查远程连接
  if ! git ls-remote "$REMOTE" &>/dev/null; then
    echo "✗ 无法连接到远程仓库 '$REMOTE'"
    return 1
  fi
  
  echo "✓ 远程仓库 '$REMOTE' 验证通过"
  return 0
}
```

## 完整验证流程

```bash
validate_release() {
  local VERSION="$1"
  local REMOTE="${2:-origin}"
  
  echo "=== 发布前验证: $VERSION ==="
  echo
  
  local ERRORS=0
  
  # 1. 版本号格式
  validate_version_format "$VERSION" || ((ERRORS++))
  
  # 2. CHANGELOG
  validate_changelog "$VERSION" || ((ERRORS++))
  
  # 3. 工作区状态
  validate_working_directory || ((ERRORS++))
  
  # 4. 子模块状态
  validate_submodules || ((ERRORS++))
  
  # 5. 标签
  validate_tag "$VERSION" || ((ERRORS++))
  
  # 6. 远程仓库
  validate_remote "$REMOTE" || ((ERRORS++))
  
  echo
  echo "=== 验证结果 ==="
  
  if [ "$ERRORS" -eq 0 ]; then
    echo "✓ 所有验证通过，可以发布"
    return 0
  else
    echo "✗ 发现 $ERRORS 个错误，请修复后再发布"
    return 1
  fi
}

# 使用示例
validate_release "v0.4.0"
```

## 集成到 devops-release

在 devops-release 的预检查步骤中调用：

```bash
# 在 .agents/skills/devops-release/SKILL.md 中
# 步骤 1. 预检查

# 调用 devops-audit
.agents/skills/devops-audit/SKILL.md validate_release "$VERSION"
if [ $? -ne 0 ]; then
  echo "发布前审查失败，请修复后再试"
  exit 1
fi
```