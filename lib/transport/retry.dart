/// 模式 L (Dart 侧): 重试 + 指数退避 + 抖动 + 总超时 + 取消令牌。
///
/// 用法:
/// ```dart
/// final cancel = CancelToken();
/// final receipt = await retry<TransportSampleReceiptDto>(
///   () => transportSampleApplyOnce(
///     idempotencyKey: requestId,
///     payload: body,
///   ),
///   policy: const RetryPolicy(maxAttempts: 5),
///   shouldRetry: (e) => e is TransportError && e.isRetriable,
///   cancelToken: cancel,
///   onRetry: (attempt, error, delay) =>
///       log.warn('attempt $attempt failed: $error, retrying in $delay'),
/// );
/// ```
library;

import 'dart:async';
import 'dart:math' as math;

class RetryPolicy {
  final int maxAttempts;
  final Duration initialBackoff;
  final Duration maxBackoff;
  final double multiplier;
  final double jitterRatio;

  /// 整个重试链的总超时 (含 backoff 等待)。`null` = 不限。
  final Duration? totalTimeout;

  const RetryPolicy({
    this.maxAttempts = 3,
    this.initialBackoff = const Duration(milliseconds: 100),
    this.maxBackoff = const Duration(seconds: 5),
    this.multiplier = 2.0,
    this.jitterRatio = 0.2,
    this.totalTimeout,
  })  : assert(maxAttempts > 0),
        assert(multiplier >= 1.0),
        assert(jitterRatio >= 0.0 && jitterRatio <= 1.0);

  Duration delayFor(int attempt, math.Random rng) {
    final base =
        initialBackoff.inMilliseconds * math.pow(multiplier, attempt - 1);
    final capped = math.min(base, maxBackoff.inMilliseconds.toDouble());
    final jitter = capped * jitterRatio * (rng.nextDouble() * 2 - 1);
    final ms = (capped + jitter).clamp(0, maxBackoff.inMilliseconds).toInt();
    return Duration(milliseconds: ms);
  }
}

/// 协作式取消令牌。重试循环在每次尝试 / 每次 backoff 之间检查。
class CancelToken {
  final _completer = Completer<void>();
  bool _cancelled = false;

  bool get isCancelled => _cancelled;
  Future<void> get whenCancelled => _completer.future;

  void cancel() {
    if (_cancelled) return;
    _cancelled = true;
    _completer.complete();
  }
}

class RetryCancelledException implements Exception {
  const RetryCancelledException();
  @override
  String toString() => 'RetryCancelledException';
}

/// 执行 `task` 直至成功 / 取消 / 超时 / 耗尽 `policy.maxAttempts`。
///
/// 仅当 `shouldRetry(error)` 返回 true 时重试。调用者应保证 `task` 是幂等的
/// (或携带 idempotency key, 见 Rust 模式 L)。
Future<T> retry<T>(
  Future<T> Function() task, {
  RetryPolicy policy = const RetryPolicy(),
  bool Function(Object error)? shouldRetry,
  CancelToken? cancelToken,
  void Function(int attempt, Object error, Duration delay)? onRetry,
  math.Random? random,
}) async {
  final rng = random ?? math.Random();
  final stopwatch = Stopwatch()..start();
  final deadline = policy.totalTimeout;

  for (var attempt = 1; attempt <= policy.maxAttempts; attempt++) {
    if (cancelToken?.isCancelled ?? false) {
      throw const RetryCancelledException();
    }
    if (deadline != null && stopwatch.elapsed >= deadline) {
      throw TimeoutException('retry exhausted total budget', deadline);
    }
    try {
      return await task();
    } catch (e, st) {
      final retriable = shouldRetry?.call(e) ?? true;
      if (!retriable || attempt == policy.maxAttempts) {
        Error.throwWithStackTrace(e, st);
      }
      final delay = policy.delayFor(attempt, rng);
      if (deadline != null && stopwatch.elapsed + delay >= deadline) {
        throw TimeoutException('retry exhausted total budget', deadline);
      }
      onRetry?.call(attempt, e, delay);
      await _delayOrCancel(delay, cancelToken);
    }
  }
  throw StateError('unreachable: retry loop exited without rethrow');
}

Future<void> _delayOrCancel(Duration d, CancelToken? token) async {
  if (token == null) {
    await Future<void>.delayed(d);
    return;
  }
  final completer = Completer<void>();
  final timer = Timer(d, () {
    if (!completer.isCompleted) completer.complete();
  });
  // ignore: unawaited_futures
  token.whenCancelled.then((_) {
    if (!completer.isCompleted) completer.complete();
  });
  await completer.future;
  timer.cancel();
  if (token.isCancelled) throw const RetryCancelledException();
}
