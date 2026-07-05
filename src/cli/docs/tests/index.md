# 测试套件设计

## 测试金字塔

```
     ╱╲
    ╱  ╲           单元测试（src/ 内嵌 #[cfg(test)]）
   ╱    ╲          模块集成测试（tests/ 各文件）
  ╱──────╲
 ╱          ╲      CLI 端到端测试（tests/cli.rs）
╱──────────────╲   mock 编排测试（tests/mock.rs）
```

## 测试文件对应

| 测试文件 | 被测试模块 | 类型 | 数量 |
|---------|-----------|------|------|
| `tests/cli.rs` | 所有 CLI 子命令 | 端到端 | ~25 |
| `tests/code.rs` | `git` + `code` | 集成 | ~11 |
| `tests/mock.rs` | `build` + `test` + `release` + `plan` | mock 编排 | ~31 |
| `tests/release.rs` | `release` | 集成 | ~16 |
| `src/lib.rs` (内嵌) | 各模块纯函数 | 单元 | ~336 |
| `src/main.rs` (内嵌) | CLI 参数解析 | 单元 | ~2 |

## tests/cli.rs — CLI 端到端测试

用 `Command::new(env!("CARGO_BIN_EXE_qtcloud-devops"))` 调用编译好的二进制，测试完整 CLI 流程。

**测试范围：**
- `--help` / `--version` 输出
- 各子命令 `--help`（code / release / contract / plan / test / doctor / build）
- `release publish -v vX.Y.Z -y` 在临时 git 仓库中的完整流程
- `release publish` 自动生成 CHANGELOG
- `release publish` gh 未安装时的降级
- `contract status` / `doctor status` / `build status` / `release status` / `plan status` / `test status`
- `plan clean` 条目清理
- `code status` / `code sync`
- `test run` 在临时 Cargo 项目中

## tests/code.rs — code 模块集成测试

创建真实 git 仓库 + 子模块，测试 `git::RepoState::scan()` 和 `GitSubmoduleEditor` 操作。

**测试范围：**
- 扫描含子模块的仓库
- 扫描无 `.gitmodules` 的仓库 → 报错
- `sync_to_parent` / `sync_all_to_parent` 成功/失败场景
- `status` 返回 clean / dirty
- offline 模式

## tests/mock.rs — mock 编排测试

用 PATH mock 替代外部命令（gh / cargo / python / coverage），验证编排逻辑。

**测试范围：**
- `build::status` CI 状态查询（成功/失败/取消/unknown）
- `build::status` cargo check 成功/失败
- `test::run` Rust（cargo llvm-cov 成功/失败）
- `test::run` Python（pytest + coverage 分离）
- `test::run` 空目录 / 未知语言
- `test::run` 多 scope
- `test::run` cwd scope 过滤
- `test::run` cargo args 捕获
- `release::create_release`（gh 成功/已存在/其他错误）
- `release::status` 和 `plan::status` 带真实契约

**注意：** mock 测试修改全局 PATH，已通过全局 Mutex 串行化。

## tests/release.rs — release 模块集成测试

在临时 git 仓库中测试 release 操作的真实 git 行为。

**测试范围：**
- `release::status` 空仓库、有 tag、多 scope tag、dirty 工作区
- CHANGELOG 存在/缺失/未发布提交
- `release::publish` 版本校验、自动生成 CHANGELOG、幂等性
- `release::publish` v 前缀 CHANGELOG（不产生重复）
- `release::publish` extract_notes 过滤头部
- `release::publish` scoped monorepo（契约映射子目录）
- `release::create_tag` 在指定 repo 路径操作

## 单元测试（src/ 内嵌）

各模块内嵌的 `#[cfg(test)] mod tests`，测试纯函数逻辑：

| 模块 | 测试重点 |
|------|---------|
| `build.rs` | `resolve_workflow` 名称解析 |
| `code/model.rs` | SyncStatus label/clone/eq |
| `contract.rs` | 自动检测、版本状态、序列化 |
| `detect.rs` | tag 解析、版本构建、fallback heuristic |
| `git/scan.rs` | 子模块状态判定优先级 |
| `git/types.rs` | 类型方法 |
| `plan.rs` | 版本行解析、progress 计算、clean/doctor |
| `publish.rs` | 版本号更新（toml/json）、scope dir 解析 |
| `test.rs` | 测试输出解析、lcov/cobertura 解析、缓存读写 |
| `util.rs` | 版本校验、changelog 预检、GitHub URL 解析、tag 操作 |
