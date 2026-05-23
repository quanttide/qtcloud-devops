# TODO

## Core — Rust 子模块管理引擎

### Step 1：复制 Rust 代码

- [ ] 从 `examples/default/` 复制以下内容到 `packages/code/`：
  - [ ] `Cargo.toml`、`Cargo.lock`
  - [ ] `rustfmt.toml`、`.gitignore`
  - [ ] `src/` 目录（全部）
  - [ ] `tests/` 目录（全部）
  - [ ] `docs/` 目录（全部）
- [ ] 验证目录结构：
  ```
  packages/code/
  ├── Cargo.toml
  ├── src/
  │   ├── main.rs
  │   ├── lib.rs
  │   ├── model/
  │   │   └── mod.rs
  │   └── commands/
  │       ├── mod.rs
  │       └── editor.rs
  ├── tests/
  │   └── integration.rs
  └── docs/
      └── user-guide.md
  ```

### Step 2：调整 Cargo.toml

- [ ] `pyo3` 改为默认依赖（移除 optional），确认 `crate-type = ["cdylib", "lib"]`
- [ ] 更新包名为 `qtcloud-devops-code`，lib 名为 `qtcloud_devops_code`
- [ ] 更新 `version` 字段为 `0.1.0`
- [ ] 更新 `authors` 字段
- [ ] 确认 `cargo build` 通过

### Step 3：配置 maturin 构建

- [ ] 在项目根 `pyproject.toml` 中添加 maturin 配置：
  ```toml
  [tool.maturin]
  module-name = "qtcloud_devops.code"
  source-dir = "packages/code"
  ```
- [ ] 在 `pyproject.toml` 的 `[project]` 中添加可选依赖组：
  ```toml
  [project.optional-dependencies]
  code = ["maturin>=1.0"]
  ```
- [ ] 验证 `pip install -e .[code]` 自动编译 Rust
- [ ] 验证 `python -c "from qtcloud_devops import code"` 导入成功

### Step 4：新增 app/code.py

- [ ] 创建 `app/__init__.py`（如不存在）
- [ ] 创建 `app/code.py`，暴露三个命令：
  ```python
  def status(path: str) -> dict: ...
  def sync(name: str | None, path: str) -> dict: ...
  def retire(name: str, path: str) -> dict: ...
  ```
- [ ] 在 `app/code.py` 中封装 Rust 调用的错误处理：
  - Rust 崩溃 → 返回友好错误信息
  - 非 git 仓库 → 提示而不是 panic
  - 子模块不存在 → 给出可用子模块列表
- [ ] 在 `app/cli.py` 中注册 `code` 子命令组：
  ```
  qtcloud-devops code status [path]
  qtcloud-devops code sync [name] [repo]
  qtcloud-devops code retire <name> [repo]
  ```
- [ ] 验证 `qtcloud-devops code status` 可运行

### Step 5：更新文档

- [ ] 更新 `AGENTS.md`：
  - 添加 Rust 开发环境要求（rustup, cargo）
  - 添加构建说明（`pip install -e .[rust]`）
  - 添加目录结构说明
- [ ] 更新 `README.md`：
  - 添加 `code` 子命令说明
  - 添加 Rust 依赖的安装说明

### Step 6：清理 examples/default

- [ ] 在 `examples/default/README.md` 中添加废弃说明
- [ ] 在 `examples/default/ROADMAP.md` 中添加迁移指引
- [ ] 移除 `examples/default` 子模块（确认不再引用后）

---

## P0 — 发布目标支持

- [ ] PyPI 发布集成
  - [ ] 版本校验（与 PyPI 已发布版本比对）
  - [ ] 构建（`python -m build`）
  - [ ] 发布（`twine upload` 或 `maturin upload`）
  - [ ] 验证（安装后导入测试）
- [ ] pub.dev 发布集成
- [ ] 发布目标抽象模型

## P1 — 体验修复

- [ ] CHANGELOG 路径智能检测

## P2 — 配置扩展

- [ ] 放宽分支限制
- [ ] 支持非 semver 版本策略
- [ ] CI Action 版本升级
- [ ] GitLink 镜像容灾同步
