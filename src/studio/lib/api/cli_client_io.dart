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

import 'dart:async';
import 'dart:convert';
import 'dart:io';

import 'cli_client.dart';

/// CLI 单次调用的超时（对齐 provider 的 cliScanTimeout：60s）。
/// cargo 候选首次运行要编译整个项目，可能远超离线扫描本身。
const _cliTimeout = Duration(seconds: 60);

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
        final result = await _runWithTimeout(name, args);
        if (result.exitCode != 0) {
          final stderr = result.stderr.trim();
          throw CliException(
            'qtcloud-devops code status 失败（exit ${result.exitCode}）'
            '${stderr.isEmpty ? '' : '：$stderr'}',
          );
        }
        return parseStatusReport(result.stdout);
      } on TimeoutException {
        // CLI 挂起（如首次 cargo 编译超长）→ 不尝试其他候选，直接报错。
        throw CliException('qtcloud-devops code status 超时（${_cliTimeout.inSeconds}s）');
      } on ProcessException catch (e) {
        // 进程无法启动（可执行文件缺失）→ 尝试下一个候选。
        lastSpawnError = e;
      }
    }
    throw CliException('无法调用 qtcloud-devops CLI：$lastSpawnError');
  }

  /// 启动进程并等待完成；超过 [_cliTimeout] 时 kill 进程并抛 [TimeoutException]。
  /// （`Process.run` 无 timeout 参数，故手动管理。）
  Future<({int exitCode, String stdout, String stderr})> _runWithTimeout(
    String name,
    List<String> args,
  ) async {
    final process = await Process.start(name, args);
    final stdout = process.stdout.transform(utf8.decoder).join();
    final stderr = process.stderr.transform(utf8.decoder).join();
    final exitCode = await process.exitCode.timeout(_cliTimeout, onTimeout: () {
      process.kill(ProcessSignal.sigkill);
      throw TimeoutException('qtcloud-devops code status 超时');
    });
    return (exitCode: exitCode, stdout: await stdout, stderr: await stderr);
  }
}

/// 当前工作目录所属 git 仓库根（studio 所在仓库，用于定位 src/cli）。
String _studioRepoRoot() {
  return gitRootOf(Directory.current.path, isGitRoot: _isGitRoot);
}

bool _isGitRoot(String dir) =>
    File('$dir/.git').existsSync() || Directory('$dir/.git').existsSync();

bool _hasGitModules(String dir) => File('$dir/.gitmodules').existsSync();
