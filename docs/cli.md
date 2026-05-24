# CLI 命令参考

## 子命令一览

| 命令 | 功能 |
|------|------|
| `stage -v <version>` | 标记版本，进入 Staged 状态 |
| `publish -v <version> [-y]` | 创建标签 + GitHub Release |
| `cancel -v <version>` | 取消 Staged 版本 |
| `retire -v <version>` | 退役已发布版本（终态） |
| `code status [path]` | 查看 Git 子模块状态 |
| `code sync [name]` | 同步子模块指针到父仓库 |
| `code retire <name>` | 退役子模块 |

## 状态机

```
stage  →  Staged     — 版本已验证，等待发布
publish →  Published  — 创建标签 + GitHub Release
cancel  →  Cancelled  — 取消发布
retire  →  Retired    — 退役版本（不可逆）
```

## 预检查

publish 时自动检查：

```
✓ 版本号格式（vX.Y.Z 或 scope/vX.Y.Z）
✓ CHANGELOG.md 是否包含目标版本
✓ 工作区是否干净
✓ 是否在可发布分支（main / master / release/*）
```

全部通过后才执行。任何一步失败自动回滚标签。

## 子模块适配

量潮使用子模块组织多仓库。进入子模块目录执行，自动使用子模块 remote：

```bash
cd apps/qtcloud-devops/src/cli
qtcloud-devops stage -v cli/v0.3.0
```

不同子模块的语言和 tag 前缀不同，但流程一致。

## 发布流程

1. 写 CHANGELOG（格式参考已有版本）
2. 提交推送
3. `qtcloud-devops stage -v v0.X.Y`
4. `qtcloud-devops publish -v v0.X.Y -y`

## 详细文档

| 文档 | 说明 |
|------|------|
| [release](src/cli/docs/release.md) | 发布命令详解、迁移指南 |
| [code](src/cli/docs/code.md) | 子模块管理命令详解 |
| [install](src/cli/docs/install.md) | 安装方式说明 |
