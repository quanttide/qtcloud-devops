# 契约自动推断

无 `.quanttide/devops/contract.yaml` 时，按仓库结构自动生成契约。

## 加载流程

```
load(repo_path)
  │
  ├─ .quanttide/devops/contract.yaml 存在?
  │   ├─ Y → serde 解析 → Contract
  │   └─ N → auto_detect_contract(repo_path)
  │           ├─ 扫描 src/*/ packages/*/ apps/*/ 下项目配置文件
  │           │   有匹配 → 每个子目录生成一个 Scope
  │           │   无匹配 → scope 为空
  │           ├─ 根目录语言可识别? → 插入 (root) scope
  │           └─ 返回 Contract（stages/platform/scopes 均默认值）
  │
  └─ 返回 Contract
```

## 推断规则

扫描 `src/*`、`packages/*`、`apps/*` 下的一级子目录：

| 标志文件 | 推断语言 | 推断构建工具 |
|----------|---------|------------|
| `Cargo.toml` | `Rust` | `Cargo` |
| `pyproject.toml` / `requirements.txt` | `Python` | `Uv` |
| `go.mod` | `Go` | `Go` |
| `pubspec.yaml` | `Dart` | `Flutter` |
| `package.json` | `TypeScript` | `Npm` |
| 无匹配 | 跳过（不生成 scope） | — |

根目录也按同样规则检测。检测到语言时生成 `(root)` scope（`dir: "."`），优先级最低——`find_scope_by_path` 按 dir 长度排序，长的先匹配。

## 自动推断的默认值

| 维度 | 默认值 |
|------|--------|
| Build 命令 | `cargo build` |
| Test 命令 | `cargo test`（阈值 70%） |
| Release changelog | `CHANGELOG.md` |
| Platform | Github / GithubActions |
| Registry | `crates.io`（所有 scope） |
