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

## 安装

```bash
pip install qtcloud-devops-cli
```

也支持 `cargo install qtcloud-devops-cli` 和 GitHub Releases。详见 [安装指南](src/cli/docs/install.md)。

## 回滚

| 失败点 | 行为 |
|--------|------|
| 创建标签失败 | 无副作用 |
| 推送标签失败 | 删除本地标签 |
| GitHub Release 失败 | 删除本地和远程标签 |

## 详细文档

| 文档 | 说明 |
|------|------|
| [发布教程](release.md) | 完整发布流程（版本号 → 提交 → 发布 → 验证） |
| [发布命令](src/cli/docs/release.md) | stage / publish / cancel / retire 命令详解 |
| [子模块管理](src/cli/docs/code.md) | code status / sync / retire |
