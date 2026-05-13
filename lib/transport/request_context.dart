/// 模式 V (Dart 侧): 请求横切元数据构建器 + Deadline 管理。
///
/// FRB 生成的 [TransportRequestMeta] 是不可变快照; 本助手提供 ergonomic 构建器:
///
/// - 自动生成 RFC 4122 UUID v4 作 `requestId` (用 `Random.secure`, 无外部依赖)。
/// - [Deadline] 单调时钟 (基于 `Stopwatch`) — 在 retry 时不受系统时钟跳变影响。
/// - 不可变更新: `withAttempt(n)` / `withIdempotency(key)` 返回新副本。
/// - `freeze()` 把当前剩余预算固化为 `budget_ms` 发给 Rust。
///
/// 用法:
/// ```dart
/// final ctx = RequestContext.create(
///   source: 'ui.fragment_list',
///   timeout: const Duration(seconds: 3),
///   idempotencyKey: 'apply-$noteId',
/// );
///
/// final receipt = await retry(
///   () => transportSampleWithMeta(
///     meta: ctx.bumpAttempt().freeze(),
///     payload: payload,
///     workMs: BigInt.from(50),
///   ),
/// );
/// ```
library;

import 'dart:math';

import '../src/rust/api/transport/pattern_v_request_meta.dart'
    show TransportRequestMeta;

export '../src/rust/api/transport/pattern_v_request_meta.dart'
    show TransportRequestMeta, TransportSampleMetaReceiptDto;

/// 单调时钟 deadline; 重试不受 wall-clock 跳变干扰。
class Deadline {
  final Stopwatch _sw;
  final Duration total;

  Deadline(this.total)
      : assert(total > Duration.zero),
        _sw = Stopwatch()..start();

  /// 剩余预算; 已耗尽返回 [Duration.zero]。
  Duration get remaining {
    final r = total - _sw.elapsed;
    return r.isNegative ? Duration.zero : r;
  }

  bool get isExpired => remaining == Duration.zero;

  /// 给 Rust 的 `budget_ms` (上限 u64; 0 → null 让 Rust 判 invalid_argument 之前先抛)。
  int? get remainingMs {
    final ms = remaining.inMilliseconds;
    return ms > 0 ? ms : null;
  }
}

/// 不可变请求上下文。生命周期一般覆盖一次"逻辑请求"含其所有 retry。
class RequestContext {
  final String requestId;
  final String? idempotencyKey;
  final Deadline? deadline;
  final String? traceParent;
  final String? locale;
  final int attempt;
  final String? source;

  const RequestContext._({
    required this.requestId,
    required this.attempt,
    this.idempotencyKey,
    this.deadline,
    this.traceParent,
    this.locale,
    this.source,
  });

  /// 创建新的上下文; 自动分配 [requestId]。
  factory RequestContext.create({
    String? requestId,
    String? idempotencyKey,
    Duration? timeout,
    String? traceParent,
    String? locale,
    String? source,
    Random? random,
  }) {
    return RequestContext._(
      requestId: requestId ?? newRequestId(random: random),
      attempt: 1,
      idempotencyKey: idempotencyKey,
      deadline: timeout == null ? null : Deadline(timeout),
      traceParent: traceParent,
      locale: locale,
      source: source,
    );
  }

  RequestContext withAttempt(int n) {
    if (n < 1) {
      throw ArgumentError.value(n, 'attempt', 'must be >= 1');
    }
    return RequestContext._(
      requestId: requestId,
      attempt: n,
      idempotencyKey: idempotencyKey,
      deadline: deadline,
      traceParent: traceParent,
      locale: locale,
      source: source,
    );
  }

  RequestContext bumpAttempt() => withAttempt(attempt + 1);

  RequestContext withIdempotency(String key) => RequestContext._(
        requestId: requestId,
        attempt: attempt,
        idempotencyKey: key,
        deadline: deadline,
        traceParent: traceParent,
        locale: locale,
        source: source,
      );

  RequestContext withLocale(String loc) => RequestContext._(
        requestId: requestId,
        attempt: attempt,
        idempotencyKey: idempotencyKey,
        deadline: deadline,
        traceParent: traceParent,
        locale: loc,
        source: source,
      );

  RequestContext withTrace(String traceparent) => RequestContext._(
        requestId: requestId,
        attempt: attempt,
        idempotencyKey: idempotencyKey,
        deadline: deadline,
        traceParent: traceparent,
        locale: locale,
        source: source,
      );

  bool get isExpired => deadline?.isExpired ?? false;

  /// 把当前状态固化成 FRB 的 [TransportRequestMeta]; 在 `await rust(...)` 前一刻调。
  ///
  /// 抛 [RequestExpiredException] 若 [deadline] 已到期 — Rust 也会判这个,
  /// 但 Dart 端先抛能省一次 FFI 调用 + 提供更清晰的 stack trace。
  TransportRequestMeta freeze() {
    final remainingMs = deadline?.remainingMs;
    if (deadline != null && remainingMs == null) {
      throw const RequestExpiredException();
    }
    return TransportRequestMeta(
      requestId: requestId,
      idempotencyKey: idempotencyKey,
      budgetMs: remainingMs == null ? null : BigInt.from(remainingMs),
      traceParent: traceParent,
      locale: locale,
      attempt: attempt,
      source: source,
    );
  }
}

/// `freeze()` 在 deadline 到期时抛此异常。业务侧应转 `TransportError.timeout`。
class RequestExpiredException implements Exception {
  const RequestExpiredException();
  @override
  String toString() => 'RequestExpiredException: request deadline already elapsed';
}

/// 生成 RFC 4122 v4 UUID (随机版本)。
///
/// 用 `Random.secure` 取 16 字节, 标记 version=4 / variant=10, 格式化为
/// `8-4-4-4-12` 小写十六进制。不依赖任何外部包。
String newRequestId({Random? random}) {
  final rng = random ?? Random.secure();
  final bytes = List<int>.generate(16, (_) => rng.nextInt(256));
  bytes[6] = (bytes[6] & 0x0f) | 0x40; // version 4
  bytes[8] = (bytes[8] & 0x3f) | 0x80; // variant 1 (10xxxxxx)
  final hex = bytes.map((b) => b.toRadixString(16).padLeft(2, '0')).toList();
  return '${hex.sublist(0, 4).join()}-'
      '${hex.sublist(4, 6).join()}-'
      '${hex.sublist(6, 8).join()}-'
      '${hex.sublist(8, 10).join()}-'
      '${hex.sublist(10, 16).join()}';
}

/// 生成 W3C `traceparent` (`00-{32hex}-{16hex}-01`)。
///
/// `traceId` 16 字节 (32 hex), `spanId` 8 字节 (16 hex), flags 固定 `01` (sampled)。
/// 任一全 0 会被 Rust 端拒绝, 因此重新滚直到非零。
String newTraceParent({Random? random}) {
  final rng = random ?? Random.secure();
  String hex(int n) {
    while (true) {
      final bytes = List<int>.generate(n, (_) => rng.nextInt(256));
      if (bytes.any((b) => b != 0)) {
        return bytes.map((b) => b.toRadixString(16).padLeft(2, '0')).join();
      }
    }
  }

  return '00-${hex(16)}-${hex(8)}-01';
}
