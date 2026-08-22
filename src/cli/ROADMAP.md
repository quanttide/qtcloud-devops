# ROADMAP

## [0.13.0] 

### Added

- [ ] 新增 `deploy` 命令集（生命周期新增一环：build → test → release → deploy），与 `release` 同级
- [ ] 新增 `deploy init`：一键为新仓库生成/升级部署能力（`--domain/--kind/--stack/--bucket`），就地写入 `.github/workflows/deploy-*.yml` + `manifests/terraform/*`
- [ ] `deploy init` 支持幂等：已存在则合并/提示，`--force` 全量覆盖；`--dry-run` 只预览；`--apply` 生成后执行 terraform apply
- [ ] `deploy init` 按模板映射表推导：kind/stack → `build_dir`/`oss_bucket`/`cdn_domain`/`cdn_domains`/DNS rr/缓存策略
- [ ] `deploy init` 内置已知坑处理：SPA 回退改写（`back_to_origin_url_rewrite`）、缓存策略分离（assets 长缓存 + index.html no-cache）、私有 OSS 回源鉴权（`l2_oss_key private_oss_auth=on` + RAM 角色/策略）、SSL 证书占位提示、ICP 备案提示
- [ ] 新增 `deploy status`：检查当前仓库部署就绪度（workflow / terraform / org secrets / CDN / DNS CNAME / 证书 / SPA 回退）
- [ ] 新增 `deploy audit`：对照平台标准模板检测现有 deploy workflow 的漂移，输出差异清单
- [ ] 新增 `deploy apply`（可选）：把 build+upload+refresh 抽成 CLI 可调用，支持 `--dry-run`
- [ ] 文档：README 新增 `deploy` 部分，说明新仓库获得部署能力的标准流程；CLI `docs/deploy/init.md`


## [0.11.1] 

### Added

- [ ] 新增 `source::manifest` 模块：封装 `Cargo.toml` 解析和 `cargo metadata` 输出（`Manifest::from_path`、`Workspace::from_metadata`）
- [ ] 新增 `persist` 模块：追加式 JSON 行文件存储，用于审计快照持久化
- [ ] contract 增加查询方法：`scopes_by_language()`、`shared_dependencies()`、`version_consistency()`
- [ ] `release audit` 增加包元数据完整性检查：按语言检测必需的包描述字段（Rust: description/license/repository, Python: description/readme, Dart: description/homepage）

### Changed

- [ ] `code/status.rs` 输出改为 `status_to(writer)` 模式，将 `print_report` 从 `main.rs` 移入 `code/status.rs`
- [ ] `code/` 错误类型统一为 `CodeError`（thiserror），替换 `Box<dyn Error>` 和裸 `String`
- [ ] Scope 迭代逻辑抽取为共享 `ScopeIter`，替换 `build/`、`test/`、`release/`、`code/` 中 6 处重复遍历
- [ ] `code audit` 完成后写入审计快照到持久化存储，供 `health` section 消费
- [ ] `code status` 支持 `PartialResult` 模式：每个 section 独立可用，部分失败不影响其他 section

### Fixed

- [ ] `code status` 在非 git 目录或空 repo 时给出友好提示而非 panic


## [0.11.2]

### Added

- [ ] `code audit` 新增死 import 检测（聚合 `cargo check` 的 `unused_imports` 警告）
- [ ] `code audit` 新增注释掉的代码块检测（超过 5 行被 `//` 或 `/* */` 包裹的块）
- [ ] `code audit` 新增 `todo!()` / `panic!()` / `unreachable!()` 在非测试文件中的检测
- [ ] `code audit --scope <name>` 参数，支持只审计单个 scope
- [ ] `build audit` 新增 `--summary` 参数，按 scope 聚合 `cargo check` 警告


## [0.12.0]

### Added

- [ ] `code status` 新增 `DepsSection`：解析 `cargo metadata` 构建模块依赖图，检测循环依赖和跨 scope 引用
- [ ] `code status` 新增 `ConsistencySection`：跨 scope 对比 Rust 版本、公共依赖版本、CI 模板一致性
- [ ] `code status` 新增 `--section` 参数（`deps` / `consistency` / `sync`），支持按 section 过滤
- [ ] `code status` 新增 `-v`（详细模式）和 `--json` 输出格式

### Changed

- [ ] `code status` 从仅子模块同步检查升级为"代码架构健康状态"视图（4 section 聚合）
