# DevOps 流程

量潮使用 Git 子模块组织多仓库项目，通过 `qtcloud-devops` CLI 统一管理。

## 八步流程

### Step 1：了解当前状态

```bash
cd /path/to/quanttide-devops

# 了解迭代计划、已知缺陷、待办事项
cat ROADMAP.md
cat BUGS.md
cat TODO.md

# 了解子模块状态
qtcloud-devops code status
```

### Step 2：同步子模块

```bash
# 同步所有子模块（推送 → 更新父指针 → 推送父仓库）
qtcloud-devops code sync
```

### Step 3：开发

在子模块内修改代码。完成后确保测试通过：

```bash
cd apps/qtcloud-devops/src/cli
cargo test          # 运行所有测试
cargo test --test cli     # CLI 集成测试
cargo test --test code    # 子模块测试
cargo test --test release # 发布流程测试
```

### Step 4：预发布验证

```bash
cd apps/qtcloud-devops/src/cli

# 版本一致性检查
./scripts/preflight.sh

# 标记预发布版本，触发 CI
qtcloud-devops release stage -v cli/v0.4.1-rc.1
```

### Step 5：CI 验证

等待 CI 完成构建和测试。确认无失败后进入下一步。

```bash
# 可以随时查看发布状态
qtcloud-devops release status
```

### Step 6：正式发布

```bash
cd apps/qtcloud-devops/src/cli

qtcloud-devops release publish -v cli/v0.4.1 -y
```

发布后 CI 自动推送到 crates.io 和 PyPI。

### Step 7：验证发布

```bash
# 确认发布状态
qtcloud-devops release status

# 确认注册源
cargo search qtcloud-devops-cli --registry crates-io
pip install qtcloud-devops-cli==0.4.1
```

### Step 8：维护

```bash
# 退役过时版本
qtcloud-devops release retire -v v0.3.0

# 退役废弃子模块
qtcloud-devops code retire old-module
```

## 文档

| 文档 | 说明 |
|------|------|
| [子模块管理](code.md) | code 命令使用场景与案例 |
| [发布教程](release.md) | 完整发布流程 |
