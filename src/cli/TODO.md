# TODO — 重构计划

## ✅ 已完成

- `release/* → source/` 结构迁移（commit `a252104`）
- `release/detect/` 坍缩为单文件（`a252104`）
- `release/util/` 删除（`a252104`）
- `get_latest_tags_by_scope` 改用 toolkit `filter_latest_tag`（`0fddb51`）
- `status.rs` 测试样板压缩（`0fddb51`）
- `collect_all` 改用 `ReleaseState::new()`（`3acf66e`）
- `status_to` 改用 toolkit `Display`（`3acf66e`）
- `is_git_repo` 委托 toolkit（`3acf66e`）
- `detect.rs` 测试移除无意义 git init（`3acf66e`）

## 待处理

### P0 — 可立即动工

- [ ] `plan.rs:parse_roadmap_str`（70 行）替换为 `Roadmap::from_str`；测试输入补 `# ROADMAP` 头
- [ ] 运行 `qtcloud-code` 扫描，消除 MUST 问题

> P1 和 P2 已移至 [ROADMAP.md](./ROADMAP.md)

## 基线

- 当前 CLI 代码总量：~9700 行（`find src/cli/src -name '*.rs' | xargs wc -l`）
- 本 session 净删：-154 行（+31 / -185）
- 剩余可压缩：估 ~800 行（P0+P1+P2 合计）
