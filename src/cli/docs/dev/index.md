# 开发想法

## release status

当前 `release` 命令只负责执行发布，不提供查看发布状态的能力。增加 `release status` 可以查看当前项目的发布状态：

- 当前版本号
- 最新发布记录
- 未发布的变更摘要
- 预发布版本列表

每次操作开始和结束时都执行一次 `release status`，形成操作前后的状态对比，确保操作结果可追溯。

## plan

围绕特殊文件的规划命令。扫描项目中的特定文件（如 CHANGELOG.md、pyproject.toml、版本标记等），自动生成发布计划或变更摘要。

## build

围绕 CI 的构建命令。触发或查询 CI 构建状态，与 GitHub Actions 等 CI 系统交互。

## test

围绕测试的测试命令。运行测试套件并报告结果，支持过滤和摘要输出。

## 风格

以上命令遵循新的 release 命令的设计风格：

- Rust 实现
- 状态驱动
- 原子操作
- 与现有的 `code` / `release` 命令组一致的 CLI 接口
