# qtcloud-devops-cli

量潮科技 DevOps 命令行工具。纯 Rust 实现。

## 安装

```bash
pip install qtcloud-devops-cli
```

详见 [安装文档](install.md)。

## 快速开始

```bash
# 查看 Git 子模块状态
qtcloud-devops code status

# 发布新版本
qtcloud-devops stage -v v1.0.0
qtcloud-devops publish -v v1.0.0 -y
```

## 文档

| 文档 | 说明 |
|------|------|
| [code](code.md) | Git 子模块管理（status / sync / retire） |
| [release](release.md) | 发布管理（stage / publish / cancel / retire） |
| [install](install.md) | 安装方式说明 |

## 子命令一览

| 命令 | 功能 |
|------|------|
| `code status` | 扫描子模块状态（7 种状态分类） |
| `code sync [name]` | 同步子模块指针到父仓库 |
| `code retire <name>` | 退役子模块 |
| `stage -v <version>` | 标记版本为 Staged |
| `publish -v <version> [-y]` | 发布上线（标签 + GitHub Release） |
| `cancel -v <version>` | 取消发布 |
| `retire -v <version>` | 退役版本 |
