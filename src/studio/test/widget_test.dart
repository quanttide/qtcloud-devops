import 'package:flutter_test/flutter_test.dart';

import 'package:studio/app_state.dart';
import 'package:studio/main.dart';

void main() {
  testWidgets('量潮 DevOps 云可渲染', (WidgetTester tester) async {
    await tester.pumpWidget(DevOpsApp(state: AppState()));
    await tester.pumpAndSettle();

    expect(find.text('量潮 DevOps 云'), findsOneWidget);
    expect(find.text('子模块状态'), findsOneWidget);
    expect(find.text('扫描'), findsOneWidget);
  });
}
