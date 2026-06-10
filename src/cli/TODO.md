# TODO — v0.5.0

## 最终目录结构

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
│   └── submodule.rs   # RepoState, SubmoduleStatus, scan, fetch, push, commit + GitSubmoduleEditor + HealthIssue
└── release/           # 发布子领域
    ├── mod.rs
    ├── stage.rs       # stage 命令（预发布）
    ├── publish.rs     # publish 命令（正式发布）
    └── util.rs        # 共享工具（validate_version, create_tag, extract_notes…）
```

## 已完成

- [x] `git/` 子领域：从 `model/code.rs` + `commands/code.rs` 迁入 submodule 模型和 git 操作
- [x] `code/` 子领域：SyncStatus 模型 + sync/status 命令（业务层包装）
- [x] `release/` 子领域：按 stage/publish/util 拆分
- [x] 删除旧 `commands/` 和 `model/` 目录
- [x] 更新 `lib.rs`、`main.rs`、`python.rs` 导入路径
- [x] 更新集成测试路径

## 剩余 P0

- [ ] `code status` 的 `offline` 参数实际生效（传入 `RepoState::scan` 跳过 fetch）
- [ ] 更新 `src/cli/AGENTS.md` CLI 设计规则为新结构

## 验证

- [x] `cargo build` 通过
- [x] `cargo test` 全部通过（141 测试）
