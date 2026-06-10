# TODO — v0.5.0

## P0: 取消 `code retire`

- [ ] `commands/mod.rs`: 从 `SubmoduleEditor` trait 删除 `retire_submodule`
- [ ] `commands/code.rs`: 删除 `retire_submodule` 实现及相关测试
- [ ] `main.rs`: 删除 `CodeAction::Retire` 变体及 match 分支

## P0: 修复 `code sync` 实现缺陷

- [ ] 提交作者从 git 配置读取（`user.name` / `user.email`），替换硬编码 `"kse" <kse@local>`
- [ ] sync 三个阶段原子化：任一阶段失败回滚已执行的操作，避免部分同步
- [ ] sync 前先 fetch，确保远端状态最新

## P0: 清理 `code status`

- [ ] 修复三次重复 scan：`run_code_status` 中 `RepoState::scan` → `editor.status` → `print_aggregate` 扫了三遍，改为一次 scan 复用
- [ ] `--offline` 标志实际生效：`RepoState::scan` 支持跳过 fetch

## P0: 更新 AGENTS.md 的 CLI 设计规则

- [ ] `code` 命令列表只保留 `sync` 和 `status`
- [ ] 同步 `src/cli/AGENTS.md` 与子模组 `AGENTS.md` 中的 CLI 文档

## 验证

- [ ] `cargo build` 通过
- [ ] `cargo test` 全部通过
