# qtcloud-devops-cli PyPI 分发包

本目录是 `pip install qtcloud-devops-cli` 的分发入口。主要作用是将 Rust 二进制安装到用户 PATH。

## 工作原理

`pyproject.toml` 中配置 `source-dir = "../.."`（指向 Rust crate），maturin 构建时会：

1. 编译 Rust 源码为 `qtcloud-devops` 二进制 + `_native.so` 原生库
2. 打包为 wheel 发布到 PyPI

## 为什么存在

- `cargo install` 需要 Rust 工具链，门槛高
- GitHub Releases 需要手动下载解压，不够自动化
- `pip install` 一行命令，是门槛最低的分发方式

## 维护状态

此包的内容**不主动维护**。核心代码在 `../src/`（Rust），本目录仅保留 PyPI 分发所需的脚手架文件。
