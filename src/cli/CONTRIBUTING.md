# CONTRIBUTING

## 模块结构

```
src/
├── code/       # 业务层：纯抽象，不暴露 git 概念
├── git/        # 事实源底层：所有 git 操作
├── release/    # 发布子领域：publish + status
├── build.rs    # 构建状态查询
├── test.rs     # 测试状态查询
└── contract.rs # 契约适配层（委托 toolkit）
```

## 测试

```sh
cargo test                      # 全部测试（175）
cargo test --test release       # 仅 release 集成测试
cargo test --test code          # 仅 code 集成测试
cargo test --test cli           # 仅 cli 集成测试
cargo llvm-cov report           # 覆盖率报告
```

## 维护

### 项目结构

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

### 命名约定

- 包名（PyPI）: `qtcloud-devops-cli`
- 导入名: `app.*`
- 仓库 tag 前缀: `cli/`

## CI 工作流

| 工作流 | 触发 | 行为 |
|--------|------|------|
| `build-cli` | `release: [published]` + tag `cli/*` | 版本校验 → 三平台构建 → wheel 构建 |
| `publish-cli` | `workflow_run` (build-cli 成功) | publish-crate + publish-pypi（独立 job） |

## 测试

```sh
cargo test                      # 全部 175 测试
cargo test --test release       # 仅 release 集成测试
cargo test --test code          # 仅 code 集成测试
```

## 发布

1. **更新版本号** → 改 `pyproject.toml`
2. **写 CHANGELOG** → 更新 `CHANGELOG.md`
3. **提交** → `chore: bump qtcloud-devops-cli to vX.Y.Z`
4. **打 tag** → `cli/vX.Y.Z`
5. **推送** → CI 自动发布


## 开发环境

### Rust 工具链

```bash
# 安装 Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# 验证
rustc --version
cargo --version
```

### 系统依赖

```bash
# Ubuntu / Debian
sudo apt install libgit2-dev

# macOS
brew install libgit2
```

### 构建

```bash
# 完整安装（自动编译 Rust + 安装 Python CLI）
pip install -e .[code]

# 仅编译 Rust 核心（独立调试）
cd packages/code
cargo build
cargo build --release

# 仅安装 Python CLI（不编译 Rust）
pip install -e .
```

## 目录结构

```
packages/code/               ← Rust crate
├── Cargo.toml
├── src/
│   ├── lib.rs               ← crate 入口
│   ├── main.rs              ← standalone CLI 入口
│   ├── python.rs            ← PyO3 绑定层
│   ├── model/mod.rs         ← SubmoduleStatus, RepoState 等核心模型
│   └── commands/
│       ├── mod.rs           ← SubmoduleEditor trait
│       └── editor.rs        ← GitSubmoduleEditor 实现
├── tests/integration.rs
└── docs/user-guide.md

src/cli/                      ← Python CLI
├── app/
│   ├── cli.py               ← Typer 入口（release + code 子命令）
│   └── code.py              ← Rust native 调用封装
├── pyproject.toml            ← maturin 构建配置（source-dir 指向 packages/code）
└── ...
```

## 测试

```bash
# Rust 测试
cd packages/code && cargo test

# Python 测试
pytest

# 全部测试
cd packages/code && cargo test && cd ../.. && pytest
```

## 提交消息

- `feat:` — 新功能
- `chore:` — 版本号变更、配置更新
- `docs:` — 文档更新
- `fix:` — 修 bug
- `test:` — 测试
