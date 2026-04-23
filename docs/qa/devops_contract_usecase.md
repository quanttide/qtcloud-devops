# DevOps 约束 usecase

## usecase 1：force push 到 main

### 场景

在 main 分支上执行了 `git push --force`

### 违反约束

- 禁止 force push 到 main

### 处理

拒绝合并，要求重做

### 避免

使用 rebase 或 merge 解决分歧

---

## usecase 2：detached HEAD 提交

### 场景

子模块更新后，在游离状态直接提交

### 违反约束

- 禁止在 detached HEAD 状态提交
- 子模块必须指定 branch

### 处理

拒绝提交，要求重做

### 避免

```bash
git checkout main
git pull
# 再提交
```

---

## usecase 3：跳过子模块引用更新

### 场景

只推送了子模块，没有更新主仓库的子模块引用

### 违反约束

- 禁止跳过子模块引用更新

### 处理

拒绝提交，要求重做

### 避免

```bash
git push
cd ..
git add <子模块路径>
git commit -m "chore: update <name> submodule"
```

---

## usecase 4：跳过 CHANGELOG 更新

### 场景

发布新版本但未更新 CHANGELOG.md

### 违反约束

- 禁止跳过 CHANGELOG 更新

### 处理

拒绝发布，要求重做

### 避免

按 Keep a Changelog 格式添加版本内容

---

## usecase 5：工作区有变更时发布

### 场景

有未提交的变更就创建标签发布

### 违反约束

- 禁止在工作区有变更时发布

### 处理

拒绝发布，要求重做

### 避免

```bash
git status  # 确认无变更
```