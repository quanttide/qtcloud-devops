# source status 命令设计

## 定位

检查系统依赖的外部命令是否可用。在运行任何操作前快速诊断环境。

## 命令

```
qtcloud-devops source status   检查系统依赖
```

## 检查项

对每个 scope 检测到的语言，检查其构建/测试工具链是否可用：

| 工具 | 检查命令 | 对应语言 |
|------|---------|---------|
| `git` | `git --version` | 始终检查 |
| `gh` | `gh --version` | 始终检查 |
| `cargo` | `cargo --version` | Rust |
| `uv` | `uv --version` | Python |
| `go` | `go version` | Go |
| `flutter` | `flutter --version` | Dart |
| `node` | `node --version` | TypeScript |
| `npm` | `npm --version` | TypeScript |
| `maturin` | `maturin --version` | Python（build） |

## 输出示例

```
系统诊断
──────────────────────────────────────────────────
  git          ✅ git version 2.43.0
  gh           ✅ gh version 2.55.0
  cargo        ✅ cargo 1.81.0
  uv           ⚠ 未安装
  go           未检测（当前项目不包含 Go）
```

- ✅ 已安装
- ⚠ 未安装（影响相关语言操作）
- 未检测（项目无需该语言，不检查）

## 公共 API

```rust
pub fn status(repo_path: &Path)
pub fn status_to(writer: &mut impl Write, repo_path: &Path) -> io::Result<()>
```
