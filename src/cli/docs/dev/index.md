# 开发想法

## release status

当前 `release` 命令只负责执行发布，不提供查看发布状态的能力。增加 `release status` 可以查看当前项目的发布状态。

每次操作开始和结束时都执行一次 `release status`，形成操作前后的状态对比，确保操作结果可追溯。

## plan

围绕项目规划文件的命令。扫描 BUGS、ROADMAP、TODO、STATUS、CHANGELOG 等项目管理文件，生成变更摘要或进度报告。

## build

围绕构建的命令。运行本地构建（Rust 的 `cargo build`、Python 的 `uv build` 等），不依赖网络。

## test

围绕测试的命令。运行本地测试套件，解析输出，生成摘要报告。支持按语言过滤（`--rust`、`--python`）。

## 风格

以上命令遵循新的 release 命令的设计风格：

- Rust 实现
- 状态驱动
- 原子操作
- 与现有的 `code` / `release` 命令组一致的 CLI 接口
