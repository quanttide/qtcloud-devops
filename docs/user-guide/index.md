# DevOps 流程

量潮使用 Git 子模块组织多仓库项目，通过 `qtcloud-devops` CLI 统一管理发布和子模块。

## 日常开发流程

```
查看状态 → 同步子模块 → 修改代码 → 提交推送 → 发布
```

### 1. 查看项目状态

```bash
cd /path/to/quanttide-devops
qtcloud-devops code status
```

了解子模块状态：哪些需要同步、哪些有未提交修改。

### 2. 同步子模块

```bash
# 同步所有子模块
qtcloud-devops code sync
```

推送本地修改 → 更新父仓库指针 → 推送到远程。

### 3. 修改代码

在子模块内正常开发。完成后确保测试通过：

```bash
cd apps/qtcloud-devops/src/cli
cargo test
```

### 4. 发布新版本

详见[发布教程](release.md)。流程：

1. 更新版本号和 CHANGELOG
2. `qtcloud-devops release stage -v cli/v0.4.1-rc.1`（预发布）
3. CI 验证通过后
4. `qtcloud-devops release publish -v cli/v0.4.1 -y`（正式发布）
5. `qtcloud-devops release status`（确认状态）

### 5. 退役

```bash
# 退役子模块
qtcloud-devops code retire old-module

# 退役过时版本
qtcloud-devops release retire -v v0.3.0
```

## 文档

| 文档 | 说明 |
|------|------|
| [子模块管理](code.md) | code 命令使用场景与案例 |
| [发布教程](release.md) | 完整发布流程 |
