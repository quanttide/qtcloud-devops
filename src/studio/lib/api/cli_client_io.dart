/// 桌面端 CLI 客户端：通过 `dart:io` 调用 qtcloud-devops CLI。
///
/// 调用命令：`qtcloud-devops code status <root> --offline`
/// （--offline 只读本地 remote-tracking refs，不逐个 fetch 远端，
/// 桌面端点击扫描即时返回；CLI 输出格式见 cli_client.dart 的解析说明）。
///
/// 可执行文件按顺序尝试，进程能启动即采用（避免反复构建）：
///   1. PATH 中的 `qtcloud-devops`（已 `cargo install` 的版本）
///   2. 当前仓库内预构建的 `src/cli/target/release/qtcloud-devops`
///   3. `cargo run --manifest-path <仓库>/src/cli/Cargo.toml`（从源码构建）
/// 全部无法启动时抛出 [CliException]，由 UI 降级为占位提示。
library;

import 'dart:convert';
import 'dart:io';

import 'cli_client.dart';

CliClient createCliClient() => CliProcessClient();

class CliProcessClient implements CliClient {
  @override
  Future<ScanReport> scan() async {
    final root = resolveScanRoot(
      Directory.current.path,
      isGitRoot: _isGitRoot,
      hasGitModules: _hasGitModules,
    );
    final baseArgs = ['code', 'status', root, '--offline'];

    final manifest = '${_studioRepoRoot()}/src/cli/Cargo.toml';
    final prebuilt = '${_studioRepoRoot()}/src/cli/target/release/qtcloud-devops';
    final candidates = <(String, List<String>)>[
      ('qtcloud-devops', baseArgs),
      if (File(prebuilt).existsSync()) (prebuilt, baseArgs),
      if (File(manifest).existsSync())
        ('cargo', ['run', '--quiet', '--manifest-path', manifest, '--', ...baseArgs]),
    ];

    Object? lastSpawnError;
    for (final (name, args) in candidates) {
      try {
        final result = await Process.run(
          name,
          args,
          stdoutEncoding: utf8,
          stderrEncoding: utf8,
        );
        if (result.exitCode != 0) {
          final stderr = result.stderr.toString().trim();
          throw CliException(
            'qtcloud-devops code status 失败（exit ${result.exitCode}）'
            '${stderr.isEmpty ? '' : '：$stderr'}',
          );
        }
        return parseStatusReport(result.stdout.toString());
      } on ProcessException catch (e) {
        // 进程无法启动（可执行文件缺失）→ 尝试下一个候选。
        lastSpawnError = e;
      }
    }
    throw CliException('无法调用 qtcloud-devops CLI：$lastSpawnError');
  }
}

/// 当前工作目录所属 git 仓库根（studio 所在仓库，用于定位 src/cli）。
String _studioRepoRoot() {
  return gitRootOf(Directory.current.path, isGitRoot: _isGitRoot);
}

bool _isGitRoot(String dir) =>
    File('$dir/.git').existsSync() || Directory('$dir/.git').existsSync();

bool _hasGitModules(String dir) => File('$dir/.gitmodules').existsSync();
