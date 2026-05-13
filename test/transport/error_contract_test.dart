/// 错误契约辅助 (`error_contract.dart`) 的纯 Dart 测试。
///
/// 这些测试不启动 Rust 桥, 只验证 Dart 端的 code → bool 映射符合约定。

import 'package:flutter_test/flutter_test.dart';
import 'package:fragments/transport/error_contract.dart';
import 'package:fragments/src/rust/api/transport/common.dart' show TransportError;

void main() {
  group('TransportError code classifiers', () {
    TransportError make(String code) =>
        TransportError(code: code, message: 'test', elapsedMs: BigInt.zero);

    test('canceled 不可重试', () {
      final e = make(TransportErrorCodes.canceled);
      expect(e.isCanceled, isTrue);
      expect(e.isRetriable, isFalse);
    });

    test('timeout 可重试', () {
      final e = make(TransportErrorCodes.timeout);
      expect(e.isTimeout, isTrue);
      expect(e.isRetriable, isTrue);
    });

    test('invalid_argument / not_found / conflict 不可重试', () {
      expect(make(TransportErrorCodes.invalidArgument).isRetriable, isFalse);
      expect(make(TransportErrorCodes.notFound).isRetriable, isFalse);
      expect(make(TransportErrorCodes.conflict).isRetriable, isFalse);
    });

    test('internal 可重试 (默认策略, 业务侧可覆盖)', () {
      expect(make(TransportErrorCodes.internal).isRetriable, isTrue);
    });
  });
}
