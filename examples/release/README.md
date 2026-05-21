# Release 发布示例

> 事实源：[devops-release 技能](../../assets/skills/devops-release/SKILL.md)

本示例演示如何基于 `devops-release` 技能发布一个 Git 仓库 Release。

## 示例场景

发布 `quanttide/quanttide-founder` 仓库的 `v0.4.0` 版本。

## 步骤

### 1. 预检查

```bash
git status

VERSION="v0.4.0"
if ! [[ "$VERSION" =~ ^v[0-9]+\.[0-9]+\.[0-9]+(-[a-zA-Z0-9.]+)?$ ]]; then
  echo "错误: 版本号格式错误"
  exit 1
fi

if ! grep -q "^## \[${VERSION#v}\]" CHANGELOG.md; then
  echo "错误: CHANGELOG.md 未找到 v0.4.0 版本记录"
  exit 1
fi

NOTES=$(sed -n "/^## \[${VERSION#v}\]/,/^## \[/p" CHANGELOG.md | sed '1d;$d')
if [ -z "$NOTES" ]; then
  echo "错误: 无法提取 Release Notes"
  exit 1
fi

if git tag -l | grep -q "^${VERSION}$"; then
  echo "错误: 标签 v0.4.0 已存在"
  exit 1
fi

echo "=== Release Notes 预览 ==="
echo "$NOTES"
echo "========================="
```

### 2. 发布前确认

确认信息无误后，执行发布：

```bash
git tag v0.4.0 && git push origin v0.4.0

gh release create v0.4.0 \
  --title "v0.4.0" \
  --notes "$NOTES" \
  --repo quanttide/quanttide-founder

gh release view v0.4.0 --repo quanttide/quanttide-founder
```

### 3. 验证

```bash
# 预期输出
# ✓ Release v0.4.0 创建成功
#   标签: v0.4.0
#   URL: https://github.com/quanttide/quanttide-founder/releases/tag/v0.4.0
```

## 参考

- 完整工作流及错误处理见 [devops-release SKILL.md](../../assets/skills/devops-release/SKILL.md)
- 子模块发布流程也请参考技能文档
