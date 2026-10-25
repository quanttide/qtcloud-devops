---
name: git-commit
description: 规范提交 Git 仓库变更，遵循 Conventional Commits 格式（feat/fix/docs/test/refactor/chore）。自动识别变更类型、生成提交信息、确认并推送。
---

# git-commit

规范提交 Git 仓库变更。

## 规则

- 提交信息遵循 Conventional Commits 格式：`<type>: <description>`
- 提交信息不超过 72 字符
- 不提交不完整的更改
- 提交前先 diff 确认变更内容

## 工作流

### 1. 检查状态

```bash
git status
git diff HEAD
```

### 2. 识别变更类型

根据变更内容推断 commit type：

| 类型 | 说明 | 示例 |
|------|------|------|
| `feat` | 新功能 | `feat: add user authentication` |
| `fix` | 修复 bug | `fix: resolve null pointer exception` |
| `docs` | 文档更新 | `docs: update README` |
| `test` | 测试相关 | `test: add unit tests for api` |
| `refactor` | 代码重构 | `refactor: simplify logic` |
| `chore` | 构建/工具 | `chore: update dependencies` |

### 3. 确认并提交

- 若有未暂存文件，询问用户是否 `git add -A`
- 向用户展示拟定的提交信息，确认后执行：

```bash
git commit -m "<type>: <description>"
```

### 4. 推送

提交成功后自动 push：

```bash
git push
```

### 5. 验证并返回链接

检查 push 是否成功，返回 GitHub 提交链接：

```bash
git log -1 --format="%H"
git remote get-url origin
```

组合返回格式：`https://github.com/{org}/{repo}/commit/{commit_hash}`
