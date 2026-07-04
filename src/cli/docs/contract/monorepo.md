# 默认单仓规范

## 布局约定

仓库按以下三个目录组织 scope：

```
<repo-root>/
├── src/*       应用模块 — 按功能拆分，随主应用发版
├── packages/*  库包     — workspace 多包，独立版本发布
└── apps/*      独立应用 — 可独立部署的服务，独立发版部署
```

scope 名称即子目录名，tag 前缀即 `{scope}/vX.Y.Z`。

### 示例

```
<repo-root>/
├── src/
│   ├── cli/        → scope "cli"      tag: cli/v0.1.0
│   └── web/        → scope "web"      tag: web/v0.1.0
├── packages/
│   ├── core/       → scope "core"     tag: core/v0.1.0
│   └── utils/      → scope "utils"    tag: utils/v0.1.0
└── apps/
    ├── admin/      → scope "admin"    tag: admin/v0.1.0
    └── api/        → scope "api"      tag: api/v0.1.0
```

## 决定条件

一个子目录是否能成为 scope，取决于它是否包含项目配置文件：

| 文件 | 语言 |
|------|------|
| `Cargo.toml` | Rust |
| `pyproject.toml` / `requirements.txt` | Python |
| `go.mod` | Go |
| `pubspec.yaml` | Dart/Flutter |
| `package.json` | TypeScript/Node |

普通目录（无配置文件）不会被识别为 scope。

## 作用

| 机制 | 无 scope 时 | 有 scope 时 |
|------|------------|------------|
| tag | `v0.1.0` | `cli/v0.1.0` |
| 版本号读取 | 根目录 `Cargo.toml` | `src/cli/Cargo.toml` |
| CHANGELOG | 根目录 `CHANGELOG.md` | `src/cli/CHANGELOG.md` |
| CI workflow | `build` | `build-cli` |
| 发版影响范围 | 整个仓库 | 仅 scope 目录 |
