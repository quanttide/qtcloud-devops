# qtcloud-devops-cli

量潮DevOps云命令行工具。

## 命令组

| 组 | 说明 |
|------|------|
| `release` | 发布管理（stage / publish / retire / status） |
| `code` | 子模块管理（status / sync / retire） |

## 快速开始

```bash
# 安装
pip install qtcloud-devops-cli
# 或 cargo install qtcloud-devops-cli

# 查看帮助
qtcloud-devops --help
qtcloud-devops release --help
qtcloud-devops code --help

# 预发布版本
qtcloud-devops release stage -v cli/v0.4.1-rc.1

# 正式发布
qtcloud-devops release publish -v cli/v0.4.1 -y
```

## 文档

| 文档 | 说明 |
|------|------|
| [release](release.md) | 发布管理命令详解 |
| [code](code.md) | 子模块管理命令详解 |
| [install](install.md) | 安装方式说明 |
