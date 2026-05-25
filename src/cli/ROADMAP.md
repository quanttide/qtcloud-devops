# ROADMAP

## v0.5.0 — 抽取 model crate

基于 `docs/roadmap/platform/module-refactor.md` 的结论，将纯数据模型从 `src/model/code.rs` 抽取为独立 crate `cli/crates/model/`。

**当前状态**：`src/model/code.rs` 超 1000 行，混有数据定义（SubmoduleStatus、RepoState）和 I/O 操作（`RepoState::scan()` 调用 git2）。

**目标**：`crates/model/` 只放纯数据模型，不引入 git2。

### 待办

- [ ] 创建 `crates/model/` workspace 成员
- [ ] 搬入 CommitHash、SubmoduleStatus、Submodule、RepoState、AggregateStatus
- [ ] 搬入 priority()、from_submodules() 等纯函数
- [ ] 更新 `src/` 下的 use 路径
- [ ] 测试随模型移动
- [ ] `RepoState::scan()` 留在原处（不是 model）

### 设计约束

- model crate 不引入 git2
- 触发条件：出现第二个 Rust 消费者时，再将 model 拆出为独立仓库

## 已完成

- [x] v0.4.2 — 子模块 fetch 修复

## 待定

- [ ] pub.dev 发布集成（`publish --registry pub-dev`）
