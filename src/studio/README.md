# studio

量潮 DevOps 云客户端（Flutter），qtcloud-devops 的可视化前端。

## 定位

CLI 的**可视化视图层**：把 `qtcloud-devops code status` 的子模块同步状态
变成人可用的界面。不做业务逻辑，全部能力来自 CLI（或经 provider 转发的 CLI）。

```
桌面端（Linux/macOS/Windows）  直连本地 CLI（cli_client_io.dart）
Web 端                         经 provider HTTP API（cli_client_http.dart）
```

- 桌面端：调用 `qtcloud-devops code status <root> --offline`，候选顺序
  PATH → 仓库内预构建 → `cargo run`；单次调用 60s 超时。
- Web 端：调用 provider `GET /api/scan`；provider 地址用编译期常量注入：
  `flutter build web --dart-define=QDEV_OPS_BASE_URL=http://host:8080`

## 开发

```bash
flutter pub get
flutter analyze
flutter test
```

## 目录

```
lib/
  main.dart            入口（状态驱动，无导航栈）
  app_state.dart       AppState：扫描流程 + 占位降级
  api/                 CLI 客户端（模型/解析/探测 + 平台实现）
    cli_client.dart       纯 Dart：模型、文本输出解析、扫描目标探测
    cli_client_io.dart    桌面端：进程调用（超时 + 候选回退）
    cli_client_http.dart  Web 端：provider HTTP API
  ui/                  ScanPage：子模块状态列表
```
