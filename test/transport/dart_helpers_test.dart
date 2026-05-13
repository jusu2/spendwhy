/// 纯 Dart 助手单元测试 (无需启动 Rust 桥)。
///
/// 覆盖:
/// - 模式 J: RequestCoalescer 合并并发请求 (含错误不缓存 / peek / clear)
/// - 模式 L: retry 退避 + 不可重试错误立即抛 + totalTimeout + CancelToken + onRetry
/// - 模式 N: Semaphore 限制并发量 + tryRun + cancelWaiters + close
/// - 模式 Q: TransportMockRegistry / withMocks per-test 隔离

import 'dart:async';
import 'dart:math' as math;

import 'package:flutter_test/flutter_test.dart';
import 'package:fragments/transport/coalescing.dart';
import 'package:fragments/transport/mock.dart';
import 'package:fragments/transport/pool.dart';
import 'package:fragments/transport/retry.dart';

void main() {
  group('模式 J: RequestCoalescer', () {
    test('相同 key 的并发请求只触发一次 task', () async {
      final coalescer = RequestCoalescer<String, int>();
      var taskCalls = 0;
      final gate = Completer<int>();
      Future<int> task() async {
        taskCalls++;
        return gate.future;
      }

      final f1 = coalescer.run('k', task);
      final f2 = coalescer.run('k', task);
      final f3 = coalescer.run('k', task);

      expect(taskCalls, 1);
      expect(coalescer.inflightCount, 1);

      gate.complete(42);
      expect(await f1, 42);
      expect(await f2, 42);
      expect(await f3, 42);
      expect(coalescer.inflightCount, 0);
    });

    test('不同 key 的请求独立执行', () async {
      final coalescer = RequestCoalescer<String, int>();
      var calls = 0;
      final a = coalescer.run('a', () async {
        calls++;
        return 1;
      });
      final b = coalescer.run('b', () async {
        calls++;
        return 2;
      });
      expect(await a, 1);
      expect(await b, 2);
      expect(calls, 2);
    });

    test('失败的 future 不缓存, 下次同 key 重新触发', () async {
      final coalescer = RequestCoalescer<String, int>();
      var calls = 0;
      await expectLater(
        coalescer.run('k', () async {
          calls++;
          throw StateError('boom');
        }),
        throwsA(isA<StateError>()),
      );
      expect(coalescer.inflightCount, 0);
      final v = await coalescer.run('k', () async {
        calls++;
        return 7;
      });
      expect(v, 7);
      expect(calls, 2);
    });

    test('peek 返回 inflight future, clear 重置', () async {
      final coalescer = RequestCoalescer<String, int>();
      final gate = Completer<int>();
      final f = coalescer.run('k', () => gate.future);
      expect(coalescer.peek('k'), same(f));
      expect(coalescer.keys, ['k']);
      final n = coalescer.clear();
      expect(n, 1);
      expect(coalescer.peek('k'), isNull);
      gate.complete(1);
      expect(await f, 1);
    });
  });

  group('模式 L: retry', () {
    test('成功的任务不会重试', () async {
      var calls = 0;
      final v = await retry<int>(() async {
        calls++;
        return 7;
      });
      expect(v, 7);
      expect(calls, 1);
    });

    test('可重试错误退避后再次执行', () async {
      var calls = 0;
      final v = await retry<int>(
        () async {
          calls++;
          if (calls < 3) throw StateError('transient');
          return 99;
        },
        policy: const RetryPolicy(
          maxAttempts: 5,
          initialBackoff: Duration(milliseconds: 1),
          maxBackoff: Duration(milliseconds: 2),
        ),
      );
      expect(v, 99);
      expect(calls, 3);
    });

    test('shouldRetry=false 时立即抛出', () async {
      var calls = 0;
      await expectLater(
        () => retry<int>(
          () async {
            calls++;
            throw ArgumentError('bad');
          },
          policy: const RetryPolicy(maxAttempts: 5),
          shouldRetry: (_) => false,
        ),
        throwsA(isA<ArgumentError>()),
      );
      expect(calls, 1);
    });

    test('耗尽 maxAttempts 后抛最后一次错误', () async {
      var calls = 0;
      await expectLater(
        () => retry<int>(
          () async {
            calls++;
            throw StateError('persist');
          },
          policy: const RetryPolicy(
            maxAttempts: 3,
            initialBackoff: Duration(milliseconds: 1),
          ),
        ),
        throwsA(isA<StateError>()),
      );
      expect(calls, 3);
    });

    test('totalTimeout 触发后抛 TimeoutException', () async {
      var calls = 0;
      await expectLater(
        () => retry<int>(
          () async {
            calls++;
            throw StateError('transient');
          },
          policy: const RetryPolicy(
            maxAttempts: 100,
            initialBackoff: Duration(milliseconds: 20),
            maxBackoff: Duration(milliseconds: 40),
            jitterRatio: 0,
            totalTimeout: Duration(milliseconds: 50),
          ),
          random: math.Random(0),
        ),
        throwsA(isA<TimeoutException>()),
      );
      expect(calls, lessThan(100));
    });

    test('CancelToken 取消在等待 backoff 时立即抛', () async {
      final token = CancelToken();
      var calls = 0;
      final future = retry<int>(
        () async {
          calls++;
          if (calls == 1) {
            Timer(const Duration(milliseconds: 5), token.cancel);
            throw StateError('again');
          }
          return 1;
        },
        policy: const RetryPolicy(
          maxAttempts: 10,
          initialBackoff: Duration(milliseconds: 500),
          jitterRatio: 0,
        ),
        cancelToken: token,
      );
      await expectLater(future, throwsA(isA<RetryCancelledException>()));
      expect(calls, 1);
    });

    test('onRetry 在每次失败重试时回调', () async {
      final attempts = <int>[];
      await retry<int>(
        () async {
          if (attempts.length < 2) throw StateError('x');
          return 0;
        },
        policy: const RetryPolicy(
          maxAttempts: 5,
          initialBackoff: Duration(milliseconds: 1),
          maxBackoff: Duration(milliseconds: 1),
          jitterRatio: 0,
        ),
        onRetry: (attempt, _, __) => attempts.add(attempt),
      );
      expect(attempts, [1, 2]);
    });
  });

  group('模式 N: Semaphore', () {
    test('立即放行 limit 个, 多出部分入队', () async {
      final pool = Semaphore(2);
      final gates = List.generate(4, (_) => Completer<int>());
      final futures = [
        for (var i = 0; i < 4; i++) pool.run<int>(() => gates[i].future),
      ];

      // 让微任务跑完: 前两个 acquire 成功, 后两个进入等待。
      await Future<void>.delayed(Duration.zero);
      expect(pool.inUse, 2);
      expect(pool.queueLength, 2);

      // 完成 0 → 释放槽位 → waiter[0] (即 future[2]) 进入临界区
      gates[0].complete(10);
      expect(await futures[0], 10);
      await Future<void>.delayed(Duration.zero);
      expect(pool.queueLength, 1);

      // 完成 1 → waiter[1] (即 future[3]) 进入
      gates[1].complete(11);
      expect(await futures[1], 11);
      await Future<void>.delayed(Duration.zero);
      expect(pool.queueLength, 0);

      // 完成剩余
      gates[2].complete(12);
      gates[3].complete(13);
      expect(await futures[2], 12);
      expect(await futures[3], 13);
      expect(pool.inUse, 0);
    });

    test('tryRun 在满时返回 null, 不入队', () async {
      final pool = Semaphore(1);
      final hold = Completer<int>();
      // 占满唯一槽位。
      final busy = pool.run<int>(() => hold.future);
      await Future<void>.delayed(Duration.zero);
      final attempt = await pool.tryRun<int>(() async => 1);
      expect(attempt, isNull);
      expect(pool.queueLength, 0);
      hold.complete(0);
      await busy;
      // 释放后 tryRun 成功。
      final ok = await pool.tryRun<int>(() async => 2);
      expect(ok, 2);
    });

    test('cancelWaiters 让目标 token 的等待者立即失败', () async {
      final pool = Semaphore(1);
      final hold = Completer<int>();
      final busy = pool.run<int>(() => hold.future);
      await Future<void>.delayed(Duration.zero);

      final token = SemaphoreCancelToken();
      final cancelled = pool.run<int>(
        () async => 99,
        cancelToken: token,
      );
      await Future<void>.delayed(Duration.zero);
      expect(pool.queueLength, 1);
      pool.cancelWaiters(token);
      await expectLater(cancelled, throwsA(isA<SemaphoreCancelledException>()));
      expect(pool.queueLength, 0);

      hold.complete(0);
      await busy;
    });

    test('close 让所有等待者失败, 之后 acquire 立刻抛', () async {
      final pool = Semaphore(1);
      final hold = Completer<int>();
      final busy = pool.run<int>(() => hold.future);
      await Future<void>.delayed(Duration.zero);
      final queued = pool.run<int>(() async => 1);
      await Future<void>.delayed(Duration.zero);
      pool.close();
      await expectLater(queued, throwsA(isA<SemaphoreCancelledException>()));
      expect(pool.isClosed, isTrue);
      await expectLater(
        pool.run<int>(() async => 2),
        throwsA(isA<SemaphoreCancelledException>()),
      );
      hold.complete(0);
      await busy;
    });
  });

  group('模式 Q: TransportMockRegistry', () {
    tearDown(TransportMockRegistry.instance.reset);

    test('withMocks 临时覆盖, 退出时恢复旧 stub', () async {
      final reg = TransportMockRegistry.instance;
      reg.register<int>('rust.add', (args) => 100);

      await withMocks({
        'rust.add': (args) => 1,
      }, () async {
        final v = await reg.tryInvoke<int>('rust.add', [1, 2]);
        expect(v, 1);
      });

      final restored = await reg.tryInvoke<int>('rust.add', [1, 2]);
      expect(restored, 100);
    });

    test('withMocks 注册新名字, 退出时删除', () async {
      final reg = TransportMockRegistry.instance;
      expect(reg.tryInvoke<int>('rust.new', const []), isNull);

      await withMocks({
        'rust.new': (args) => 7,
      }, () async {
        expect(await reg.tryInvoke<int>('rust.new', const []), 7);
      });

      expect(reg.tryInvoke<int>('rust.new', const []), isNull);
    });

    test('未注册 stub 返回 null (让业务侧 fallback 到真实 FRB)', () {
      expect(
        TransportMockRegistry.instance.tryInvoke<int>('absent', const []),
        isNull,
      );
    });

    test('stub 返回错误类型时抛 (TypeError / StateError)', () async {
      final reg = TransportMockRegistry.instance;
      // 故意宽签名: 在 tryInvoke<int> 时触发运行时类型不匹配。
      reg.register<Object>('rust.bad', (args) => 'not-an-int');
      await expectLater(
        () => reg.tryInvoke<int>('rust.bad', const [])!,
        throwsA(anyOf(isA<TypeError>(), isA<StateError>())),
      );
    });
  });
}
