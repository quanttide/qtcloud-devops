# ROADMAP

## [0.11.1] 

### Added

- [ ] 新增 `source::manifest` 模块：封装 `Cargo.toml` 解析和 `cargo metadata` 输出（`Manifest::from_path`、`Workspace::from_metadata`）
- [ ] 新增 `persist` 模块：追加式 JSON 行文件存储，用于审计快照持久化
- [ ] contract 增加查询方法：`scopes_by_language()`、`shared_dependencies()`、`version_consistency()`

### Changed

- [ ] `code/status.rs` 输出改为 `status_to(writer)` 模式，将 `print_report` 从 `main.rs` 移入 `code/status.rs`
- [ ] `code/` 错误类型统一为 `CodeError`（thiserror），替换 `Box<dyn Error>` 和裸 `String`
- [ ] Scope 迭代逻辑抽取为共享 `ScopeIter`，替换 `build/`、`test/`、`release/`、`code/` 中 6 处重复遍历
- [ ] `code audit` 完成后写入审计快照到持久化存储，供 `health` section 消费
- [ ] `code status` 支持 `PartialResult` 模式：每个 section 独立可用，部分失败不影响其他 section

### Fixed

- [ ] `code status` 在非 git 目录或空 repo 时给出友好提示而非 panic


## [0.12.0]

### Added

- [ ] `code status` 新增 `DepsSection`：解析 `cargo metadata` 构建模块依赖图，检测循环依赖和跨 scope 引用
- [ ] `code status` 新增 `ConsistencySection`：跨 scope 对比 Rust 版本、公共依赖版本、CI 模板一致性
- [ ] `code status` 新增 `--section` 参数（`deps` / `consistency` / `sync`），支持按 section 过滤
- [ ] `code status` 新增 `-v`（详细模式）和 `--json` 输出格式

### Changed

- [ ] `code status` 从仅子模块同步检查升级为"代码架构健康状态"视图（4 section 聚合）
