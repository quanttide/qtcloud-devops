# 发布管理

## 状态机

```
Staged → Published → Retired
   ↓
Cancelled → (可重新 Staged，已废弃)
```

| 状态 | 含义 |
|------|------|
| Staged | 版本已标记（rc），等待 CI 验证 |
| Published | 版本已正式上线（标签推送 + GitHub Release） |
| Cancelled | 发布被取消（已废弃） |
| Retired | 版本已退役，终态不可逆 |

## 命令

### stage — 标记预发布版本

```bash
qtcloud-devops stage -v cli/v0.3.2-rc.1
```

仅用于 rc 版本（版本号含 `-rc.N` 后缀）。推送 tag，标记版本进入 Staged 状态等待 CI 验证。

版本号必须符合 `vX.Y.Z-rc.N` 或 `scope/vX.Y.Z-rc.N` 格式。

### publish — 正式发布

```bash
qtcloud-devops publish -v cli/v0.3.2
qtcloud-devops publish -v cli/v0.3.2 -y   # 跳过确认
```

创建正式 tag + GitHub Release。仅允许 Published → Retired 转换的终态操作之前使用。

### retire — 退役版本

```bash
qtcloud-devops retire -v v1.0.0
```

仅允许 Published → Retired。终态操作，退役后不可逆。

### release status — 查看发布状态

```bash
qtcloud-devops release status
```

从 journal 读取发布记录，输出当前版本状态摘要。

### cancel — 废弃（v0.3.x）

```bash
qtcloud-devops cancel -v cli/v0.3.2-rc.1
```

**已废弃**。rc 失败时直接递增序号即可（`stage -v cli/v0.3.2-rc.2`），不需要 cancel。v0.4.0 将移除。

## 推荐工作流

```bash
# 1. 标记 rc，触发 CI
stage -v cli/v0.3.2-rc.1

# 2. CI 验证通过后，正式发布
publish -v cli/v0.3.2 -y

# rc 失败时：递增序号重新 stage
stage -v cli/v0.3.2-rc.2
```

## v0.2.x → v0.3.0 迁移

| 旧用法 | 新用法 |
|--------|--------|
| `qtcloud-devops release --version v0.1.0 -y` | `qtcloud-devops stage -v cli/v0.1.0-rc.1 && qtcloud-devops publish -v cli/v0.1.0 -y` |

## 回滚

| 失败点 | 行为 |
|--------|------|
| 创建标签失败 | 直接返回错误 |
| 推送标签失败 | 删除本地标签 |
| GitHub Release 失败 | 删除本地和远程标签 |
