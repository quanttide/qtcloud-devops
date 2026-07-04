# Scope

Scope 是契约的上下文维度，将组件名称映射到 repo 子目录，挂载语言、构建工具、制品库等配置。

## 结构

```rust
pub struct Scope {
    pub name: String,            // scope 名称，对应 tag 前缀
    pub dir: String,             // 相对 repo 根的子目录
    pub language: Language,      // 编程语言
    pub framework: String,       // 框架名（如 actix、next.js）
    pub build_tool: BuildTool,   // 构建工具
    pub registry: Registry,      // 制品库
    pub release: StageRelease,   // 发布配置（覆盖全局 stages.release）
    pub test_threshold: Option<f64>,   // 测试阈值（覆盖全局 stages.test.threshold）
    pub ci_workflow: Option<String>,   // CI workflow 名
}
```

## 名称约定

scope 名称也是 git tag 前缀：

| scope | tag 示例 | 目录 |
|-------|---------|------|
| `cli` | `cli/v0.1.0` | `src/cli/` |
| `web` | `web/v0.1.0` | `src/web/` |
| 无 scope（root） | `v0.1.0` | repo 根 |

## YAML 配置

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
    ci_workflow: build-cli
```

### 字段说明

| 字段 | 必填 | 默认 | 说明 |
|------|------|------|------|
| `dir` | 是 | — | scope 目录，相对 repo 根 |
| `language` | 否 | 自动检测 | 声明时跳过文件检测 |
| `framework` | 否 | `""` | 框架名，暂仅供展示 |
| `build_tool` | 否 | 自动推断 | 按 language 映射 |
| `registry` | 否 | `None` | 自动推断时设为 `Crates` |
| `test_threshold` | 否 | `None` | 覆盖全局阈值 |
| `release` | 否 | 全局值 | 只覆盖显式指定的字段 |
| `ci_workflow` | 否 | `None` | 按 `build-{scope}` 约定推导 |

## 路径匹配

`find_scope_by_path` 使用最长前缀匹配：

```rust
pub fn find_scope_by_path(&self, current_dir: &Path) -> Option<&Scope>
```

| 当前目录 | 匹配 scope | 规则 |
|---------|-----------|------|
| `src/cli/sub` | `cli`（dir: `src/cli`） | 最长前缀 |
| `src/web` | `web`（dir: `src/web`） | 精确匹配 |
| `unknown/any` | `root`（dir: `.`） | 兜底 |

## scope 与发布

- `release publish -v cli/v0.2.0` → scope `cli` → 操作 `src/cli/` 下的配置文件
- `release publish -v v0.2.0` → root scope → 操作 repo 根目录

scope 的 `release.changelog` 和全局 `stages.release.changelog` 的区别：

- scope 级配置指向 scope 目录下的文件
- 全局配置指向 repo 根目录下的文件
