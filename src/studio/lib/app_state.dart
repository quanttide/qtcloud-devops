/// 应用状态（参考 qtcloud-secret 的 AppState 模式——状态驱动页面）。
///
/// 扫描流程：点击「扫描」→ `CliClient.scan()` 调用 qtcloud-devops CLI
/// 的 `code status` → 解析出子模块状态列表与摘要。
/// CLI 不可用（web 平台、未安装且无预构建产物）时记录错误，保留占位。
library;

import 'package:flutter/foundation.dart';

import 'api/cli_client.dart';

class AppState extends ChangeNotifier {
  AppState({CliClient? cliClient}) : _cliClient = cliClient ?? createCliClient();

  final CliClient _cliClient;

  bool _scanning = false;
  String? _lastScanSummary;
  String? _lastScanError;
  List<ScanComponent> _components = const [];

  bool get scanning => _scanning;
  String? get lastScanSummary => _lastScanSummary;
  String? get lastScanError => _lastScanError;
  List<ScanComponent> get components => _components;

  /// 扫描子模块状态：调用 qtcloud-devops CLI 并展示状态列表。
  ///
  /// CLI 不可用时不抛异常，而是记录 [lastScanError] 保留占位提示。
  Future<void> scan() async {
    _scanning = true;
    _lastScanError = null;
    notifyListeners();
    try {
      final report = await _cliClient.scan();
      _components = report.components;
      _lastScanSummary = report.pending == 0
          ? '扫描完成：${report.total} 个组件全部已同步'
          : '扫描完成：${report.total} 个组件'
              '（${report.synced} 已同步 / ${report.pending} 待处理）';
    } catch (e) {
      _components = const [];
      _lastScanSummary = null;
      _lastScanError = '无法调用 qtcloud-devops CLI：$e';
    } finally {
      _scanning = false;
      notifyListeners();
    }
  }
}
