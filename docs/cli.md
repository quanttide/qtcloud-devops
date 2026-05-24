# CLI 命令参考

> `[]` = 可选参数，`<>` = 必填参数

> `[]` = 可选参数，`<>` = 必填参数

## 发布管理

```
stage   -v <version>          标记版本 → Staged
publish -v <version> [-y]     创建标签 + GitHub Release
cancel  -v <version>          取消 Staged 版本
retire  -v <version>          退役已发布版本（终态）
```

## 子模块管理

```
code status [path]    查看 Git 子模块状态
code sync   [name]    同步子模块指针到父仓库
code retire <name>    退役子模块
```

## 状态机

```
stage  → Staged
publish → Published
cancel  → Cancelled
retire  → Retired（终态）
```

## 安装

```bash
pip install qtcloud-devops-cli
```

也支持 `cargo install qtcloud-devops-cli` 和 GitHub Releases。详见[安装指南](../src/cli/docs/install.md)。

## 回滚

创建标签失败 → 无副作用
推送标签失败 → 删除本地标签
GitHub Release 失败 → 删除本地和远程标签

## 详细文档

- [发布教程](release.md) — 完整发布流程
- [发布命令详解](../src/cli/docs/release.md) — stage / publish / cancel / retire
- [子模块管理详解](../src/cli/docs/code.md) — code status / sync / retire
