# 发布管理

所有发布命令在 `release` 子命令组下。

## 状态机

```
Staged → Published → Retired
```

| 状态 | 含义 |
|------|------|
| Staged | 版本已标记（预发布），等待 CI 验证 |
| Published | 版本已正式上线（标签推送 + GitHub Release） |
| Retired | 版本已退役，终态不可逆 |

## 命令

### release stage — 标记预发布版本

```bash
qtcloud-devops release stage -v cli/v0.4.1-rc.1
```

仅用于预发布版本（含 `-rc.N`、`-alpha.N` 等后缀）。推送 tag + 创建 GitHub Release，触发 CI。

### release publish — 正式发布

```bash
qtcloud-devops release publish -v cli/v0.4.1
qtcloud-devops release publish -v cli/v0.4.1 -y   # 跳过确认
```

创建正式 tag + GitHub Release。正式版不需要先执行 stage。

### release retire — 退役版本

```bash
qtcloud-devops release retire -v v1.0.0
```

仅允许 Published → Retired。终态操作，退役后不可逆。

### release status — 查看发布状态

```bash
qtcloud-devops release status
```

从 journal 读取发布记录，输出当前版本状态摘要。

## 推荐工作流

```bash
# 1. 标记预发布，触发 CI
qtcloud-devops release stage -v cli/v0.4.1-rc.1

# 2. CI 验证通过后，正式发布
qtcloud-devops release publish -v cli/v0.4.1 -y

# 预发布失败时：递增序号重新 stage
qtcloud-devops release stage -v cli/v0.4.1-rc.2
```

## 回滚

| 失败点 | 行为 |
|--------|------|
| 创建标签失败 | 直接返回错误（幂等，已存在时跳过） |
| 推送标签失败 | 删除本地标签 |
| GitHub Release 失败 | 删除本地和远程标签（幂等，已存在时跳过） |
