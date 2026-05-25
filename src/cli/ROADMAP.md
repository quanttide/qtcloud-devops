# ROADMAP

## v0.5.0 — 模块重构

基于 `docs/roadmap/platform/module-refactor.md` 的结论，主动治理代码结构。

### 1. 抽取 model crate

`cli/crates/model/` — 纯数据模型，不引入 git2 等 I/O 依赖。

- [ ] CommitHash（纯字符串包装）
- [ ] SubmoduleStatus（三分法：Synchronized / OutOfSync / Anomaly）
- [ ] Submodule
- [ ] RepoState
- [ ] AggregateStatus
- [ ] priority()、from_submodules() 等纯函数
- [ ] 测试随模型移动

测试随模型移动。集成测试留在原处。

### 2. 抽取 git crate

`cli/crates/git/` — 底层 git 操作封装。

- [ ] RepoState::scan() — 子模块状态扫描
- [ ] fetch / push 等 git 操作
- [ ] 测试

### 3. code crate

`cli/crates/code/` — DevOps 原语（sync、status、repair）。

当前逻辑暂留 `src/commands/code.rs`，待 model 和 git 稳定后抽取。

### 4. CLI 入口

`src/` 仅剩参数解析与分发，不做业务逻辑。

### 设计约束

- model 不引入 git2
- git 返回 model 类型
- code 协调 model 和 git，实现业务逻辑

## 已完成

- [x] v0.4.2 — 子模块 fetch 修复（`RepoState::scan()` 内逐子模块 fetch）

## 待定

- [ ] pub.dev 发布集成（`publish --registry pub-dev`）
