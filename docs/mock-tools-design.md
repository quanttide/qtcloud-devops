# 编排层测试：PATH mock 工具设计

## 问题

`build status` / `test status` / `release status` 的 I/O 编排路径（gh 未安装、cargo check 失败、git remote 不存在等完整行为）不在可控环境下测试。当前仅纯函数有单元测试，编排层的分支完全依赖开发环境偶然满足条件。

## 方案：PATH mock

延续 `release/status.rs` 的 `with_mock_path` 已有模式：

```
1. temp dir 建 bin/ 目录
2. 写入 mock 脚本（sh script，echo 固定输出）
3. chmod +x
4. 前置到 PATH
5. 调用真实的库函数（Command::new("gh") 找到的是 mock）
6. 恢复 PATH
```

## 优缺点

- ✅ 零代码侵入（不引入 trait、泛型、DI、feature gate）
- ✅ 测试执行真实的生产代码路径
- ❌ 串行执行（mock 覆盖全局 PATH）
- ❌ 每场景一个脚本文件

串行可接受：编排层测试数量少，CLI 不要求并行测试。

## 实现

文件：`tests/mock.rs`（单文件，Cargo 自动发现为集成测试）

```rust
/// 创建返回固定输出的 shell 脚本。
fn mock_script(stdout: &str, stderr: &str, exit_code: i32) -> String

/// 模拟"命令未安装"（exit 127）。
fn mock_not_found() -> String

/// 自定义 mock 脚本。
fn mock_custom(body: &str) -> String

/// 设置 mock PATH 并运行闭包。
fn with_mock_env<F: FnOnce() -> R, R>(scripts: &[(&str, &str)], f: F) -> R

/// 创建 mock git repo（git init + commit）。
fn setup_repo() -> (TempDir, PathBuf)

/// 创建 mock git repo + contract.yaml。
fn setup_repo_with_contract() -> (TempDir, PathBuf)
```

## 已覆盖场景（15 个测试）

### build status（9 个）

| mock 命令 | 场景 | 验证 |
|-----------|------|------|
| gh | 未安装 | 不 panic |
| gh | 空数组 | 不 panic |
| gh | success | 不 panic |
| gh | failure | 不 panic |
| gh | cancelled | 不 panic |
| gh | 未知结论 | 不 panic |
| cargo | check 通过 | 不 panic |
| cargo | check 失败 | 不 panic |
| — | 无 manifest | 不 panic（不调 cargo） |

### test status（3 个）

| mock 命令 | 场景 | 验证 |
|-----------|------|------|
| — | 空目录 | 不 panic |
| cargo | test 通过 | 不 panic |
| cargo | test 失败 | 不 panic |

### release status（3 个）

| 场景 | 验证 |
|------|------|
| 有标签（真实 git） | 不 panic |
| 无标签 | 不 panic |
| 非 git 目录 | 不 panic |

## 需要 mock 的外部命令

| 命令 | 所在模块 | 已模拟场景 |
|------|---------|-----------|
| `gh` | `build.rs` | ✅ 6 场景 |
| `cargo` | `build.rs` / `test.rs` | ✅ 4 场景 |
| `git` | `release/status.rs` | 使用真实 git（子命令复杂） |
