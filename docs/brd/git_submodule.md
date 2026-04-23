# Git 子模块常见错误

本文档总结 Git 子模块操作中的常见错误及解决方案。

## 错误 1：Detached HEAD

### 症状
```
WARNING: you're about to be in detached HEAD mode.
```

### 原因
`git submodule update --remote` 后子模块进入游离状态，不在任何分支上。

### 解决
```bash
cd <子模块路径>
git checkout main
git pull
```

---

## 错误 2：重复子模块

### 症状
```
error: invalid submodule.<name>
```

### 原因
`.gitmodules` 中同一子模块路径重复定义。

### 解决
编辑 `.gitmodules` 去重，然后：
```bash
git submodule sync
```

---

## 错误 3：空仓库

### 症状
```
fatal: repository '<url>' does not have a commit yet
```

### 原因
尝试从空仓库添加子模块。

### 解决
先在子仓库添加内容并推送初始提交。

---

## 错误 4：远程未配置

### 症状
```
error: could not fetch from remote
```

### 原因
子模块没有配置 remote origin。

### 解决
```bash
git remote add origin <url>
git fetch origin
```

---

## 错误 5：未初始化

### 症状
```
克隆的子模块目录为空
```

### 原因
克隆主仓库时未使用 `--recurse-submodules`。

### 解决
```bash
git submodule update --init --recursive
```

---

## 错误 6：提交哈希冲突

### 症状
```
warning: couldn't find commit <hash>
```

### 原因
子模块引用指向的提交在子仓库中不存在。

### 解决
```bash
git submodule update --init --force
```

---

## 错误 7：config 残留

### 症状
```
error: could not deinit, still contains changes
```

### 原因
删除子模块后 `.git/config` 中残留配置。

### 解决
```bash
git config --file .git/config --remove-section submodule.<name>
```

---

## 错误 8：未指定分支

### 症状
```
submodule branch not configured
```

### 原因
子模块未配置跟踪分支。

### 解决
```bash
git config -f .gitmodules submodule.<name>.branch main
```

---

## 最佳实践

1. 更新前先检查状态：`git submodule status`
2. 更新后检查是否在 main 分支
3. 提交前确认分支正确
4. 删除子模块要清理所有位置