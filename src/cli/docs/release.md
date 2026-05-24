# 发布管理 `[BREAKING v0.3.0]`

## 状态机

```
Staged → Published → Retired
   ↓
Cancelled → (可重新 Staged)
```

| 状态 | 含义 |
|------|------|
| Staged | 版本已标记，准备发布 |
| Published | 版本已正式上线（标签推送 + GitHub Release） |
| Cancelled | 发布被取消，可重新 Staged |
| Retired | 版本已退役，终态不可逆 |

## 命令

### stage

```bash
qtcloud-devops stage -v v1.0.0
```

版本号必须符合 `vX.Y.Z` 或 `pkg/vX.Y.Z` 格式。已发布或已退役的版本拒绝操作。已取消的版本会生成新 UUID 重新 Staged。已 Staged 的版本重复执行视为刷新（幂等）。

### publish

```bash
qtcloud-devops publish -v v1.0.0
qtcloud-devops publish -v v1.0.0 -y   # 跳过确认
```

仅允许 Staged → Published。执行标签创建、推送、GitHub Release（从 CHANGELOG.md 提取 Release Notes）。任一步骤失败自动回滚标签。

### cancel

```bash
qtcloud-devops cancel -v v1.0.0
```

仅允许 Staged → Cancelled。自动删除远程标签和 GitHub Release（若存在）。

### retire

```bash
qtcloud-devops retire -v v1.0.0
```

仅允许 Published → Retired。终态操作，退役后不可重新 stage。

## v0.2.x → v0.3.0 迁移

| 旧用法 | 新用法 |
|--------|--------|
| `qtcloud-devops release --version v0.1.0 -y` | `qtcloud-devops stage -v v0.1.0 && qtcloud-devops publish -v v0.1.0 -y` |
| `qtcloud-devops release --version v0.1.0 --tag-only` | `git tag v0.1.0 && git push origin v0.1.0` |
| `qtcloud-devops release --version v0.1.0 --release-only` | 无直接等价（publish 始终创建标签） |

## 预检查与回滚

回滚策略与 v0.2.x 一致：

| 失败点 | 行为 |
|--------|------|
| 创建标签失败 | 直接返回错误 |
| 推送标签失败 | 删除本地标签 |
| GitHub Release 失败 | 删除本地和远程标签 |
