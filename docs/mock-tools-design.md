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
5. 执行被测试函数（调的是真实 Command::new，但找到的是 mock）
6. 恢复 PATH
```

## 优缺点取舍

- ✅ 零代码侵入（不引入 trait、泛型、DI、feature gate）
- ✅ 测试执行真实的生产代码路径（`Command::new("gh")`）
- ❌ 串行执行（mock 覆盖全局 PATH，不能并行）
- ❌ 每场景一个脚本文件（boilerplate 但不复杂）

串行 + boilerplate 可接受：编排层测试本来就少（每条命令几个场景），CLI 本身不要求并行测试。

## 需要 mock 的外部命令

| 命令 | 所在模块 | 需要模拟的场景 |
|------|---------|---------------|
| `gh` | `build.rs` `release/status.rs` `release/util.rs` | 成功 / 失败 / 不存在 / 异常 JSON |
| `cargo` | `build.rs` | check 通过 / check 失败 / 未安装 |
| `git` | `build.rs` `release/status.rs` `release/util.rs` `release/changelog.rs` | status / tag / log / remote / init 各子命令 |
| `gh release` | `release/util.rs` | create 成功 / 已存在 / 失败 |
| `gh run` | `build.rs` | list 成功 / 空 / 不存在 |

## 测试文件组织

```
tests/
├── cli.rs           ← 端到端（已有）
├── code.rs          ← code 模块集成（已有）
├── release.rs       ← release 模块集成（已有）
└── mock/            ← mock 测试（新增）
    ├── mod.rs       ← with_mock_path 等共享工具
    ├── build.rs     ← build status 各场景
    ├── test.rs      ← test status 各场景
    └── release.rs   ← release publish/status 各场景
```

## mock 工具函数

在 `tests/mock/mod.rs` 中提供：

```rust
/// 创建一个返回固定 stdout 的 mock 脚本。
fn mock_script(stdout: &str, stderr: &str, exit_code: i32) -> String

/// 创建一个"command not found"的 mock（模拟命令未安装）。
fn mock_not_found() -> String

/// 在 mock 环境中运行闭包。
/// 将 mock 脚本写入 temp dir 的 bin/，前置到 PATH，执行 f，恢复 PATH。
fn with_mock_env(scripts: &[(&str, &str)], f: impl FnOnce(&Path))

/// 为测试创建 mock git repo（简化 git_init + git_commit 的重复代码）。
fn setup_git_repo() -> TempDir
```
