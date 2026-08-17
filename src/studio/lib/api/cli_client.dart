/// qtcloud-devops CLI 客户端：模型、输出解析与扫描目标探测（纯 Dart，无平台依赖）。
///
/// 分层对齐 qtcloud-secret/src/studio：`api/` 放外部工具客户端，
/// `ui/` 只消费 `AppState` 暴露的状态。
///
/// 桌面端（Linux/macOS/Windows）通过 `dart:io` 调用 CLI
/// （`qtcloud-devops code status <root> --offline`，见 cli_client_io.dart）；
/// Web 端无法启动进程，改走 provider 的 HTTP API（见 cli_client_http.dart）。
/// 具体平台由 `createCliClient()` 按 `dart.library.io` 条件导入选择。
library;

import 'cli_client_http.dart' if (dart.library.io) 'cli_client_io.dart' as impl;

/// 创建当前平台的 CLI 客户端。
CliClient createCliClient() => impl.createCliClient();

/// CLI 客户端抽象：扫描子模块状态。
abstract class CliClient {
  /// 扫描子模块同步状态并返回报告。
  ///
  /// 扫描目标由实现自行探测（桌面端从工作目录向上找含 `.gitmodules`
  /// 的 git 仓库根）。无法调用 CLI 时抛出 [CliException]，
  /// 由调用方降级为占位提示。
  Future<ScanReport> scan();
}

/// 子模块同步状态（对齐 CLI `SyncStatus` 四档）。
enum ComponentSyncStatus { synced, pendingPush, pendingPull, conflict }

/// 状态中文标签（与 CLI `SyncStatus::label()` 一致）。
String statusLabelOf(ComponentSyncStatus status) => switch (status) {
      ComponentSyncStatus.synced => '已同步',
      ComponentSyncStatus.pendingPush => '待推送',
      ComponentSyncStatus.pendingPull => '待拉取',
      ComponentSyncStatus.conflict => '冲突',
    };

/// 单个子模块状态（对齐 CLI `ComponentStatus`）。
class ScanComponent {
  const ScanComponent({
    required this.name,
    required this.status,
    this.ahead = 0,
    this.behind = 0,
  });

  final String name;
  final ComponentSyncStatus status;
  final int ahead;
  final int behind;

  String get statusLabel => statusLabelOf(status);
}

/// 扫描报告（对齐 CLI `StatusReport`）。
class ScanReport {
  const ScanReport({
    required this.root,
    required this.total,
    required this.synced,
    required this.pending,
    required this.components,
  });

  final String root;
  final int total;
  final int synced;
  final int pending;
  final List<ScanComponent> components;
}

/// CLI 调用失败：进程无法启动、非零退出码、平台不支持等。
class CliException implements Exception {
  const CliException(this.message);

  final String message;

  @override
  String toString() => message;
}

// ═══════════════════════════════════════════════════════════════════════
// 输出解析（对齐 src/cli/src/main.rs `print_report`）
// ═══════════════════════════════════════════════════════════════════════
//
// `qtcloud-devops code status <root> [--offline]` 输出示例：
//   仓库: /home/user/repo
//   组件总数: 3
//   待处理: 1
//     libs/sub             待推送 (领先 2 提交)
// 全部同步时末段为「全部组件已同步」；只有非同步组件会逐行列出。
// 详情后缀格式：` (领先 N 提交)` / ` (落后 N 提交)` / ` (+A/-B)` / 无。

const _componentStatusLabels = ['已同步', '待推送', '待拉取', '冲突'];

/// 解析 `code status` 文本输出为 [ScanReport]。
///
/// 未知行跳过（向前兼容），无法识别的组件行忽略。
ScanReport parseStatusReport(String output) {
  var root = '';
  var total = 0;
  var pending = 0;
  final components = <ScanComponent>[];

  for (final rawLine in output.split('\n')) {
    final line = rawLine.trim();
    if (line.isEmpty) continue;
    if (line.startsWith('仓库:')) {
      root = line.substring('仓库:'.length).trim();
    } else if (line.startsWith('组件总数:')) {
      total = _parseIntAfter(line, '组件总数:');
    } else if (line.startsWith('待处理:')) {
      pending = _parseIntAfter(line, '待处理:');
    } else if (line == '全部组件已同步') {
      // 无待处理组件，跳过。
    } else {
      final component = _parseComponentLine(line);
      if (component != null) components.add(component);
    }
  }
  return ScanReport(
    root: root,
    total: total,
    synced: total - pending,
    pending: pending,
    components: components,
  );
}

int _parseIntAfter(String line, String prefix) {
  final rest = line.substring(prefix.length).trim();
  final match = RegExp(r'\d+').firstMatch(rest);
  return match == null ? 0 : int.parse(match.group(0)!);
}

/// 解析组件行：`  <名称(左对齐 20 列)> <状态标签> <详情>`。
///
/// 用「已知状态标签 + 前置空格」定位分隔，不依赖固定列宽——
/// 名称超过 20 字符时填充消失，只剩单个空格分隔（对齐 AGENTS.md：
/// 解析外部输出用内容特征而非定界符）。
ScanComponent? _parseComponentLine(String line) {
  String? label;
  var labelIndex = -1;
  for (final candidate in _componentStatusLabels) {
    final index = line.indexOf(candidate);
    if (index > 0 &&
        line[index - 1] == ' ' &&
        (labelIndex < 0 || index < labelIndex)) {
      label = candidate;
      labelIndex = index;
    }
  }
  if (label == null) return null;

  final name = line.substring(0, labelIndex).trim();
  final detail = line.substring(labelIndex + label.length).trim();
  final (ahead: ahead, behind: behind) = _parseDetail(detail);
  return ScanComponent(
    name: name,
    status: _statusFromLabel(label),
    ahead: ahead,
    behind: behind,
  );
}

ComponentSyncStatus _statusFromLabel(String label) => switch (label) {
      '已同步' => ComponentSyncStatus.synced,
      '待推送' => ComponentSyncStatus.pendingPush,
      '待拉取' => ComponentSyncStatus.pendingPull,
      _ => ComponentSyncStatus.conflict,
    };

/// 解析详情后缀，返回 (ahead, behind)。
({int ahead, int behind}) _parseDetail(String detail) {
  final ahead = _extractCount(detail, '领先');
  final behind = _extractCount(detail, '落后');
  if (ahead > 0 || behind > 0) {
    return (ahead: ahead, behind: behind);
  }
  final both = RegExp(r'\+(\d+)\s*/\s*-(\d+)').firstMatch(detail);
  if (both != null) {
    return (ahead: int.parse(both.group(1)!), behind: int.parse(both.group(2)!));
  }
  return (ahead: 0, behind: 0);
}

int _extractCount(String detail, String marker) {
  final index = detail.indexOf(marker);
  if (index < 0) return 0;
  final rest = detail.substring(index + marker.length);
  final match = RegExp(r'\d+').firstMatch(rest);
  return match == null ? 0 : int.parse(match.group(0)!);
}

// ═══════════════════════════════════════════════════════════════════════
// 扫描目标探测
// ═══════════════════════════════════════════════════════════════════════

/// 返回 [start] 所属的 git 仓库根；不在任何 git 仓库中时返回 [start] 本身。
String gitRootOf(String start, {required bool Function(String dir) isGitRoot}) {
  var current = start;
  while (!isGitRoot(current)) {
    final parent = parentOf(current);
    if (parent == null) return start;
    current = parent;
  }
  return current;
}

/// 向上探测扫描目标：从 [start] 所属的 git 根开始逐级向上，
/// 返回最近的「git 仓库根且含 .gitmodules」的祖先目录（qtcloud-devops
/// 聚合仓库，中间可隔非 git 目录如 `apps/`）；找不到则回退到
/// [start] 所属的 git 根。
///
/// [isGitRoot] / [hasGitModules] 为文件系统判断，由调用方注入
/// （真实实现见 cli_client_io.dart，测试可注入假实现）。
String resolveScanRoot(
  String start, {
  required bool Function(String dir) isGitRoot,
  required bool Function(String dir) hasGitModules,
}) {
  final ownRoot = gitRootOf(start, isGitRoot: isGitRoot);
  var current = ownRoot;
  while (true) {
    if (isGitRoot(current) && hasGitModules(current)) return current;
    final parent = parentOf(current);
    if (parent == null) return ownRoot;
    current = parent;
  }
}

/// 返回 [dir] 的父目录；已是文件系统根（POSIX `/` 或 Windows 盘符根如
/// `C:\`）时返回 null。
String? parentOf(String dir) {
  var d = dir;
  while (d.endsWith('/') || d.endsWith('\\')) {
    d = d.substring(0, d.length - 1);
  }
  if (d.isEmpty || d == '/') return null;
  if (RegExp(r'^[A-Za-z]:$').hasMatch(d)) return null;
  final slash = d.lastIndexOf('/');
  final backslash = d.lastIndexOf('\\');
  final index = slash > backslash ? slash : backslash;
  if (index < 0) return null;
  final parent = d.substring(0, index);
  return parent.isEmpty ? '/' : parent;
}
