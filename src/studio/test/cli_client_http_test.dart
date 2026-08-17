import 'package:flutter_test/flutter_test.dart';
import 'package:http/http.dart' as http;
import 'package:http/testing.dart';

import 'package:studio/api/cli_client.dart';
import 'package:studio/api/cli_client_http.dart';

void main() {
  group('HttpCliClient', () {
    test('解析 provider JSON 报告', () async {
      final client = MockClient((request) async {
        expect(request.url.toString(), 'http://provider.test:8080/api/scan');
        return http.Response(
          '''
{
  "root": "/home/user/repo",
  "total": 5,
  "synced": 1,
  "pending": 4,
  "components": [
    {"name": "libs/sub", "status": "pending_push", "ahead": 2, "behind": 0},
    {"name": "docs/tutorial", "status": "pending_pull", "ahead": 0, "behind": 1},
    {"name": "examples/default", "status": "conflict", "ahead": 3, "behind": 1},
    {"name": "apps/core", "status": "conflict", "ahead": 0, "behind": 0}
  ]
}
''',
          200,
          headers: {'content-type': 'application/json; charset=utf-8'},
        );
      });
      final cli = HttpCliClient(
        baseUrl: 'http://provider.test:8080',
        client: client,
      );

      final report = await cli.scan();
      expect(report.root, '/home/user/repo');
      expect(report.total, 5);
      expect(report.synced, 1);
      expect(report.pending, 4);
      expect(report.components, hasLength(4));
      expect(report.components[0].name, 'libs/sub');
      expect(report.components[0].status, ComponentSyncStatus.pendingPush);
      expect(report.components[0].ahead, 2);
      expect(report.components[1].status, ComponentSyncStatus.pendingPull);
      expect(report.components[1].behind, 1);
      expect(report.components[2].status, ComponentSyncStatus.conflict);
      expect(report.components[2].ahead, 3);
      expect(report.components[2].behind, 1);
    });

    test('provider 非 200 时抛 CliException', () async {
      final client = MockClient(
        (request) async => http.Response('internal error', 502),
      );
      final cli = HttpCliClient(baseUrl: 'http://provider.test', client: client);

      await expectLater(
        cli.scan(),
        throwsA(isA<CliException>().having(
          (e) => e.message,
          'message',
          contains('HTTP 502'),
        )),
      );
    });

    test('provider 不可达时抛 CliException（连接失败）', () async {
      final client = MockClient(
        (request) async => throw http.ClientException('Connection refused'),
      );
      final cli = HttpCliClient(baseUrl: 'http://provider.test', client: client);

      await expectLater(
        cli.scan(),
        throwsA(isA<CliException>().having(
          (e) => e.message,
          'message',
          contains('无法连接 provider'),
        )),
      );
    });

    test('provider 响应非法 JSON 时抛 CliException', () async {
      final client = MockClient((request) async => http.Response('oops', 200));
      final cli = HttpCliClient(baseUrl: 'http://provider.test', client: client);

      await expectLater(
        cli.scan(),
        throwsA(isA<CliException>().having(
          (e) => e.message,
          'message',
          contains('不是合法 JSON'),
        )),
      );
    });
  });
}
