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

**`packages/code/src/python.rs`**（需要手动修改）：
- [ ] 重命名 pymodule：`fn kse_core` → `fn qtcloud_devops_code`
  - 当前仍为 `fn kse_core`，Python 导入时会找不到模块
- [ ] 新增 `sync_single(name: String, path: String)` pyfunction
  - 调用 `GitSubmoduleEditor::new(root).sync_to_parent(&name)`
- [ ] 新增 `sync_all(path: String)` pyfunction
  - 调用 `GitSubmoduleEditor::new(root).sync_all_to_parent()`
- [ ] 新增 `retire_submodule(name: String, path: String)` pyfunction
  - 调用 `GitSubmoduleEditor::new(root).retire_submodule(&name)`
- [ ] 确认 `cargo build` 通过

### Step 3：配置 maturin 构建

- [x] `pyproject.toml` 已配置 maturin（`source-dir = "packages/code"`，`features = ["python"]`）
- [x] `[project.optional-dependencies] code = ["maturin>=1.0"]` 已添加
- [x] `lib.rs` 中 `#[cfg(feature = "python")] pub mod python;` 已启用
- [ ] 验证 `pip install -e .[code]` 自动编译 Rust（需 python.rs 修改完成后）
- [ ] 验证 `python -c "from qtcloud_devops.code import scan_repo"` 导入成功

### Step 4：新增 app/code.py

- [x] `app/code.py` 已创建，封装 `status()` / `sync()` / `retire()` 三个函数
- [x] `app/code.py` 包含 Rust native 模块不可用时的降级提示
- [ ] `python.rs` 绑定补全后，验证 `qtcloud-devops code status` 可用

### Step 5：更新文档

- [x] `AGENTS.md` 已分离开发环境信息到 `CONTRIBUTING.md`
- [x] `CONTRIBUTING.md` 已创建（含 Rust 工具链、构建、测试说明）
- [ ] 更新 `README.md`：
  - 添加 `code` 子命令说明
  - 添加 Rust 依赖的安装说明

### Step 6：清理 examples/default

**前置确认**：
- [ ] 确认 `examples/default` 的 CI/workflow 不再被引用
- [ ] 确认 `docs/` 和 `README` 不包含 `examples/default` 路径
- [ ] 确认 `AGENTS.md` 已指向 `packages/code`（已指向）

**执行**：
- [ ] `examples/default/README.md` 添加废弃说明，指向 `packages/code`
- [ ] `examples/default/ROADMAP.md` 添加迁移指引
- [ ] 移除 `examples/default` 子模块（`git submodule deinit examples/default && git rm examples/default`）

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
