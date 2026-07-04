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

## Monorepo 布局约定

扫描的三个目录是 monorepo 的默认 scope 布局规范：

| 目录 | 组件类型 | 说明 |
|------|---------|------|
| `src/*` | 应用模块 | 按功能拆分的应用模块，随主应用发版 |
| `packages/*` | 库包 | workspace 多包，独立版本发布 |
| `apps/*` | 独立服务 | 可独立部署的应用，独立发版部署 |

详见 `docs/essay/contract/index.md` 四维架构的 Scope 维度。

## 推断规则

扫描 `src/*`、`packages/*`、`apps/*` 下的一级子目录，检测项目配置文件：

| 标志文件 | 推断语言 | 推断构建工具 |
|----------|---------|------------|
| `Cargo.toml` | `Rust` | `Cargo` |
| `pyproject.toml` / `requirements.txt` | `Python` | `Uv` |
| `go.mod` | `Go` | `Go` |
| `pubspec.yaml` | `Dart` | `Flutter` |
| `package.json` | `TypeScript` | `Npm` |
| 无匹配 | 跳过（不生成 scope） | — |

根目录也按同样规则检测。检测到语言时生成 `(root)` scope（`dir: "."`），优先级最低——`find_scope_by_path` 按 dir 长度排序，长的先匹配。

## 三层过滤逻辑

代码内部按三层过滤判断一个子目录是否为 scope：

**第一层：目录过滤**

只在 `src/`、`packages/`、`apps/` 下的一级子目录里寻找。不在这些目录下的子目录不会被扫描。

```
src/ 下的 .rs 文件         → 不是目录，跳过
packages/lab/Cargo.toml    → 是一级子目录，进入第二层
src/cli/                   → 是一级子目录，进入第二层
src/cli/sub/deep.rs        → 不是一级子目录，跳过
```

**第二层：文件检测**

进入扫描范围的子目录，检查是否有可识别的项目配置文件。没有配置文件的普通目录就只是目录，不是 scope。

```
packages/lab/
├── Cargo.toml             → Rust → 生成 scope "lab"
└── src/ ...               → 不扫描嵌套内容

src/utils/
└── (只有 .rs 文件)        → 无 Cargo.toml → 跳过
```

**第三层：根目录兜底**

根目录能识别语言时，生成一个 `(root)` scope 覆盖整个仓库。`src/` 下没有 Cargo.toml 的源文件、`docs/`、`tests/` 等普通目录都归入 `(root)`。

```
repo 根
├── Cargo.toml             → 根目录可识别 → 生成 (root) scope
├── src/main.rs             → 属于 (root)
├── docs/                   → 属于 (root)
└── packages/lab/
    └── Cargo.toml          → 独立 scope "lab"
```

## 自动推断的默认值

| 维度 | 默认值 |
|------|--------|
| Build 命令 | `cargo build` |
| Test 命令 | `cargo test`（阈值 70%） |
| Release changelog | `CHANGELOG.md` |
| Platform | Github / GithubActions |
| Registry | `crates.io`（所有 scope） |
