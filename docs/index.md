# 量潮DevOps云

DevOps 基础设施仓库，封装量潮发布规范为可执行命令。

## 安装

```bash
pip install qtcloud-devops-cli
```

也支持 `cargo install` 和 GitHub Releases。

## 快速开始

```bash
# 查看子模块状态
qtcloud-devops code status

# 预发布版本
qtcloud-devops release stage -v cli/v0.4.1-rc.1

# 正式发布
qtcloud-devops release publish -v cli/v0.4.1 -y
```

## 文档

- [用户指南](user-guide/index.md) — 完整 DevOps 八步流程
- [发布管理](api-references/release.md) — release stage / publish / retire / status
- [子模块管理](api-references/code.md) — code status / sync / retire
- [安装指南](api-references/install.md) — pip / cargo / GitHub Releases
