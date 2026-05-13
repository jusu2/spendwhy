/// 模式 V (RequestContext) 纯 Dart 测试 — 不启动 Rust 桥。
///
/// 覆盖:
/// - UUID v4 / traceparent 生成格式合规
/// - Deadline 单调时钟剩余 / 过期
/// - RequestContext 不可变更新 (withAttempt / withIdempotency / bumpAttempt)
/// - freeze() 在 deadline 到期时抛 RequestExpiredException

import 'dart:math';

import 'package:flutter_test/flutter_test.dart';
import 'package:fragments/transport/request_context.dart';

void main() {
  group('newRequestId (UUID v4)', () {
    test('格式: 8-4-4-4-12 小写十六进制', () {
      final id = newRequestId();
      expect(id.length, 36);
      expect(
        RegExp(r'^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$')
            .hasMatch(id),
        isTrue,
      );
    });

    test('version=4 / variant=10xx', () {
      final id = newRequestId();
      expect(id[14], '4'); // version nibble
      // variant nibble: 取 19 位字符 (跨过 3 个 dash)
      final variant = id[19];
      expect(['8', '9', 'a', 'b'].contains(variant), isTrue);
    });

    test('多次生成不重复 (碰撞极低)', () {
      final ids = {for (var i = 0; i < 1000; i++) newRequestId()};
      expect(ids.length, 1000);
    });

    test('注入 Random 后可重复 (测试可种子化)', () {
      final a = newRequestId(random: Random(42));
      final b = newRequestId(random: Random(42));
      expect(a, b);
    });
  });

  group('newTraceParent (W3C)', () {
    test('格式严格匹配', () {
      final tp = newTraceParent();
      expect(tp.length, 55);
      expect(
        RegExp(r'^00-[0-9a-f]{32}-[0-9a-f]{16}-01$').hasMatch(tp),
        isTrue,
      );
    });

    test('trace_id / span_id 非全 0', () {
      final tp = newTraceParent();
      expect(tp.substring(3, 35), isNot('0' * 32));
      expect(tp.substring(36, 52), isNot('0' * 16));
    });
  });

  group('Deadline', () {
    test('remaining 在 elapsed 后变小', () async {
      final d = Deadline(const Duration(milliseconds: 100));
      final start = d.remaining;
      await Future<void>.delayed(const Duration(milliseconds: 30));
      final later = d.remaining;
      expect(later, lessThan(start));
      expect(d.isExpired, isFalse);
    });

    test('超过 total 后 remaining=0 / isExpired=true / remainingMs=null', () async {
      final d = Deadline(const Duration(milliseconds: 5));
      await Future<void>.delayed(const Duration(milliseconds: 30));
      expect(d.remaining, Duration.zero);
      expect(d.isExpired, isTrue);
      expect(d.remainingMs, isNull);
    });

    test('assert: total 必须 > 0', () {
      expect(() => Deadline(Duration.zero), throwsA(isA<AssertionError>()));
    });
  });

  group('RequestContext', () {
    test('create 默认 attempt=1, 自动生成 requestId', () {
      final ctx = RequestContext.create(source: 'ui.test');
      expect(ctx.attempt, 1);
      expect(ctx.requestId.length, 36);
      expect(ctx.source, 'ui.test');
      expect(ctx.deadline, isNull);
    });

    test('withAttempt 不可变更新, 原 ctx 不变', () {
      final a = RequestContext.create();
      final b = a.withAttempt(3);
      expect(a.attempt, 1);
      expect(b.attempt, 3);
      expect(a.requestId, b.requestId);
    });

    test('bumpAttempt = attempt+1', () {
      final a = RequestContext.create();
      expect(a.bumpAttempt().attempt, 2);
      expect(a.bumpAttempt().bumpAttempt().attempt, 3);
    });

    test('withAttempt(<1) 抛 ArgumentError', () {
      final a = RequestContext.create();
      expect(() => a.withAttempt(0), throwsA(isA<ArgumentError>()));
    });

    test('withIdempotency / withLocale / withTrace 返回新副本', () {
      final a = RequestContext.create();
      final b = a.withIdempotency('k').withLocale('zh-CN').withTrace(
            newTraceParent(),
          );
      expect(a.idempotencyKey, isNull);
      expect(b.idempotencyKey, 'k');
      expect(b.locale, 'zh-CN');
      expect(b.traceParent, isNotNull);
    });

    test('freeze 把当前状态拷贝到 TransportRequestMeta', () {
      final ctx = RequestContext.create(
        timeout: const Duration(seconds: 1),
        source: 'svc',
      ).withIdempotency('idem').withLocale('en-US');
      final meta = ctx.freeze();
      expect(meta.requestId, ctx.requestId);
      expect(meta.attempt, 1);
      expect(meta.idempotencyKey, 'idem');
      expect(meta.locale, 'en-US');
      expect(meta.source, 'svc');
      expect(meta.budgetMs, isNotNull);
      expect(meta.budgetMs!.toInt(), inInclusiveRange(1, 1000));
    });

    test('freeze 在 deadline 过期时抛 RequestExpiredException', () async {
      final ctx = RequestContext.create(
        timeout: const Duration(milliseconds: 5),
      );
      await Future<void>.delayed(const Duration(milliseconds: 30));
      expect(ctx.isExpired, isTrue);
      expect(ctx.freeze, throwsA(isA<RequestExpiredException>()));
    });

    test('freeze 无 deadline 时 budgetMs=null', () {
      final meta = RequestContext.create().freeze();
      expect(meta.budgetMs, isNull);
    });
  });
}
