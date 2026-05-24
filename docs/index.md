# 量潮DevOps云

DevOps 基础设施仓库，封装量潮发布规范为可执行命令。

量潮有自己的发布规范：版本号 `vX.Y.Z`、CHANGELOG 要先写、分支限 main/master/release。qtcloud-devops 把这些规范封装成了命令：

```bash
pip install qtcloud-devops-cli
qtcloud-devops stage -v v0.1.0
qtcloud-devops publish -v v0.1.0 -y
```

告诉它版本号，它检查所有规范，通过后执行。详见 [CLI 命令参考](cli.md)。

## 安装

```bash
pip install qtcloud-devops-cli
```

也支持 `cargo install` 和 GitHub Releases。详见 [安装文档](src/cli/docs/install.md)。

## 快速开始

```bash
# 标记版本
qtcloud-devops stage -v v1.0.0

# 发布上线
qtcloud-devops publish -v v1.0.0 -y
```

## 文档

| 文档 | 说明 |
|------|------|
| [CLI 命令参考](cli.md) | 所有子命令、状态机、预检查 |
| [安装指南](src/cli/docs/install.md) | pip / cargo / GitHub Releases |
| [发布管理](src/cli/docs/release.md) | stage / publish / cancel / retire |
| [子模块管理](src/cli/docs/code.md) | code status / sync / retire |
