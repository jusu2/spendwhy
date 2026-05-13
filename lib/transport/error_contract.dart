/// 错误契约 (Dart 镜像): 与 `rust/src/api/transport/common.rs` 中
/// `TransportErrorCode` 常量逐项对应。
///
/// `TransportError` 本身由 FRB 生成 (含 `code` / `message` / `elapsedMs` 字段),
/// 这里只补一个 Dart 端便利访问器 + 重试判定。
library;

import '../src/rust/api/transport/common.dart' show TransportError;

/// 错误码字符串常量。与 Rust 侧 `TransportErrorCode` 字段值保持一致。
abstract final class TransportErrorCodes {
  static const invalidArgument = 'invalid_argument';
  static const notFound = 'not_found';
  static const conflict = 'conflict';
  static const canceled = 'canceled';
  static const timeout = 'timeout';
  static const internal = 'internal';
}

extension TransportErrorX on TransportError {
  bool get isCanceled => code == TransportErrorCodes.canceled;
  bool get isTimeout => code == TransportErrorCodes.timeout;
  bool get isInvalidArgument => code == TransportErrorCodes.invalidArgument;
  bool get isNotFound => code == TransportErrorCodes.notFound;
  bool get isConflict => code == TransportErrorCodes.conflict;
  bool get isInternal => code == TransportErrorCodes.internal;

  /// 是否值得重试。`canceled` / `invalidArgument` / `notFound` / `conflict` 不重试。
  bool get isRetriable => isTimeout || isInternal;
}
