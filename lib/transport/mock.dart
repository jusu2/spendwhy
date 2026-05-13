/// 模式 Q (Dart 侧): 测试时替换 transport 调用。
///
/// 设计取舍: FRB 生成的入口是顶层函数, 不易直接 mockito 替换。
/// 推荐做法是**在你的服务层做接口分层** (业务侧 service 类持有一个抽象,
/// 默认实现转发到 FRB 函数, 测试时换 fake)。
///
/// 本助手提供"轻量 stub 注册表": 适合不想引入完整 DI 容器的单元测试。
/// `withMocks` 提供 per-test 隔离: 保存→替换→执行→恢复。
library;

import 'dart:async';

/// FRB stub 函数签名。FutureOr 的内部值类型在测试侧由调用方约束。
typedef TransportStub = FutureOr<dynamic> Function(List<dynamic> args);

/// 测试用 stub 仓库; 业务代码不应依赖此类。
class TransportMockRegistry {
  static final TransportMockRegistry instance = TransportMockRegistry._();
  TransportMockRegistry._();

  final _stubs = <String, TransportStub>{};

  /// 注册 `name` 对应的 stub。返回被替换前的旧 stub (若无则 `null`)。
  TransportStub? register<T>(
    String name,
    FutureOr<T> Function(List<dynamic> args) stub,
  ) {
    final old = _stubs[name];
    _stubs[name] = stub;
    return old;
  }

  /// 调用 stub; 业务代码在测试模式下用此入口包装真实 FRB 调用。
  ///
  /// 示例:
  /// ```dart
  /// Future<int> add(int a, int b) {
  ///   final stub = TransportMockRegistry.instance.tryInvoke<int>('add', [a, b]);
  ///   if (stub != null) return stub;
  ///   return Future.value(transportSampleAdd(a: a, b: b));
  /// }
  /// ```
  Future<T>? tryInvoke<T>(String name, List<dynamic> args) {
    final stub = _stubs[name];
    if (stub == null) return null;
    return _coerce<T>(name, stub(args));
  }

  static Future<T> _coerce<T>(String name, Object? result) {
    if (result is Future<T>) return result;
    if (result is T) return Future.value(result);
    if (result is Future) {
      return result.then<T>((v) => v as T);
    }
    throw StateError('stub for $name returned wrong type: $result');
  }

  /// 当前已注册的 stub 名 (供测试断言)。
  Iterable<String> get registeredNames => _stubs.keys;

  /// 清空所有注册。多用于 `tearDown`。
  void reset() => _stubs.clear();
}

/// Per-test 隔离 helper: 用给定 stub 临时覆盖现有注册, 执行完毕自动恢复。
///
/// ```dart
/// test('foo uses stub', () => withMocks({
///   'rust.add': (args) => 42,
/// }, () async {
///   expect(await myAdd(1, 2), 42);
/// }));
/// ```
Future<T> withMocks<T>(
  Map<String, FutureOr<dynamic> Function(List<dynamic>)> stubs,
  Future<T> Function() body,
) async {
  final reg = TransportMockRegistry.instance;
  final previous = <String, TransportStub?>{};
  stubs.forEach((name, stub) {
    previous[name] = reg.register(name, stub);
  });
  try {
    return await body();
  } finally {
    previous.forEach((name, old) {
      if (old == null) {
        reg._stubs.remove(name);
      } else {
        reg._stubs[name] = old;
      }
    });
  }
}
