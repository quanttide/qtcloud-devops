# qtcloud-devops-cli

DevOps CLI — 发布管理、契约检查与工作流自动化。

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

## 项目结构

```
apps/qtcloud-devops/src/cli/     ← Python CLI
├── app/
│   ├── cli.py                   # Typer 入口（release + code 子命令）
│   ├── code.py                  # Rust native 调用封装
│   ├── config.py                # pydantic-settings 配置
│   └── release.py               # 发布 Release 逻辑
├── packages/code/               ← Rust crate（maturin 构建）
│   ├── Cargo.toml
│   ├── pyproject.toml           # maturin 构建配置
│   └── src/
│       ├── lib.rs               # crate 入口
│       ├── main.rs              # standalone CLI 入口
│       ├── python.rs            # PyO3 绑定层
│       ├── model/mod.rs         # SubmoduleStatus, RepoState
│       └── commands/
│           ├── mod.rs           # SubmoduleEditor trait
│           └── editor.rs        # GitSubmoduleEditor 实现
├── tests/
│   └── ...
├── pyproject.toml               # setuptools 构建
├── AGENTS.md
├── CHANGELOG.md
├── ROADMAP.md
└── README.md
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

## 发布流程

1. **更新版本号** → 改 `pyproject.toml`
2. **写 CHANGELOG** → 更新 `CHANGELOG.md`
3. **提交** → `chore: bump qtcloud-devops-cli to vX.Y.Z`
4. **打 tag** → `cli/vX.Y.Z`
5. **推送** → CI 自动发布

## 命名约定

- 包名（PyPI）: `qtcloud-devops-cli`
- 导入名: `app.*`
- 仓库 tag 前缀: `cli/`
