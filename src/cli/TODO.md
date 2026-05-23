# TODO

## Core — Rust 子模块管理引擎

### Step 1：复制 Rust 代码 ✅

- [x] 从 `examples/default/` 复制以下内容到 `packages/code/`：
  - [x] `Cargo.toml`、`Cargo.lock`
  - [x] `rustfmt.toml`
  - [x] `src/` 目录（全部）
  - [x] `tests/` 目录（全部）
  - [x] `docs/` 目录（全部）
- [x] 目录结构确认：
  ```
  packages/code/
  ├── Cargo.toml
  ├── Cargo.lock
  ├── rustfmt.toml
  ├── src/
  │   ├── main.rs
  │   ├── lib.rs
  │   ├── python.rs          ← PyO3 绑定层
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

### Step 2：调整 python.rs

**Cargo.toml**（源文件已满足，核实确认）：
- [x] `pyo3` 为默认依赖（非 optional），`crate-type = ["cdylib", "lib"]`
- [x] 包名 `qtcloud-devops-code`，lib 名 `qtcloud_devops_code`
- [x] `version = "0.1.0"`，`authors` 已设置

**`packages/code/src/python.rs`**：
- [x] 重命名 pymodule：`fn kse_core` → `fn qtcloud_devops_code`
- [x] 新增 `sync_single` / `sync_all` / `retire_submodule` pyfunction
- [x] `cargo build` + `cargo test`（22 tests）通过

### Step 3：配置 maturin 构建

- [x] `packages/code/pyproject.toml` 已配置 maturin
- [x] `src/cli/pyproject.toml` 用 setuptools（分离构建）
- [x] 验证 `python -c "from qtcloud_devops_code import scan_repo, sync_single, sync_all, retire_submodule"` 成功

### Step 4：新增 app/code.py

- [x] `app/code.py` 已创建，封装 `status()` / `sync()` / `retire()` 三个函数
- [x] 验证 `qtcloud-devops code status` 可用

### Step 5：更新文档

- [x] `README.md` 已更新（安装说明、项目结构、code 子命令用法）
- [x] `CONTRIBUTING.md` 已创建（Rust 工具链、构建、测试说明）

### Step 6：清理 examples/default

**已执行**：
- [x] `examples/default/README.md` 添加废弃说明
- [x] `examples/default/ROADMAP.md` 添加迁移指引
- [ ] 移除 `examples/default` 子模块（需确认不再引用后执行 `git submodule deinit examples/default && git rm examples/default`）

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
