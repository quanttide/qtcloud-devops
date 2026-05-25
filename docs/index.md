# 量潮DevOps云

DevOps 基础设施仓库，封装量潮发布规范为可执行命令。

量潮有自己的发布规范：版本号 `vX.Y.Z`、CHANGELOG 要先写、分支限 main/master/release。qtcloud-devops 把这些规范封装成了命令：

```bash
pip install qtcloud-devops-cli
qtcloud-devops release stage -v cli/v0.4.1-rc.1
qtcloud-devops release publish -v cli/v0.4.1 -y
```

## 安装

```bash
pip install qtcloud-devops-cli
```

也支持 `cargo install` 和 GitHub Releases。详见 [安装指南](api-references/install.md)。

## 快速开始

```bash
# 预发布
qtcloud-devops release stage -v cli/v0.4.1-rc.1

# 正式发布
qtcloud-devops release publish -v cli/v0.4.1 -y

# 查看子模块状态
qtcloud-devops code status
```

## 文档

| 文档 | 说明 |
|------|------|
| [发布管理](api-references/release.md) | release stage / publish / retire / status |
| [子模块管理](api-references/code.md) | code status / sync / retire |
| [安装指南](api-references/install.md) | pip / cargo / GitHub Releases |
| [用户指南](user-guide/code.md) | code 命令使用场景与案例 |
