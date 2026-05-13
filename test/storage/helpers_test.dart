/// 纯 Dart 助手单元测试 (无需启动 Rust 桥, 无需 sqflite_ffi)。
///
/// 覆盖:
/// - 模式 O: TenantContext 命名空间隔离 + 校验 + KV 包装
/// - 模式 R: ReadThroughCache L1/L2 命中统计 + 故障降级
///           WriteBackCache 阻塞 flush
library;

import 'dart:async';

import 'package:flutter_test/flutter_test.dart';
import 'package:fragments/storage/cache_combinator.dart';
import 'package:fragments/storage/tenant.dart';

void main() {
  group('模式 O: TenantContext', () {
    test('合法 tenantId 通过, 非法 (空 / 大写 / 特殊符号) 抛 ArgumentError', () {
      expect(() => StorageSampleTenantContext('user_42'), returnsNormally);
      expect(() => StorageSampleTenantContext(''), throwsArgumentError);
      expect(() => StorageSampleTenantContext('User'), throwsArgumentError);
      expect(() => StorageSampleTenantContext('a/b'), throwsArgumentError);
      expect(
        () => StorageSampleTenantContext('a' * 65),
        throwsArgumentError,
      );
    });

    test('wrapKey 加前缀, unwrapKey 反向, belongs 判定', () {
      final t = StorageSampleTenantContext('acme');
      expect(t.wrapKey('profile'), 'tenant:acme:profile');
      expect(t.unwrapKey('tenant:acme:profile'), 'profile');
      expect(t.belongs('tenant:acme:profile'), isTrue);
      expect(t.belongs('tenant:other:x'), isFalse);
      expect(() => t.unwrapKey('tenant:other:x'), throwsStateError);
    });

    test('TenantedKv 自动给底层 key 加前缀', () async {
      final store = <String, String>{};
      final t = StorageSampleTenantContext('u1');
      final kv = StorageSampleTenantedKv<String>(
        tenant: t,
        read: (k) async => store[k],
        write: (k, v) async => store[k] = v,
        delete: (k) async => store.remove(k) != null,
      );
      await kv.put('greeting', 'hi');
      expect(store, {'tenant:u1:greeting': 'hi'});
      expect(await kv.get('greeting'), 'hi');
      expect(await kv.delete('greeting'), isTrue);
      expect(store, isEmpty);
    });

    test('未配置 delete 时调用抛 UnsupportedError', () async {
      final t = StorageSampleTenantContext('u1');
      final kv = StorageSampleTenantedKv<String>(
        tenant: t,
        read: ( k) async => null,
        write: (k, v) async {},
      );
      await expectLater(() => kv.delete('x'), throwsUnsupportedError);
    });
  });

  group('模式 R: ReadThroughCache', () {
    test('L1 命中不触发 L2 / loader', () async {
      final l1 = <String, int>{'k': 1};
      var loaderCalls = 0;
      final cache = StorageSampleReadThroughCache<String, int>(
        l1Get: (k) async => l1[k],
        l1Put: (k, v) async => l1[k] = v,
        loader: (k) async {
          loaderCalls++;
          return 99;
        },
      );
      expect(await cache.get('k'), 1);
      expect(cache.hitsL1, 1);
      expect(cache.hitsL2, 0);
      expect(cache.misses, 0);
      expect(loaderCalls, 0);
    });

    test('L1 miss 走 L2, 命中后回填 L1', () async {
      final l1 = <String, int>{};
      final l2 = <String, int>{'k': 42};
      final cache = StorageSampleReadThroughCache<String, int>(
        l1Get: (k) async => l1[k],
        l1Put: (k, v) async => l1[k] = v,
        l2Get: (k) async => l2[k],
        l2Put: (k, v) async => l2[k] = v,
        loader: (k) async => -1,
      );
      expect(await cache.get('k'), 42);
      expect(cache.hitsL2, 1);
      expect(l1['k'], 42, reason: 'L2 命中应回填 L1');
    });

    test('两层都 miss → loader, 同时回填 L1+L2', () async {
      final l1 = <String, int>{};
      final l2 = <String, int>{};
      final cache = StorageSampleReadThroughCache<String, int>(
        l1Get: (k) async => l1[k],
        l1Put: (k, v) async => l1[k] = v,
        l2Get: (k) async => l2[k],
        l2Put: (k, v) async => l2[k] = v,
        loader: (k) async => 7,
      );
      expect(await cache.get('x'), 7);
      expect(cache.misses, 1);
      expect(l1['x'], 7);
      expect(l2['x'], 7);
    });

    test('L2 抛错 → 降级走 loader, 不冒出', () async {
      final l1 = <String, int>{};
      final cache = StorageSampleReadThroughCache<String, int>(
        l1Get: (k) async => l1[k],
        l1Put: (k, v) async => l1[k] = v,
        l2Get: ( k) async => throw StateError('l2 down'),
        l2Put: (k, v) async => throw StateError('l2 down'),
        loader: (k) async => 5,
      );
      expect(await cache.get('a'), 5);
      expect(cache.misses, 1);
    });
  });

  group('模式 R: WriteBackCache', () {
    test('put 同步写 L1, 异步刷 L2; waitFlush 等到 L2 完成', () async {
      final l1 = <String, int>{};
      final l2 = <String, int>{};
      final l2Gate = Completer<void>();
      final cache = StorageSampleWriteBackCache<String, int>(
        l1Put: (k, v) async => l1[k] = v,
        l2Put: (k, v) async {
          await l2Gate.future;
          l2[k] = v;
        },
      );
      await cache.put('x', 1);
      expect(l1['x'], 1);
      expect(l2.containsKey('x'), isFalse);
      l2Gate.complete();
      await cache.waitFlush('x');
      expect(l2['x'], 1);
    });

    test('waitFlush 对未知 key 立即返回', () async {
      final cache = StorageSampleWriteBackCache<String, int>(
        l1Put: (k, v) async {},
        l2Put: (k, v) async {},
      );
      await cache.waitFlush('nothing');
    });

    test('waitAllFlush 等待全部 pending L2 写完', () async {
      final l2 = <String, int>{};
      final gate = Completer<void>();
      final cache = StorageSampleWriteBackCache<String, int>(
        l1Put: (k, v) async {},
        l2Put: (k, v) async {
          await gate.future;
          l2[k] = v;
        },
      );
      await cache.put('a', 1);
      await cache.put('b', 2);
      expect(l2, isEmpty);
      gate.complete();
      await cache.waitAllFlush();
      expect(l2, {'a': 1, 'b': 2});
    });
  });
}
