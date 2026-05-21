# 命令参考

## release

发布 Git Release。

### 用法

```bash
qtcloud-devops release --version <version> [options]
```

### 必填参数

#### `--version, -V`

版本号，必须符合 `vX.Y.Z` 或 `scope/vX.Y.Z` 格式。

```
v0.1.0
v0.1.0-alpha.1
cli/v0.1.0
python/v0.2.0
```

### 可选参数

#### `--changelog`

CHANGELOG 文件路径，默认 `CHANGELOG.md`。

```bash
qtcloud-devops release --version v0.1.0 --changelog docs/CHANGELOG.md
```

#### `--dry-run`

仅执行预检查，不执行任何写入操作。

```bash
qtcloud-devops release --version v0.1.0 --dry-run

# 输出示例：
# 预检查失败:
#   ✗ 版本号格式错误: v0.1
# 
# 成功时输出 Release Notes 预览
```

#### `--tag-only`

仅创建并推送 Git 标签，跳过 GitHub Release。

```bash
# 先发标签
qtcloud-devops release --version v0.1.0 --tag-only

# 稍后补发 GitHub Release
qtcloud-devops release --version v0.1.0 --release-only
```

#### `--release-only`

仅为已有标签创建 GitHub Release，跳过标签创建。预检查会验证标签确实存在。

```bash
# 标签已通过其他方式创建
git tag v0.1.0 && git push origin v0.1.0

# 仅补 GitHub Release
qtcloud-devops release --version v0.1.0 --release-only
```

`--tag-only` 与 `--release-only` 互斥，同时使用会报错。

#### `--yes, -y`

跳过交互式确认，直接发布。

```bash
qtcloud-devops release --version v0.1.0 -y
```

### 工作流示例

#### 场景一：常规发布

```bash
# 1. 更新 CHANGELOG.md

# 2. 提交
git add CHANGELOG.md && git commit -m "chore: prepare CHANGELOG for v0.1.0"

# 3. 发布（默认 = 标签 + GitHub Release）
qtcloud-devops release --version v0.1.0
```

#### 场景二：Tag 先发，Release 后补

```bash
# 第一步：仅打标签
qtcloud-devops release --version v0.1.0 --tag-only

# 第二步：CI 通过后补 Release
qtcloud-devops release --version v0.1.0 --release-only
```

#### 场景三：子模块发布

```bash
# 进入子模块目录
cd apps/qtcloud-devops/src/cli

# 发布（自动使用子模块的 remote）
qtcloud-devops release --version cli/v0.2.0
```

#### 场景四：仅检查

```bash
qtcloud-devops release --version v0.1.0 --dry-run
```

### 预检查项

执行 release 前自动检查以下内容：

| 检查项 | 失败条件 |
|--------|---------|
| 版本号格式 | 不符合 `vX.Y.Z` 或 `scope/vX.Y.Z` |
| CHANGELOG | 文件不存在或不含目标版本 |
| 标签状态 | `--release-only` 时标签必须存在，否则不检查 |
| 工作区 | 有未提交的变更 |
| 分支 | 不在 `main` / `master` / `release/*` 上 |

### 回滚

发布过程自动处理回滚：

| 失败点 | 自动回滚 |
|--------|---------|
| 创建标签失败 | 无操作（未产生副作用） |
| 推送标签失败 | 删除本地标签 |
| GitHub Release 失败 | 删除本地和远程标签 |

```bash
# 也可手动回滚
git tag -d v0.1.0
git push origin --delete v0.1.0
gh release delete v0.1.0 --repo quanttide/repo --yes
```

### 退出码

| 退出码 | 含义 |
|-------|------|
| 0 | 发布成功或用户取消 |
| 1 | 预检查失败或执行失败 |
