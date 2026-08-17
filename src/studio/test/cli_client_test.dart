import 'package:flutter_test/flutter_test.dart';

import 'package:studio/api/cli_client.dart';

void main() {
  group('parseStatusReport', () {
    test('解析待处理输出（四种详情格式与汇总行）', () {
      // 注意：真实 CLI 只列出非同步组件行（main.rs:201 跳过 Synced），
      // 这里的 data/ok「已同步」行是向前兼容的容错输入——解析器应容忍。
      const output = '''
仓库: /home/user/repo
组件总数: 5
待处理: 4
  libs/sub             待推送 (领先 2 提交)
  docs/tutorial        待拉取 (落后 1 提交)
  examples/default     冲突 (+3/-1)
  apps/core            冲突
  data/ok              已同步
''';
      final report = parseStatusReport(output);
      expect(report.root, '/home/user/repo');
      expect(report.total, 5);
      expect(report.pending, 4);
      expect(report.synced, 1);
      expect(report.components, hasLength(5));
      expect(report.components[0].name, 'libs/sub');
      expect(report.components[0].status, ComponentSyncStatus.pendingPush);
      expect(report.components[0].ahead, 2);
      expect(report.components[0].behind, 0);
      expect(report.components[1].name, 'docs/tutorial');
      expect(report.components[1].status, ComponentSyncStatus.pendingPull);
      expect(report.components[1].behind, 1);
      expect(report.components[2].name, 'examples/default');
      expect(report.components[2].status, ComponentSyncStatus.conflict);
      expect(report.components[2].ahead, 3);
      expect(report.components[2].behind, 1);
      expect(report.components[3].name, 'apps/core');
      expect(report.components[3].status, ComponentSyncStatus.conflict);
      expect(report.components[3].ahead, 0);
      expect(report.components[3].behind, 0);
      expect(report.components[4].status, ComponentSyncStatus.synced);
    });

    test('解析真实 CLI 输出（code status . --offline 捕获）', () {
      const output = '''仓库: /home/iguo/repos/quanttide/domains/quanttide-devops
组件总数: 20
待处理: 5
  data/context         冲突 (落后 2 提交)
  data/insight         冲突 (落后 1 提交)
  data/journal         冲突 (落后 29 提交)
  docs/tutorial        冲突 (落后 2 提交)
  examples/default     冲突
''';
      final report = parseStatusReport(output);
      expect(report.root, '/home/iguo/repos/quanttide/domains/quanttide-devops');
      expect(report.total, 20);
      expect(report.pending, 5);
      expect(report.synced, 15);
      expect(report.components, hasLength(5));
      expect(report.components.first.name, 'data/context');
      expect(report.components.first.behind, 2);
      expect(report.components.last.name, 'examples/default');
      expect(report.components.last.behind, 0);
    });

    test('全部同步时只有汇总行', () {
      const output = '''仓库: /tmp/repo
组件总数: 3
全部组件已同步
''';
      final report = parseStatusReport(output);
      expect(report.total, 3);
      expect(report.pending, 0);
      expect(report.synced, 3);
      expect(report.components, isEmpty);
    });

    test('空输出返回零值报告', () {
      final report = parseStatusReport('');
      expect(report.root, '');
      expect(report.total, 0);
      expect(report.pending, 0);
      expect(report.synced, 0);
      expect(report.components, isEmpty);
    });
  });

  group('statusLabelOf', () {
    test('四档状态标签与 CLI 一致', () {
      expect(statusLabelOf(ComponentSyncStatus.synced), '已同步');
      expect(statusLabelOf(ComponentSyncStatus.pendingPush), '待推送');
      expect(statusLabelOf(ComponentSyncStatus.pendingPull), '待拉取');
      expect(statusLabelOf(ComponentSyncStatus.conflict), '冲突');
    });
  });

  group('parentOf', () {
    test('POSIX 路径', () {
      expect(parentOf('/a/b'), '/a');
      expect(parentOf('/a'), '/');
      expect(parentOf('/'), isNull);
      expect(parentOf('/a/b/'), '/a');
    });

    test('Windows 路径', () {
      expect(parentOf(r'C:\a\b'), r'C:\a');
      expect(parentOf(r'C:\'), isNull);
      expect(parentOf(r'C:'), isNull);
    });

    test('相对路径与无分隔符路径', () {
      expect(parentOf('a'), isNull);
      expect(parentOf(''), isNull);
    });
  });

  group('resolveScanRoot', () {
    test('向上找到含 .gitmodules 的聚合仓库根（中间隔着非 git 目录）', () {
      // 真实结构：src/studio → apps/qtcloud-devops（git 根，无 .gitmodules）
      // → apps/（非 git 目录）→ 聚合仓库根（git 根 + .gitmodules）。
      const gitRoots = {'/mono/apps/qtcloud-devops', '/mono'};
      const withModules = {'/mono'};
      final root = resolveScanRoot(
        '/mono/apps/qtcloud-devops/src/studio',
        isGitRoot: gitRoots.contains,
        hasGitModules: withModules.contains,
      );
      expect(root, '/mono');
    });

    test('自身仓库即含 .gitmodules 时直接返回自身', () {
      const gitRoots = {'/repo'};
      const withModules = {'/repo'};
      final root = resolveScanRoot(
        '/repo/sub',
        isGitRoot: gitRoots.contains,
        hasGitModules: withModules.contains,
      );
      expect(root, '/repo');
    });

    test('祖先均无 .gitmodules 时回退自身 git 根', () {
      const gitRoots = {'/repo'};
      final root = resolveScanRoot(
        '/repo/src/studio',
        isGitRoot: gitRoots.contains,
        hasGitModules: (_) => false,
      );
      expect(root, '/repo');
    });

    test('不在任何 git 仓库中时返回起始目录', () {
      final root = resolveScanRoot(
        '/plain/dir',
        isGitRoot: (_) => false,
        hasGitModules: (_) => false,
      );
      expect(root, '/plain/dir');
    });
  });
}
