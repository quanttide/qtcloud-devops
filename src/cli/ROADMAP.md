# ROADMAP

- [ ] `source/tag.rs:parse_version` + `build_version`（60 行）→ 等待 toolkit 提供 `increment_version`（[issue #6](https://github.com/quanttide/quanttide-devops-toolkit/issues/6)）
- [ ] `release/detect.rs` 改用 `parse_semver_tag`（同上，等待 toolkit）
- [ ] 推广 `MockTagSource` 替代真实 git repo 测试
- [ ] `plan.rs:print_progress` 改用 `RoadmapVersion::percent()`
