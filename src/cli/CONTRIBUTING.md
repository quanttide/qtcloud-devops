# CONTRIBUTING

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
