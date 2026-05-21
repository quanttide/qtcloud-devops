# 发布示例

> 事实源：[devops-release 技能](../../assets/skills/devops-release/SKILL.md)

## 执行模式说明

| 标识 | 含义 | 说明 |
|:----:|------|------|
| 🤖 规则 | 可自动化 | 静态规则/脚本即可执行，无需 AI 判断 |
| 🧠 AI | 需 AI 参与 | 需要 AI 向用户展示信息并等待确认，或处理异常决策 |

## 示例场景

发布某仓库的 `v0.4.0` 版本。

## 步骤

### 1. 预检查 — 🤖 规则

所有预检查均为自动化规则，可直接执行：

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

### 2. 发布前确认 — 🧠 AI

AI 向用户展示检查结果摘要，等待用户确认后再继续：

```text
发布版本: v0.4.0

检查结果:
✓ 版本号格式正确
✓ CHANGELOG.md 包含目标版本
✓ Release Notes 提取成功
✓ 标签不存在
✓ 工作区干净

待执行命令:
1. git tag v0.4.0 && git push origin v0.4.0
2. gh release create v0.4.0 --title "v0.4.0" --notes "..."

确认发布? (y/n)
```

### 3. 执行发布 — 🤖 规则

用户确认后，自动执行以下命令：

```bash
git tag v0.4.0 && git push origin v0.4.0

gh release create v0.4.0 \
  --title "v0.4.0" \
  --notes "$NOTES" \
  --repo quanttide/quanttide-founder

gh release view v0.4.0 --repo quanttide/quanttide-founder
```

### 4. 验证 — 🤖 规则

```bash
# 预期输出
# ✓ Release v0.4.0 创建成功
#   标签: v0.4.0
#   URL: https://github.com/quanttide/quanttide-founder/releases/tag/v0.4.0
```

## 错误处理

| 错误 | 处理方式 | 模式 |
|------|----------|:----:|
| CHANGELOG 缺少版本 | 输出错误信息并终止，提示用户先更新 CHANGELOG | 🤖 规则 |
| 标签已存在 | 输出错误信息并终止，提示用户删除旧标签或使用新版本 | 🤖 规则 |
| 工作区脏 | 输出错误信息并终止，提示用户提交或暂存变更 | 🤖 规则 |
| Release Notes 为空 | 输出错误信息并终止，提示检查 CHANGELOG 格式 | 🤖 规则 |
| 标签已推送但 Release 创建失败 | 向用户展示错误，询问是否删除标签并回滚 | 🧠 AI |
| 其他不可预见的错误 | 向用户展示完整错误信息，由用户决策下一步 | 🧠 AI |

## 参考

- 完整工作流见 [devops-release SKILL.md](../../assets/skills/devops-release/SKILL.md)
- 子模块发布流程也请参考技能文档
