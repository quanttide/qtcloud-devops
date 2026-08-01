# Scope

## 要解决的问题

monorepo 里有多个组件（CLI、web、studio），每个组件：
- 独立发版，不同版本号
- 各自有 Cargo.toml、CHANGELOG.md
- 可能用不同语言和构建工具

没有 scope 时，所有 tag 平铺在根命名空间下，分不清 `v0.1.0` 是哪个组件的，`release status` 也没法按组件展示。

## 核心逻辑

**Scope = 命名边界。** 它把一个名字（`cli`）绑定到一个目录（`src/cli`），所有操作都在这个边界内执行。

```
                       tag 命名空间
                       ┌─────────────────────────┐
   scope "cli"  ───────┤  cli/v0.1.0              │
   dir: src/cli        │  cli/v0.2.0              │
                       ├─────────────────────────┤
   scope "web"  ───────┤  web/v0.1.0              │
   dir: src/web        │  web/v0.2.0              │
                       ├─────────────────────────┤
   (root)       ───────┤  v0.1.0                  │
   dir: .              │  v0.2.0                  │
                       └─────────────────────────┘
```

### 三件事

**1. tag 隔离** — tag 带 scope 前缀，不同 scope 的 tag 不会冲突

```
cli/v0.1.0     ← scope "cli" 的 tag
web/v0.1.0     ← scope "web" 的 tag，可以和 cli 同版本号
v0.1.0         ← root scope（无前缀）
```

**2. 目录绑定** — 所有文件操作定位到 scope 的目录

| 操作 | scope "cli" (dir: src/cli) | root scope (dir: .) |
|------|---------------------------|---------------------|
| 读版本号 | `src/cli/Cargo.toml` | `Cargo.toml` |
| 更新 CHANGELOG | `src/cli/CHANGELOG.md` | `CHANGELOG.md` |
| git tag | `cli/v0.2.0` | `v0.2.0` |

**3. 配置继承** — scope 有值覆盖全局，无值走默认

```
全局 stages.test.threshold = 70
         ↓
cli scope 声明 test_threshold: 90
         ↓
cli 的测试阈值 = 90（覆盖全局）
web 没有声明 → 70（走全局）
```

## 数据流

```
release publish -v cli/v0.2.0
         │
         ▼
  1. 从版本号提取 scope 名："cli"
         │
         ▼
  2. 从 contract.scopes 查找 name="cli"
         │
         ▼
  3. 取 scope.dir → "src/cli" → 拼出 scope_dir
         │
         ▼
  4. 所有文件读写 → scope_dir 下
     Cargo.toml  → repo_root/src/cli/Cargo.toml
     CHANGELOG   → repo_root/src/cli/CHANGELOG.md
         │
         ▼
  5. tag 创建 → git tag cli/v0.2.0
```

## 路径匹配

`find_scope_by_path` 用于"当前目录属于哪个 scope"的判断（`plan status` 省略 scope 参数时）：

```
scopes:
  cli:  dir: src/cli
  root: dir: .

当前在 src/cli/sub/     → 匹配 cli（最长前缀）
当前在 src/web/         → 匹配 root（兜底）
```

匹配规则：按 `dir` 长度排序，最长的优先。

## 结构定义

```rust
pub struct Scope {
    pub name: String,            // scope 名称 → tag 前缀
    pub dir: String,             // 相对 repo 根的子目录 → 文件定位
    pub language: Language,      // 语言（声明的跳过文件检测）
    pub framework: String,       // 框架名，展示用
    pub build_tool: BuildTool,   // 构建工具（按 language 推断）
    pub registry: Registry,      // 制品库
    pub release: StageRelease,   // 覆盖全局 stages.release
    pub test_threshold: Option<f64>,   // 覆盖全局 stages.test.threshold
    pub ci_workflow: Option<String>,   // CI workflow 名
}
```

## YAML 示例

```yaml
scopes:
  cli:
    dir: src/cli
    language: rust
    build_tool: cargo
    registry: crates
    test_threshold: 90
    release:
      changelog: CHANGELOG.md
    ci_workflow: release-cli
```
