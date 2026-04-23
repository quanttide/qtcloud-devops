# DevOps 契约

本文档定义 qtcloud-devops 仓库的 DevOps 规范，以 Contract 形式声明各环节的最佳实践。

## Git 契约

### 分支管理

- 主分支：`main`，仅接受 PR 合并
- 功能分支：`feature/<name>` 或 `fix/<name>`
- 发布分支：`release/v<version>`

### 提交规范

使用 Conventional Commits：

```
<type>: <description>
```

类型：
- `feat`：新功能
- `fix`：修复 bug
- `docs`：文档更新
- `test`：测试相关
- `refactor`：重构
- `chore`：构建/工具

### 提交流程

1. 创建分支
2. 开发并提交
3. 推送并创建 PR
4. 代码审查后合并

---

## 子模块契约

### 添加子模块

```bash
# 检查远程仓库非空
git ls-remote <url> HEAD

# 添加子模块
git submodule add <url> <path>

# 提交主仓库
git add <path>
git commit -m "feat: add <name> submodule"
```

### 更新子模块

```bash
# 检查子模块状态
git submodule status

# 初始化并更新
git submodule update --init <path>

# 拉取最新代码
git submodule update --remote <path>

# 检查是否在 main 分支
git -C <path> status

# 切换到 main 分支（如需）
git -C <path> checkout main

# 主仓库记录变更
git add <path>
git commit -m "chore: update <name> submodule"
```

### 删除子模块

```bash
# 取消初始化
git submodule deinit -f <path>

# 删除目录
rm -rf <path>

# 删除 Git 索引
git rm -f <path>

# 清理配置残留
git config --file .git/config --remove-section submodule.<name>

# 提交
git commit -m "chore: remove <name> submodule"
```

---

## 发布契约

### 版本号

遵循 semver：`v<major>.<minor>.<patch>`

### 发布流程

1. 更新 CHANGELOG.md
2. 创建标签
3. 推送标签
4. 创建 GitHub Release

### CHANGELOG 格式

```markdown
## [<version>] - <date>

### Added
- 新增内容

### Changed
- 变更内容

### Removed
- 移除内容
```

---

## 验证清单

### 提交前

- [ ] 代码符合规范
- [ ] 无未提交的变更
- [ ] 分支正确

### 发布前

- [ ] CHANGELOG 已更新
- [ ] 版本号正确
- [ ] 工作区干净

### 子模块操作前

- [ ] 子模块状态正常
- [ ] 远程仓库存在
- [ ] 分支正确

---

## 依赖工具

- `git`：版本控制
- `gh`：GitHub CLI
- `commitizen`：规范提交（可选）