# 量潮DevOps云命令行工具(`qtcloud-devops-cli`)

## 安装

### 前置依赖

- Python 3.12+
- **Rust 工具链**（`code` 子命令需要）：`rustup` + `cargo`
- **libgit2**：`sudo apt install libgit2-dev`（Ubuntu）或 `brew install libgit2`（macOS）

### 安装 CLI（release 命令）

```bash
cd apps/qtcloud-devops
pip install -e src/cli
```

### 安装 code 子命令（Rust 原生模块）

```bash
pip install -e apps/qtcloud-devops/src/cli/packages/code
```

这会通过 maturin 自动编译 `packages/code/` 下的 Rust crate。

### 验证安装

```bash
qtcloud-devops --help        # 应看到 release 和 code 子命令
qtcloud-devops code status   # 扫描当前仓库子模块状态
```

## 用法

### release 命令

```bash
# 标签 + GitHub Release
qtcloud-devops release --version v0.1.0

# 仅标签
qtcloud-devops release --version v0.1.0 --tag-only

# 仅为已有标签补 GitHub Release
qtcloud-devops release --version v0.1.0 --release-only

# 仅预检查
qtcloud-devops release --version v0.1.0 --dry-run
```

### code 子命令

```bash
# 扫描子模块状态（三路 commit 比对 + 7 种状态分类）
qtcloud-devops code status [path]

# 同步子模块指针到父仓库
qtcloud-devops code sync [name] --repo path

# 退役子模块（deinit + .gitmodules + index 清理）
qtcloud-devops code retire <name> --repo path
```
