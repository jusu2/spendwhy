/// 错误契约 (`error_contract.dart`) 的纯 Dart 测试。
///
/// 不启动 Rust 桥; 只验证 Dart 端 code → bool 映射符合约定。
/// 重点关注模式 I (字段级加密) 的 `corrupted` 不可重试语义 —
/// AES-GCM tag 不匹配 / 密文被改时 Rust 侧返回 `corrupted`,
/// Dart 侧绝不能错误地认为它可以重试。
library;

import 'package:flutter_test/flutter_test.dart';
import 'package:fragments/src/rust/api/storage/common.dart' show StorageError;
import 'package:fragments/storage/error_contract.dart';

void main() {
  StorageError make(String code) =>
      StorageError(code: code, message: 'test message');

  group('StorageError code classifiers', () {
    test('invalid_argument 不可重试', () {
      final e = make(StorageErrorCodes.invalidArgument);
      expect(e.isInvalidArgument, isTrue);
      expect(e.isRetriable, isFalse);
    });

    test('not_found 不可重试', () {
      final e = make(StorageErrorCodes.notFound);
      expect(e.isNotFound, isTrue);
      expect(e.isRetriable, isFalse);
    });

    test('conflict 不可重试', () {
      final e = make(StorageErrorCodes.conflict);
      expect(e.isConflict, isTrue);
      expect(e.isRetriable, isFalse);
    });

    test('模式 I 关键: corrupted 不可重试 (重试只会再次解密失败)', () {
      final e = make(StorageErrorCodes.corrupted);
      expect(e.isCorrupted, isTrue);
      expect(e.isRetriable, isFalse,
          reason: 'AES-GCM 解密失败重试无意义; 需人工 / 恢复备份');
    });

    test('quota_exceeded 不可重试 (要先腾空间)', () {
      final e = make(StorageErrorCodes.quotaExceeded);
      expect(e.isQuotaExceeded, isTrue);
      expect(e.isRetriable, isFalse);
    });

    test('internal 可重试 (临时 IO 错通常会恢复)', () {
      final e = make(StorageErrorCodes.internal);
      expect(e.isInternal, isTrue);
      expect(e.isRetriable, isTrue);
    });

    test('未知 code 默认不可重试, 也不命中任何 isXxx', () {
      final e = make('something_new');
      expect(e.isInvalidArgument, isFalse);
      expect(e.isNotFound, isFalse);
      expect(e.isConflict, isFalse);
      expect(e.isCorrupted, isFalse);
      expect(e.isQuotaExceeded, isFalse);
      expect(e.isInternal, isFalse);
      expect(e.isRetriable, isFalse);
    });
  });

  group('错误码常量与 Rust 端字符串严格对齐', () {
    // 若 Rust 侧 StorageErrorCode 重命名, 这里要同步; 这组断言是"防漂移".
    test('常量值锁定', () {
      expect(StorageErrorCodes.invalidArgument, 'invalid_argument');
      expect(StorageErrorCodes.notFound, 'not_found');
      expect(StorageErrorCodes.conflict, 'conflict');
      expect(StorageErrorCodes.corrupted, 'corrupted');
      expect(StorageErrorCodes.quotaExceeded, 'quota_exceeded');
      expect(StorageErrorCodes.internal, 'internal');
    });
  });
}
