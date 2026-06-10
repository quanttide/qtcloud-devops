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

- [x] 三子领域拆分（code / git / release）
- [x] 删除旧 `commands/` 和 `model/` 目录
- [x] 更新全部导入路径和集成测试

## 已完成

- [x] `code status` 的 `offline` 参数实际生效（`RepoState::scan_offline` 跳过 fetch）
- [x] `HealthIssue.status` 从 `SubmoduleStatus` 改为 `String`
- [x] `python.rs`: 删除 `retire_submodule` 绑定、修复测试路径
- [x] `release/` 导出层级统一：工具函数在 mod 层重新导出
- [x] `code/sync.rs` 做实：返回 `SyncStatus` 业务状态

## P1

- [ ] 测试辅助函数去重：将 `git_init()` 提取为共享测试工具
