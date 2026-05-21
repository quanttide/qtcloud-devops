# 发布示例

> 事实源：[devops-release 技能](../../assets/skills/devops-release/SKILL.md)
> 对应 CLI：[qtcloud-devops-cli](../../src/cli) — `uv run python -m app.cli release`

## 执行模式说明

| 标识 | 含义 | 说明 |
|:----:|------|------|
| 🤖 规则 | 可自动化 | 静态规则/脚本即可执行，无需 AI 判断 |
| 🧠 AI | 需 AI 参与 | 需要 AI 向用户展示信息并等待确认，或处理异常决策 |

## 示例场景

发布某仓库的 `v0.4.0` 版本。

## 步骤

### 1. 预检查 — 🤖 规则

所有预检查均为自动化规则，CLI 内部由 `release.py:precheck()` 执行：

```bash
# 直接使用 CLI（dry-run 仅检查不执行）
qtcloud-devops release v0.4.0 --repo quanttide/quanttide-founder --dry-run
```

检查项：
- 版本号格式（semver）
- CHANGELOG.md 存在且包含目标版本
- 标签是否已存在
- 工作区是否干净
- 是否在可发布分支（main/master/release/*）

### 2. 发布前确认 — 🧠 AI

CLI 的 `release.py:confirm_release()` 向用户展示摘要并等待确认：

```text
发布版本: v0.4.0

检查结果:
  ✓ 预检查全部通过

Release Notes 预览:
  初始版本。

  ### Added
  - 功能 A
  - 功能 B

确认发布? (y/N):
```

AI 等待用户输入 `y/yes` 后继续。可使用 `-y` 跳过确认（CI 等自动化场景）：

```bash
qtcloud-devops release v0.4.0 --repo quanttide/quanttide-founder -y
```

### 3. 执行发布 — 🤖 规则

用户确认后，`release.py:run()` 自动依次执行：

1. `create_tag()` — `git tag v0.4.0`
2. `push_tag()` — `git push origin v0.4.0`
3. `create_release()` — `gh release create v0.4.0`

### 4. 验证 — 🤖 规则

`release.py:verify_release()` 自动执行 `gh release view` 验证结果：

```bash
gh release view v0.4.0 --repo quanttide/quanttide-founder
```

预期输出：

```
✓ Release v0.4.0 创建成功
  标签: v0.4.0
  URL: https://github.com/quanttide/quanttide-founder/releases/tag/v0.4.0
```

### 5. 一键执行

```bash
qtcloud-devops release v0.4.0 --repo quanttide/quanttide-founder
```

## 错误处理

| 阶段 | 错误 | 处理方式 | 模式 |
|------|------|----------|:----:|
| 预检查 | CHANGELOG 缺少版本 | 输出错误并终止，提示先更新 CHANGELOG | 🤖 规则 |
| 预检查 | 标签已存在 | 输出错误并终止，提示删除旧标签或使用新版本 | 🤖 规则 |
| 预检查 | 工作区脏 | 输出错误并终止，提示提交或暂存变更 | 🤖 规则 |
| 预检查 | 分支不正确 | 输出错误并终止，提示切换到正确分支 | 🤖 规则 |
| 执行 | 创建标签失败 | 输出错误并终止 | 🤖 规则 |
| 执行 | 推送标签失败 | 自动回滚（删除本地+远程标签），输出错误 | 🧠 AI |
| 执行 | Release 创建失败 | 自动回滚（删除本地+远程标签），输出错误 | 🧠 AI |
| 验证 | Release 验证失败 | 输出警告（标签和 Release 已存在，需人工检查） | 🤖 规则 |

## 参考

- 完整工作流见 [devops-release SKILL.md](../../assets/skills/devops-release/SKILL.md)
- CLI 源码见 [src/cli/release.py](../../src/cli/app/release.py)
- 子模块发布流程也请参考技能文档
