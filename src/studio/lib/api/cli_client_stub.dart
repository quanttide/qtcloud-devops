/// Web 等无法启动进程的平台：CLI 扫描不可用，抛 [CliException]
/// 由 UI 保留占位提示（不崩溃）。
library;

import 'cli_client.dart';

CliClient createCliClient() => const _UnavailableCliClient();

class _UnavailableCliClient implements CliClient {
  const _UnavailableCliClient();

  @override
  Future<ScanReport> scan() async {
    throw const CliException('CLI 扫描仅支持桌面端（Linux/macOS/Windows），当前平台不可用');
  }
}
