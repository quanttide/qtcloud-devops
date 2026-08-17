# provider

qtcloud-devops 的服务端：把 `src/cli` 的 scan 能力暴露为 HTTP API。

## 定位

CLI 能力的 **HTTP 适配层（网关）**，不做业务逻辑（无认证/存储/加密）。
结构分层参考 qtcloud-secret/src/provider（`cmd/server` + `internal`），
但职责不同：qtcloud-secret 的 provider 是业务服务端，本 provider 只是
把 CLI 远程化，服务「无法启动进程」的消费端（studio Web 浏览器、远程部署）。

```
studio 桌面端 ──直连──▶ src/cli（本地进程）
studio Web 端  ──HTTP──▶ provider ──▶ src/cli（provider 所在机器）
```

## 运行

```bash
cd src/provider
go run ./cmd/server
# 环境变量：
#   QDEV_OPS_ADDR              监听地址（默认 :8080）
#   QDEV_OPS_ROOT              扫描目标；为空自动探测聚合仓库根
#   QDEV_OPS_CLI_BIN           显式 CLI 路径；为空按候选顺序探测
#   QDEV_OPS_ALLOWED_ORIGINS   浏览器跨源白名单（逗号分隔）；空则不设 CORS
```

## API

| 端点 | 说明 |
|------|------|
| `GET /health` | 健康检查 |
| `GET /api/scan` | 运行 `qtcloud-devops code status <root> --offline`，返回子模块状态 JSON；CLI 不可用/调用失败返回 502 |

## 测试

```bash
go test ./...
```

CLI 调用受 60s 超时保护（`cliScanTimeout`）；候选按
PATH → 预构建二进制 → `cargo run` 顺序探测，文件存在才加入候选。
