# DevOps 契约

本文档以约束形式声明 qtcloud-devops 仓库的 DevOps 规范。

## Git 约束

### 约束 1：禁止直接 push 到 main

```
不允许：git push origin main
必须：创建 PR，经审查后合并
```

### 约束 2：禁止 force push

```
不允许：git push --force
必须：使用 rebase 或 merge 解决分歧
例外：个人实验分支可 forcepush
```

### 约束 3：禁止空提交

```
不允许：git commit --allow-empty
必须：每次提交有实际变更
```

### 约束 4：使用规范提交格式

```
必须：Conventional Commits (<type>: <description>)
类型：feat, fix, docs, test, refactor, chore
禁止：自由格式 commit message
```

---

## 子模块约束

### 约束 5：禁止添加空仓库为子模块

```
不允许：git submodule add <empty-repo-url> <path>
必须：子仓库至少有初始提交
```

### 约束 6：子模块必须指定 branch

```
不允许：未配置 track 分支
必须：git config -f .gitmodules submodule.<name>.branch main
```

### 约束 7：子模块更新前检查分支

```
不允许：在 detached HEAD 状态提交
必须：git checkout main 后再提交
```

### 约束 8：禁止跳过子模块引用更新

```
不允许：只推送子模块不更新主仓库引用
必须：子模块推送后 git add <path> 提交
```

---

## 发布约束

### 约束 9：禁止跳过 CHANGELOG

```
不允许：发布前未更新 CHANGELOG.md
必须：按 Keep a Changelog 格式添加版本内容
```

### 约束 10：禁止重复版本号

```
不允许：git tag <已存在版本>
必须：确认版本号未使用
```

### 约束 11：禁止在工作区有变更时发布

```
不允许：git status 有变更时发布
必须：工作区干净后再发布
```

---

## 边界声明

### 边界 1：何时用 feature 分支

```
场景：任何非紧急修改
分支：feature/<name>
合并后：删除分支
```

### 边界 2：何时用 hotfix 分支

```
场景：生产环境 bug 修复
分支：hotfix/<name>
合并后：删除分支
```

### 边界 3：发布前必须检查

```
检查项：
- [ ] CHANGELOG 已更新
- [ ] 版本号符合 semver
- [ ] 工作区干净
- [ ] 测试通过
```

---

## 违反约束的处理

| 约束 | 违反处理 |
|------|---------|
| 约束 1-2 | 拒绝合并，要求重做 |
| 约束 3-4 | 提醒后强制修正 |
| 约束 5-8 | 拒绝提交，要求重做 |
| 约束 9-11 | 拒绝发布，要求重做 |