/// 错误契约 (Dart 镜像): 与 `rust/src/api/storage/common.rs` 中
/// `StorageErrorCode` 常量逐项对应。
///
/// `StorageError` 本身由 FRB 生成 (含 `code` / `message` 字段);
/// 这里补一个 Dart 端便利访问器, 让业务代码不必到处写 `e.code == 'not_found'`。
library;

import '../src/rust/api/storage/common.dart' show StorageError;

/// 错误码字符串常量。与 Rust 侧 `StorageErrorCode` 字段值保持一致。
abstract final class StorageErrorCodes {
  static const invalidArgument = 'invalid_argument';
  static const notFound = 'not_found';
  static const conflict = 'conflict';
  static const corrupted = 'corrupted';
  static const quotaExceeded = 'quota_exceeded';
  static const internal = 'internal';
}

extension StorageErrorX on StorageError {
  bool get isInvalidArgument => code == StorageErrorCodes.invalidArgument;
  bool get isNotFound => code == StorageErrorCodes.notFound;
  bool get isConflict => code == StorageErrorCodes.conflict;
  bool get isCorrupted => code == StorageErrorCodes.corrupted;
  bool get isQuotaExceeded => code == StorageErrorCodes.quotaExceeded;
  bool get isInternal => code == StorageErrorCodes.internal;

  /// 是否值得重试。`invalidArgument` / `notFound` / `conflict` / `corrupted` 不重试。
  /// `quotaExceeded` 一般也不重试 (需要先腾出空间); 留给调用方判断。
  bool get isRetriable => isInternal;
}
