/// 应用状态（参考 qtcloud-secret 的 AppState 模式——状态驱动页面）。
library;

import 'package:flutter/foundation.dart';

class AppState extends ChangeNotifier {
  bool _scanning = false;
  String? _lastScanSummary;

  bool get scanning => _scanning;
  String? get lastScanSummary => _lastScanSummary;

  /// 扫描子模块状态（占位：真实扫描由 qtcloud-devops CLI 提供）
  Future<void> scan() async {
    _scanning = true;
    notifyListeners();
    await Future<void>.delayed(const Duration(milliseconds: 800));
    _scanning = false;
    _lastScanSummary = '扫描完成（占位）——子模块状态待接入 CLI';
    notifyListeners();
  }
}
