# 安装

支持三种安装方式：

## pip install（推荐）

```bash
pip install qtcloud-devops-cli
```

一行命令安装到 PATH，同时安装 `_native` 原生库到 site-packages（作为 maturin 构建副产品保留，不主动维护）。

最低门槛，适合 CI 和大多数开发者。

## cargo install

```bash
cargo install qtcloud-devops-cli
```

从源码编译，较慢但可获得最新提交。需要 Rust 工具链。

## GitHub Releases

从 [GitHub Releases](https://github.com/quanttide/qtcloud-devops/releases) 下载预编译二进制，解压后放入 PATH。

适合无法使用 pip 或 cargo 的环境。
