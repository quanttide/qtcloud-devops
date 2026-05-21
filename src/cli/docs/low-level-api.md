# 底层 API 参考

## release.py

### `get_remote_repo() -> Optional[str]`

从 git remote 解析 GitHub 仓库的 `owner/name`。

```python
get_remote_repo() -> str | None
```

- 执行 `git remote get-url origin`
- 正则提取 `github.com/owner/name` 或 `github.com:owner/name`
- 失败时返回 `None`

---

### `precheck(version, changelog, release_only) -> list[str]`

发布前检查，返回错误列表。空列表表示通过。

```python
def precheck(
    version: str,
    changelog: Path,
    release_only: bool = False,
) -> list[str]
```

检查项：

| 检查 | 失败条件 |
|------|---------|
| 版本号格式 | 不匹配 `vX.Y.Z` 或 `scope/vX.Y.Z` |
| CHANGELOG | 文件不存在或不含目标版本 |
| Tag 状态 | `release_only=True` 时 tag 必须存在；否则不检查 |
| 工作区 | 有未提交变更 |
| 分支 | 当前不在 `main`/`master`/`release/*` |

---

### `extract_notes(version, changelog) -> Optional[str]`

从 CHANGELOG 提取目标版本的 Release Notes。

```python
def extract_notes(
    version: str,
    changelog: Path,
) -> str | None
```

- 匹配 `## [version]` 标题后的内容
- 到下一个 `## [version]` 标题结束
- 找不到返回 `None`

---

### `confirm_release(version, notes, yes) -> bool`

展示发布摘要并等待用户确认。

```python
def confirm_release(
    version: str,
    notes: str | None,
    yes: bool = False,
) -> bool
```

- `yes=True` 时跳过交互直接确认
- 非交互环境（EOFError/KeyboardInterrupt）返回 `False`

---

### `create_tag(version) -> bool`

创建 Git 标签。

```python
def create_tag(version: str) -> bool
```

- 执行 `git tag version`
- 成功返回 `True`，失败打印错误返回 `False`

---

### `push_tag(version) -> bool`

推送 Git 标签到远程。

```python
def push_tag(version: str) -> bool
```

- 执行 `git push origin version`
- 成功返回 `True`，失败打印错误返回 `False`

---

### `create_release(version, notes, repo) -> bool`

创建 GitHub Release。

```python
def create_release(
    version: str,
    notes: str,
    repo: str,
) -> bool
```

- 执行 `gh release create version --title version --notes notes --repo repo`
- 成功返回 `True`，失败打印错误返回 `False`

---

### `verify_release(version, repo) -> bool`

验证 GitHub Release 是否创建成功。当前未在 `run()` 中使用。

```python
def verify_release(
    version: str,
    repo: str,
) -> bool
```

- 执行 `gh release view version --repo repo`
- 成功打印 Release 信息并返回 `True`

---

### `rollback_tag(version) -> None`

删除本地和远程标签（回滚用）。

```python
def rollback_tag(version: str) -> None
```

- `git tag -d version`
- `git push origin --delete version`

---

### `run(version, changelog, dry_run, tag_only, release_only, yes) -> int`

发布流程编排，顶层入口。

```python
def run(
    version: str,
    changelog: Path | None = None,
    dry_run: bool = False,
    tag_only: bool = False,
    release_only: bool = False,
    yes: bool = False,
) -> int
```

返回码：

| 返回码 | 含义 |
|-------|------|
| 0 | 成功或取消发布 |
| 1 | 预检查失败或执行失败 |

执行流程：

1. `precheck()` — 失败返回 1
2. `extract_notes()` — 预览 Release Notes
3. `dry_run` — 返回 0
4. `confirm_release()` — 拒绝返回 0
5. 创建并推送 tag（除非 `release_only=True`）
6. 创建 GitHub Release（除非 `tag_only=True`，repo 从 `get_remote_repo()` 自动检测）

## 代码结构

```
app/
├── cli.py          Typer CLI 入口，定义命令和参数
├── config.py       pydantic-settings 配置
├── contract.py     契约加载与查询
└── release.py      发布 Release 核心逻辑
```

## cli.py

### CLI 参数

```
--version, -V    版本号（必填，如 v0.1.0）
--changelog      CHANGELOG.md 路径（默认 CHANGELOG.md）
--dry-run        仅检查不执行
--tag-only       仅创建 Git 标签，跳过 GitHub Release
--release-only   仅创建 GitHub Release，跳过 Git 标签
--yes, -y        跳过确认提示
```
