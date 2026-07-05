# plan 命令设计

## 定位

管理 ROADMAP.md 规划文件：查看进度、清理已完成条目、修复格式问题。

## 命令

```
qtcloud-devops plan status [scope]    查看 scope 规划进度
qtcloud-devops plan clean [scope]     删除已完成条目
qtcloud-devops plan doctor [scope]    修复格式问题
```

## 公共数据模型

```rust
/// 单个版本的规划进度
pub struct VersionProgress {
    pub version: String,
    pub done: usize,    // [x] 条目数
    pub total: usize,   // [ ] + [x] 条目数
}

/// 格式问题
pub struct Issue {
    pub line: usize,
    pub scope: String,
    pub message: String,
}
```

## plan status — 查看规划进度

解析 ROADMAP.md，统计每个版本版本的完成度。

### ROADMAP.md 格式约定

```markdown
## [0.1.0]

### Added
- [ ] 功能 A
- [x] 功能 B

### Fixed
- [x] Bug 修复
```

- 版本标题：`## [X.Y.Z]` 格式
- 任务项：`- [ ]` 待办、`- [x]` 已完成
- 分类标题：`### Added`、`### Fixed`、`### Changed` 等

### 输出示例

```
规划状态 — 当前目录
────────────────────────────────────────
  [cli] 版本规划
    0.1.0  2/3  ✅
    0.2.0  0/1  ⚠
```

### scope 参数

- 省略时自动检测当前目录所属 scope
- 指定 scope 名称时查看指定 scope
- 特殊值 `(root)` 查看根 scope

## plan clean — 删除已完成条目

删除 ROADMAP.md 中所有 `- [x]` 行。如果某版本条目全为空，也删除该版本头和分类头。

### 行为

1. 删除 `- [x] xxx` 行
2. 分类下无条目时删除分类头（`### xxx`）
3. 版本下无分类时删除版本头（`## [X.Y.Z]`）
4. 自动 git add + git commit

### 输出示例

```
  ✓ 已清理 42 字节，文件: ROADMAP.md
  ✓ 已提交
```

## plan doctor — 修复格式问题

验证 ROADMAP.md 的格式正确性，目前为只读检测（LLM 修复未接入）。

### 校验规则

| 规则 | 说明 |
|------|------|
| 版本号格式 | `## [X.Y.Z]` 必须合法 |
| 分类命名 | `###` 后必须为 Added / Fixed / Changed / Removed / Breaking 之一 |
| 条目格式 | 必须为 `- [ ]` 或 `- [x]` 开头 |
| 空版本 | 无条目的版本会被标记 |
| 重复版本 | 同一版本出现多次会被标记 |

### 输出示例

```
  ⚠ L5: 分类名 '### Fixd' 拼写错误，应为 '### Fixed'
  ⚠ L10: 条目格式错误，缺少 '- [ ]' 或 '- [x]'
  ✅ 格式无误
```

## 公共 API

```rust
pub fn resolve_roadmap_path(repo_path: &Path, scope: Option<&str>) -> PathBuf
pub fn is_version_line(line: &str) -> Option<String>
pub fn parse_roadmap(path: &Path) -> Result<Vec<VersionProgress>, String>
pub fn clean_roadmap(path: &Path) -> Result<usize, String>
pub fn doctor_roadmap(path: &Path, scope_label: &str) -> Result<Vec<Issue>, String>
pub fn print_status(repo_path: &Path, scope: Option<&str>) -> Result<(), String>
```
