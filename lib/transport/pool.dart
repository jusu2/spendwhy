/// 模式 N (Dart 侧): Semaphore / 资源池。
///
/// 限制对昂贵下游 (Rust 调用、HTTP) 的并发量。与 Rust 侧
/// [`transportSampleThrottledOp`] 配合使用, 形成双层防护。
///
/// 用法:
/// ```dart
/// final pool = Semaphore(4);
/// await Future.wait(items.map((it) => pool.run(() => doRustWork(it))));
///
/// // 非阻塞: 满了就返回 null。
/// final maybe = await pool.tryRun(() => doRustWork(item));
///
/// // 带取消: 等待中途取消即抛 SemaphoreCancelledException。
/// final cancel = SemaphoreCancelToken();
/// await pool.run(work, cancelToken: cancel);
/// ```
library;

import 'dart:async';
import 'dart:collection';

class SemaphoreCancelToken {
  bool _cancelled = false;
  bool get isCancelled => _cancelled;
  void cancel() {
    _cancelled = true;
  }
}

class SemaphoreCancelledException implements Exception {
  const SemaphoreCancelledException();
  @override
  String toString() => 'SemaphoreCancelledException';
}

class _Waiter {
  final Completer<void> completer = Completer<void>();
  final SemaphoreCancelToken? token;
  _Waiter(this.token);
}

class Semaphore {
  final int _limit;
  int _current = 0;
  final _waiters = Queue<_Waiter>();
  bool _closed = false;

  Semaphore(this._limit) : assert(_limit > 0);

  /// 阻塞地获取一个槽位并执行 `task`。
  /// 若提供 [cancelToken] 且在等待期间被取消 → 抛 [SemaphoreCancelledException]。
  Future<T> run<T>(
    Future<T> Function() task, {
    SemaphoreCancelToken? cancelToken,
  }) async {
    await _acquire(cancelToken);
    try {
      return await task();
    } finally {
      _release();
    }
  }

  /// 非阻塞: 当前有空槽则执行, 否则返回 `null`。
  Future<T?> tryRun<T>(Future<T> Function() task) async {
    if (_closed || _current >= _limit) return null;
    _current++;
    try {
      return await task();
    } finally {
      _release();
    }
  }

  Future<void> _acquire(SemaphoreCancelToken? token) {
    if (_closed) {
      return Future.error(const SemaphoreCancelledException());
    }
    if (token?.isCancelled ?? false) {
      return Future.error(const SemaphoreCancelledException());
    }
    if (_current < _limit) {
      _current++;
      return Future.value();
    }
    final w = _Waiter(token);
    _waiters.add(w);
    return w.completer.future;
  }

  void _release() {
    while (_waiters.isNotEmpty) {
      final w = _waiters.removeFirst();
      if (w.token?.isCancelled ?? false) {
        if (!w.completer.isCompleted) {
          w.completer.completeError(const SemaphoreCancelledException());
        }
        continue;
      }
      if (!w.completer.isCompleted) {
        w.completer.complete();
        return;
      }
    }
    _current--;
  }

  /// 主动取消一个 token 对应的所有等待者。
  /// 已在临界区内的任务不受影响 (Dart 没有协作中断)。
  void cancelWaiters(SemaphoreCancelToken token) {
    final remaining = Queue<_Waiter>();
    while (_waiters.isNotEmpty) {
      final w = _waiters.removeFirst();
      if (identical(w.token, token) && !w.completer.isCompleted) {
        w.completer.completeError(const SemaphoreCancelledException());
      } else {
        remaining.add(w);
      }
    }
    _waiters.addAll(remaining);
  }

  /// 关闭信号量: 拒绝新 acquire, 已排队的全部失败。
  void close() {
    if (_closed) return;
    _closed = true;
    while (_waiters.isNotEmpty) {
      final w = _waiters.removeFirst();
      if (!w.completer.isCompleted) {
        w.completer.completeError(const SemaphoreCancelledException());
      }
    }
  }

  int get inUse => _current;
  int get available => _limit - _current;
  int get queueLength => _waiters.length;
  int get limit => _limit;
  bool get isClosed => _closed;
}
