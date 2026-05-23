# TODO

## 开发中

### Orphaned 状态拆分

- [ ] 分析 Orphaned 的细分场景（rebase force push、squash merge、仓库替换、gc 清理）
- [ ] 定义子状态枚举（新增 `SubmoduleStatus` 变体或补充字段）
- [ ] 更新 `RepoState::scan()` 判定逻辑，区分不同场景
- [ ] 更新 `describe_issue()` 为各子状态提供针对性建议
- [ ] 更新 `docs/code.md` 状态表和 Orphaned 说明
- [ ] 更新集成测试覆盖各子状态

---

## P0 — 发布目标支持

- [ ] PyPI 发布集成
  - [ ] 版本校验（与 PyPI 已发布版本比对）
  - [ ] 构建（`python -m build`）
  - [ ] 发布（`twine upload` 或 `maturin upload`）
  - [ ] 验证（安装后导入测试）
- [ ] pub.dev 发布集成
- [ ] 发布目标抽象模型

## P1 — 体验修复

- [ ] CHANGELOG 路径智能检测

## P2 — 配置扩展

- [ ] 放宽分支限制
- [ ] 支持非 semver 版本策略
- [ ] CI Action 版本升级
- [ ] GitLink 镜像容灾同步
