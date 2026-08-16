import 'package:flutter/material.dart';

import 'app_state.dart';
import 'ui/scan_page.dart';

void main() {
  runApp(DevOpsApp(state: AppState()));
}

/// 量潮 DevOps 云客户端入口。
///
/// 设计参考 qtcloud-secret：页面由 AppState 状态驱动（不依赖导航栈）。
class DevOpsApp extends StatelessWidget {
  const DevOpsApp({super.key, required this.state});

  final AppState state;

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      title: '量潮 DevOps 云',
      debugShowCheckedModeBanner: false,
      theme: ThemeData(
        colorScheme: ColorScheme.fromSeed(seedColor: const Color(0xFF4F46E5)),
        useMaterial3: true,
      ),
      home: ScanPage(state: state),
    );
  }
}
