# qtcloud-devops-cli

DevOps CLI — 发布管理、契约检查与工作流自动化。

## 安装

### 前置依赖

- Python 3.12+，包管理器 uv
- **Rust 工具链**（`code` 子命令需要）：`rustup` + `cargo`
- **libgit2**：`sudo apt install libgit2-dev`（Ubuntu）或 `brew install libgit2`（macOS）

### 安装 CLI

```bash
cd apps/qtcloud-devops
pip install -e src/cli
```

### 安装 code 子命令（带 Rust 原生模块）

```bash
cd apps/qtcloud-devops/src/cli
pip install -e .[code]
```

这会通过 maturin 自动编译 `packages/code/` 下的 Rust crate。

## 项目结构

```
src/cli/
├── app/
│   ├── __init__.py
│   ├── cli.py          # Typer CLI 入口
│   ├── config.py       # pydantic-settings 配置
│   ├── contract.py     # 契约加载与查询
│   └── release.py      # 发布 Release 逻辑
├── tests/
│   ├── __init__.py
│   ├── conftest.py
│   ├── test_cli_commands.py
│   ├── test_release.py
│   └── test_contract.py
├── pyproject.toml
├── AGENTS.md
├── CHANGELOG.md
├── ROADMAP.md
└── .gitignore
```

## 用法

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
