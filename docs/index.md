# 量潮DevOps云

DevOps 基础设施仓库，封装量潮发布规范为可执行命令。

## 这是什么

量潮有一套自己的发布规范：版本号要 `vX.Y.Z`、CHANGELOG 要先写、分支限 main/master/release、子模块有 scoped tag、提交流程用 conventional commit……

这些规范写在很多文档里，但靠人记总会漏。qtcloud-devops 把这些规范**封装成了一条命令**：

```bash
pip install qtcloud-devops-cli
qtcloud-devops release --version v0.1.0
```

你告诉它版本号，它检查量潮的所有规范，通过后执行发布。不给你犯错的机会。

## 它帮你检查什么

```
✓ 版本号格式（vX.Y.Z 或 scope/vX.Y.Z）
✓ CHANGELOG 是否已写入
✓ 工作区是否干净
✓ 是否在可发布分支上（main / master / release/*）
```

全部通过后才执行。

## 输出

```
✓ 标签 v0.1.0 已创建并推送
✓ GitHub Release v0.1.0 已创建
  https://github.com/quanttide/xxx/releases/tag/v0.1.0
```

任何一步失败自动回滚。

## 适配量潮的仓库结构

量潮使用子模块组织多仓库。进入子模块目录执行，自动使用子模块的 remote：

```bash
cd apps/qtcloud-devops/src/cli
qtcloud-devops release --version cli/v0.1.0
```

不同子模块的语言和 tag 前缀不同，但流程一致。

## 也支持拆步

有些场景规范允许分步执行：

```bash
# 先打 tag 触发 CI
qtcloud-devops release --version v0.1.0 --tag-only

# CI 通过后补 Release
qtcloud-devops release --version v0.1.0 --release-only
```

## 发布流程

1. 更新 `pyproject.toml` 版本号（如有）
2. 写 CHANGELOG（格式参考已有版本）
3. 提交推送：`git add -A && git commit -m "chore: prepare CHANGELOG for v0.X.Y" && git push`
4. 发布：`qtcloud-devops release --version v0.X.Y`

## 规范之外

- `CHANGELOG.md` 默认在當前目录查找，包在子目录时需 `cd` 到包目录或用 `--changelog` 指定
- 仅支持 GitHub，GitLink 镜像需手动同步
- 仅限 main / master / release/* 分支可发布，其他分支执行会报错
