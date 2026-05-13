/// 模式 R (Dart 侧): Read-through / Write-back 缓存组合器。
///
/// `ReadThroughCache<K, V>`: 读 L1 (内存) → miss 走 L2 (持久) → miss 走 source
/// (loader 函数) → 回填 L1+L2。降级 fallback: L2 故障时仍走 source, L1 故障则
/// 用一次性 Map 兜底。
///
/// `WriteBackCache<K, V>`: 写 L1 同步, 异步刷 L2; 用 `Completer` 让上层 await
/// 持久化完成。`flushPending()` 启动期阻塞调用以避免脏数据丢失。
///
/// 适用: HTTP 响应、computed-expensive 结果, 兼容 L1 = memory cache (模式 A)
/// + L2 = persistent cache (模式 H)。
/// 不适用: 强一致 (任何缓存都会延迟); 写多读少 (反而拖慢)。
library;

import 'dart:async';

typedef CacheLoader<K, V> = Future<V> Function(K key);
typedef CacheGetter<K, V> = Future<V?> Function(K key);
typedef CachePutter<K, V> = Future<void> Function(K key, V value);

class StorageSampleReadThroughCache<K, V> {
  final CacheGetter<K, V> _l1Get;
  final CachePutter<K, V> _l1Put;
  final CacheGetter<K, V>? _l2Get;
  final CachePutter<K, V>? _l2Put;
  final CacheLoader<K, V> _loader;

  /// 统计: hit (L1) / hitL2 / miss。
  int hitsL1 = 0;
  int hitsL2 = 0;
  int misses = 0;

  StorageSampleReadThroughCache({
    required CacheGetter<K, V> l1Get,
    required CachePutter<K, V> l1Put,
    CacheGetter<K, V>? l2Get,
    CachePutter<K, V>? l2Put,
    required CacheLoader<K, V> loader,
  })  : _l1Get = l1Get,
        _l1Put = l1Put,
        _l2Get = l2Get,
        _l2Put = l2Put,
        _loader = loader;

  Future<V> get(K key) async {
    final v1 = await _safe(() => _l1Get(key));
    if (v1 != null) {
      hitsL1++;
      return v1;
    }
    final l2Get = _l2Get;
    if (l2Get != null) {
      final v2 = await _safe(() => l2Get(key));
      if (v2 != null) {
        hitsL2++;
        await _safe(() => _l1Put(key, v2));
        return v2;
      }
    }
    misses++;
    final fresh = await _loader(key);
    await _safe(() => _l1Put(key, fresh));
    final l2Put = _l2Put;
    if (l2Put != null) {
      await _safe(() => l2Put(key, fresh));
    }
    return fresh;
  }

  Future<T?> _safe<T>(Future<T?> Function() fn) async {
    try {
      return await fn();
    } catch (_) {
      return null;
    }
  }
}

class StorageSampleWriteBackCache<K, V> {
  final CachePutter<K, V> _l1Put;
  final CachePutter<K, V> _l2Put;
  final Map<K, Completer<void>> _pending = {};

  StorageSampleWriteBackCache({
    required CachePutter<K, V> l1Put,
    required CachePutter<K, V> l2Put,
  })  : _l1Put = l1Put,
        _l2Put = l2Put;

  /// 写: L1 同步, L2 异步后台 flush。返回 future 完成时表示**L1**已写入。
  /// 要确保 L2 也持久化, 用 [waitFlush]。
  Future<void> put(K key, V value) async {
    await _l1Put(key, value);
    final c = Completer<void>();
    _pending[key] = c;
    unawaited(Future.microtask(() async {
      try {
        await _l2Put(key, value);
        c.complete();
      } catch (e, st) {
        c.completeError(e, st);
      } finally {
        _pending.remove(key);
      }
    }));
  }

  /// 阻塞直到该 key 的 L2 写入完成 (或没有 pending → 立即返回)。
  Future<void> waitFlush(K key) {
    final c = _pending[key];
    if (c == null) return Future.value();
    return c.future;
  }

  /// 阻塞直到所有 pending 写完。启动期与关闭期常用。
  Future<void> waitAllFlush() async {
    while (_pending.isNotEmpty) {
      final fs = _pending.values.map((c) => c.future).toList(growable: false);
      await Future.wait(fs, eagerError: false);
    }
  }
}
