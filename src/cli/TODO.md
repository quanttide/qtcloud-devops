# TODO

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
