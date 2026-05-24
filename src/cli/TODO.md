# TODO

## v0.3.0 — 纯 Rust CLI，移除 Python 入口 `[BREAKING]`

### 1. Rust 重写 release 逻辑

- [ ] 实现状态机模型：`ReleaseRecord`、`ReleaseEntry`、`ReleaseStatus`、`Storage` trait、`FileStorage`
- [ ] 实现 `stage` 命令（标记版本）
- [ ] 实现 `publish` 命令（发布上线，含 tag + GitHub Release）
- [ ] 实现 `cancel` 命令（取消发布，含回滚 tag/Release）
- [ ] 实现 `retire` 命令（退役版本）
- [ ] 整合断言检查（git 工作区干净、合法分支）到 Rust 层
- [ ] 注册子命令到 `src/main.rs`（删除旧 `release` 命令）

### 2. 移除 Python CLI 入口

- [ ] 删除 `src/qtcloud_devops_cli/release.py`
- [ ] 更新 `pyproject.toml`：移除 `[project.scripts]` 入口点

### 3. 清理 Python 封装层

- [ ] 删除 `src/qtcloud_devops_cli/code.py`、`config.py`、`cli.py`
- [ ] `__init__.py` 只保留一行注释
- [ ] `_native.so` 作为 maturin 构建副产品保留，不主动维护

### 4. 保留 PyPI 分发

- [ ] 保留 `python` feature，`maturin build` 正常输出 wheel
- [ ] 配置 GitHub Releases 分发 Rust 二进制（附加渠道）
- [ ] 验证 `cargo install` 安装路径

### 5. 测试迁移

- [ ] **全部删除** `tests/python/` 下所有测试
- [ ] **全部删除** `integrated_tests/` 中依赖 Python CLI 的用例
- [ ] Rust 单元测试覆盖全部 release 逻辑路径（目标 ≥ 40 个新测试）
- [ ] 验收：`cargo test` 全部通过，无 Python 测试残留

### 6. 文档更新

- [ ] `docs/release.md` 更新为 Rust CLI，标注 BREAKING
- [ ] `docs/index.md` 更新构建/安装说明

---

## P0 — 发布目标支持

- [ ] pub.dev 发布集成
- [ ] 发布目标抽象模型

## P1 — 体验修复

- [ ] **Orphaned 状态拆分**（推迟自"开发中"）：拆分为更精确的子状态（rebase force push、squash merge、仓库替换等），更新 `RepoState::scan()` 和 `describe_issue()`

## P2 — 配置扩展

- [ ] 放宽分支限制（可配置允许的分支列表）
- [ ] 支持非 semver 版本策略
- [ ] CI Action 版本升级
- [ ] GitLink 镜像容灾同步
