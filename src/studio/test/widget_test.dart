import 'package:flutter_test/flutter_test.dart';

import 'package:studio/api/cli_client.dart';
import 'package:studio/app_state.dart';
import 'package:studio/main.dart';

void main() {
  testWidgets('量潮 DevOps 云可渲染（占位提示）', (WidgetTester tester) async {
    await tester.pumpWidget(DevOpsApp(state: AppState()));
    await tester.pumpAndSettle();

    expect(find.text('量潮 DevOps 云'), findsOneWidget);
    expect(find.text('子模块状态'), findsOneWidget);
    expect(find.text('扫描'), findsOneWidget);
    expect(find.text('未扫描'), findsOneWidget);
    expect(
      find.text('暂无子模块数据——点击「扫描」调用 qtcloud-devops CLI 获取状态'),
      findsOneWidget,
    );
  });

  testWidgets('扫描成功展示子模块状态列表', (WidgetTester tester) async {
    final state = AppState(cliClient: _FakeCliClient(report: _report));
    await tester.pumpWidget(DevOpsApp(state: state));

    await tester.tap(find.text('扫描'));
    await tester.pumpAndSettle();

    expect(find.textContaining('2 个组件'), findsOneWidget);
    expect(find.text('libs/sub'), findsOneWidget);
    expect(find.text('待推送'), findsOneWidget);
    expect(find.text('docs/tutorial'), findsOneWidget);
    expect(find.text('待拉取'), findsOneWidget);
    expect(find.text('领先 2 提交'), findsOneWidget);
    expect(find.text('落后 1 提交'), findsOneWidget);
  });

  testWidgets('CLI 不可用时保留占位提示', (WidgetTester tester) async {
    final state = AppState(
      cliClient: _FakeCliClient(error: const CliException('CLI 扫描仅支持桌面端（Linux/macOS/Windows）')),
    );
    await tester.pumpWidget(DevOpsApp(state: state));

    await tester.tap(find.text('扫描'));
    await tester.pumpAndSettle();

    expect(find.textContaining('无法调用 qtcloud-devops CLI'), findsOneWidget);
    expect(find.text('未扫描'), findsOneWidget);
    expect(
      find.text('暂无子模块数据——点击「扫描」调用 qtcloud-devops CLI 获取状态'),
      findsOneWidget,
    );
  });
}

const _report = ScanReport(
  root: '/tmp/repo',
  total: 2,
  synced: 0,
  pending: 2,
  components: [
    ScanComponent(
      name: 'libs/sub',
      status: ComponentSyncStatus.pendingPush,
      ahead: 2,
    ),
    ScanComponent(
      name: 'docs/tutorial',
      status: ComponentSyncStatus.pendingPull,
      behind: 1,
    ),
  ],
);

class _FakeCliClient implements CliClient {
  _FakeCliClient({this.report, this.error});

  final ScanReport? report;
  final Object? error;

  @override
  Future<ScanReport> scan() async {
    if (error != null) throw error!;
    return report!;
  }
}
