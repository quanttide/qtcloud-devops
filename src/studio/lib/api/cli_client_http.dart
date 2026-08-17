/// Web 端 CLI 客户端：通过 provider HTTP API 消费 CLI 能力。
///
/// Web 平台无法启动本地进程，改走 provider 的 HTTP 接口
/// （`GET /api/scan`，见 src/provider），由 provider 在其所在机器
/// 调用 CLI 并返回 JSON。端点契约对齐 provider `Report`：
/// `{root, total, synced, pending, components: [{name, status, ahead, behind}]}`。
///
/// provider 地址通过编译期常量 `QDEV_OPS_BASE_URL` 注入
/// （`flutter build web --dart-define=QDEV_OPS_BASE_URL=http://host:8080`），
/// 默认 `http://localhost:8080`。
library;

import 'dart:convert';

import 'package:http/http.dart' as http;

import 'cli_client.dart';

const _defaultBaseUrl = String.fromEnvironment(
  'QDEV_OPS_BASE_URL',
  defaultValue: 'http://localhost:8080',
);

CliClient createCliClient() => HttpCliClient(baseUrl: _defaultBaseUrl);

/// 经 provider HTTP API 扫描的客户端。
class HttpCliClient implements CliClient {
  HttpCliClient({required this.baseUrl, http.Client? client})
      : _client = client ?? http.Client();

  final String baseUrl;
  final http.Client _client;

  @override
  Future<ScanReport> scan() async {
    final http.Response response;
    try {
      response = await _client.get(Uri.parse('$baseUrl/api/scan'));
    } catch (e) {
      throw CliException('无法连接 provider（$baseUrl）：$e');
    }
    if (response.statusCode != 200) {
      throw CliException(
        'provider /api/scan 失败（HTTP ${response.statusCode}）：${response.body}',
      );
    }
    return _parseReportJson(response.body);
  }
}

/// 解析 provider 的 JSON 报告为 [ScanReport]。
ScanReport _parseReportJson(String body) {
  final Object? decoded;
  try {
    decoded = jsonDecode(body);
  } catch (e) {
    throw CliException('provider 响应不是合法 JSON：$e');
  }
  if (decoded is! Map<String, dynamic>) {
    throw CliException('provider 响应结构异常：${body.length > 200 ? body.substring(0, 200) : body}');
  }
  final components = (decoded['components'] as List<dynamic>? ?? const [])
      .whereType<Map<String, dynamic>>()
      .map(_parseComponentJson)
      .toList();
  return ScanReport(
    root: decoded['root'] as String? ?? '',
    total: (decoded['total'] as num?)?.toInt() ?? 0,
    synced: (decoded['synced'] as num?)?.toInt() ?? 0,
    pending: (decoded['pending'] as num?)?.toInt() ?? 0,
    components: components,
  );
}

ScanComponent _parseComponentJson(Map<String, dynamic> map) => ScanComponent(
      name: map['name'] as String? ?? '',
      status: _statusFromJson(map['status'] as String? ?? ''),
      ahead: (map['ahead'] as num?)?.toInt() ?? 0,
      behind: (map['behind'] as num?)?.toInt() ?? 0,
    );

ComponentSyncStatus _statusFromJson(String status) => switch (status) {
      'synced' => ComponentSyncStatus.synced,
      'pending_push' => ComponentSyncStatus.pendingPush,
      'pending_pull' => ComponentSyncStatus.pendingPull,
      'conflict' => ComponentSyncStatus.conflict,
      _ => ComponentSyncStatus.conflict,
    };
