# TODO — v0.5.0

## 目录结构

```
src/
├── main.rs
├── lib.rs
├── code/              # 业务层：纯抽象，不暴露 git 概念
│   ├── mod.rs
│   ├── model.rs       # SyncStatus（Synced / PendingPush / PendingPull / Conflict）
│   ├── sync.rs        # sync 命令逻辑
│   └── status.rs      # status 命令逻辑
├── git/               # 事实源底层：所有 git 操作
│   ├── mod.rs
│   └── submodule.rs   # RepoState, SubmoduleStatus, scan, fetch, push, commit…
└── release/           # 发布子领域（已有，保留）
    ├── mod.rs
    └── status.rs
    ----publish.rs
```

## P0: `git/` 子领域（事实源底层）

- [ ] 创建 `git/` 目录结构
- [ ] 从 `model/code.rs` 迁入：`RepoState`、`SubmoduleStatus`、`CommitHash`、`scan()`、`scan_all()`、`AggregateStatus`
- [ ] 从 `commands/code.rs` 迁入：`fetch_submodule`、`push_submodule`、`update_parent_pointer`、`push_parent`、`revert_parent_commit`
- [ ] 抽象 git 错误统一处理（`git/error.rs` 或 mod.rs 内部）
- [ ] `git/submodule.rs` 只对 `code/` 暴露必要接口，不直接对 CLI 暴露

## P0: `code/` 子领域（业务层）

- [ ] 创建 `code/` 目录结构
- [ ] `code/model.rs`: 定义 `SyncStatus` 四态（Synced / PendingPush / PendingPull / Conflict），替代 `SubmoduleStatus` 七态
- [ ] `code/status.rs`: 调用 `git::submodule::scan()`，将七态映射为四种业务状态输出
- [ ] `code/sync.rs`: 调用 `git::submodule` 操作，封装为 `sync(name)` 原语
- [ ] `code/model.rs`: 删除 `HealthIssue`（暴露子模块概念），替换为业务错误/报告类型

## P0: 清理旧结构

- [ ] 删除 `commands/` 目录（内容已迁入 `code/` 和 `release/`）
- [ ] 删除 `model/` 目录（内容已迁入 `git/`）
- [ ] 更新 `lib.rs` 公开路径
- [ ] 更新 `main.rs` 导入路径和 CLI 定义
- [ ] 修复三次重复 scan + `--offline` 生效

## P0: 更新 AGENTS.md

- [ ] `code` 命令列表只保留 `code sync` 和 `code status`
- [ ] 新增 `submodule` 作为调试后门（仅 --verbose 时提及）
- [ ] 同步 `src/cli/AGENTS.md` 与子模组 `AGENTS.md`

## 验证

- [ ] `cargo build` 通过
- [ ] `cargo test` 全部通过
- [ ] `code status` 输出不出现 `SubmoduleStatus` 枚举名
