# ROADMAP

## v0.1.0 ✅

发布 `release` 命令 — 预检查、打标签、推送、创建 GitHub Release。

### Done

- `release` 命令：预检查（版本号格式、CHANGELOG 存在性、tag 重复、工作区干净、分支检查）
- `contract-check` 命令：加载并展示 DevOps 契约

### Todo

- 契约检查集成到 `release` 预检查中
- 子模块发布支持
- `check` 命令：运行所有预定义契约检查
- `init` 命令：在当前目录初始化 DevOps 契约
