---
name: git-submodule
description: 管理 Git 子模块的完整生命周期：添加、移除、更新、修复冲突、查看状态。处理 detached HEAD、合并冲突等边缘情况。
---

# git-submodule

管理 Git 子模块（添加、移除、更新、修复）。

## 规则

- **最小干预**：仅执行用户明确请求的操作
- **原子提交**：每个子模块变更独立提交
- **验证优先**：操作后运行 `git status` 和 `git submodule status` 确认
- 子模块可能进入 detached HEAD 状态，提交前需切换到 main 分支

## 子命令

### add - 添加子模块

```bash
# 1. 检查远程仓库是否非空（不能是空仓库）
git ls-remote <url> HEAD

# 2. 添加子模块
git submodule add <url> <path>

# 3. 提交主仓库引用
git add <path>
git commit -m "feat: add <name> submodule"
```

### remove - 移除子模块

```bash
# 1. 取消初始化
git submodule deinit -f <path>

# 2. 从 Git 中移除
git rm -f <path>

# 3. （可选）删除云端仓库
gh repo delete <owner>/<repo> --yes

# 4. 清理 .git/config 残留
git config --file .git/config --remove-section submodule.<name>

# 5. 提交
git commit -m "chore: remove <name> submodule"
```

### update - 更新子模块

```bash
# 1. 先检查子模块状态（路径必须是完整路径，如 docs/fiction）
git submodule status

# 2. 初始化（如需要）
git submodule update --init <path>

# 3. 拉取最新代码
git submodule update --remote <path>

# 4. 检查子模块状态
git -C <path> status

# 3. 若在 detached HEAD 上有提交，切换到 main 分支并合并
git -C <path> checkout main
git -C <path> pull

# 4. 在主仓库记录变更
git add <path>
git commit -m "chore: update <name> submodule"

# 5. 推送
git push --recurse-submodules=on-demand
```

### fix - 修复子模块冲突

```bash
# 1. 若 rebase 失败，改用 merge
git merge --abort  # 如有需要
git pull --no-rebase --no-recurse-submodules

# 2. 解决 .gitmodules 冲突（手动编辑）

# 3. 进入子模块解决 commit hash 冲突
git -C <path> add -A
git -C <path> commit -m "fix: resolve merge conflict"

# 4. 记录解决结果
git add <path>
git commit -m "fix: resolve <name> submodule conflict"
```

### status - 查看状态

```bash
git status
git submodule status
```
