# ROADMAP

## P0 — status fetch 修错了仓库

`code status` 中加了 fetch，但 fetch 的是主仓库。`RepoState::scan()` 读取的是子模块的 remote ref，fetch 进主仓库没有用，子模块的 remote_head 仍然是本地缓存。

- [x] 修复：在 `RepoState::scan()` 中对每个子模块执行 fetch
- 涉及：`src/commands/code.rs` `status()` / `src/model/code.rs` `RepoState::scan()`
- 来源：`docs/journal/platform/2026-05-25.md` 记载的 manual sync 场景

## P3 — pub.dev 发布集成

- [ ] `publish --registry pub-dev`
