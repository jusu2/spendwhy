/// 模式 J: 请求合并 (Request Coalescing)。
///
/// 短时间内对相同 key 的并发请求合并为一次底层调用, 其余等待者共享结果。
/// **行为约定**: 失败的 future 也会被等待者拿到 (它们都看到同一个 exception),
/// 但失败完成后 inflight 条目立即移除 → 下次同 key 调用会真正重发。
///
/// 用法:
/// ```dart
/// final coalescer = RequestCoalescer<String, UserDto>();
/// final user = await coalescer.run(
///   userId,
///   () => transportSampleCompute(input: userId),
/// );
/// ```
// Coalescer 的设计就是 fire-and-forget 内部 future, 由 Completer 桥接给等待者。
// ignore_for_file: discarded_futures, unawaited_futures
library;

import 'dart:async';

class RequestCoalescer<K, V> {
  final _inflight = <K, Future<V>>{};

  /// 合并相同 [key] 的并发请求。
  Future<V> run(K key, Future<V> Function() task) {
    final existing = _inflight[key];
    if (existing != null) return existing;
    final completer = Completer<V>();
    _inflight[key] = completer.future;
    _drive(key, task, completer);
    return completer.future;
  }

  Future<void> _drive(
    K key,
    Future<V> Function() task,
    Completer<V> completer,
  ) async {
    try {
      completer.complete(await task());
    } catch (e, st) {
      completer.completeError(e, st);
    } finally {
      _inflight.remove(key);
    }
  }

  /// 仅返回正在进行的 future, 不触发新调用。
  Future<V>? peek(K key) => _inflight[key];

  /// 强制清空 (测试 / hot-restart 重置场景)。返回清理前的条数。
  /// 注: 不会取消已经注册的 future, 它们仍会完成。
  int clear() {
    final n = _inflight.length;
    _inflight.clear();
    return n;
  }

  /// 当前进行中的请求数量。
  int get inflightCount => _inflight.length;

  /// 当前持有 inflight future 的所有 key (供观测)。
  Iterable<K> get keys => _inflight.keys;
}
