/// 模式 O: 在后台 Isolate 中调 Rust。
///
/// 由于 FRB 的桥默认绑定到 main isolate, 子 isolate 必须先 `await RustLib.init()`,
/// 然后才能调用 Rust 函数。这个文件提供一个 `runRustInIsolate` 助手。
///
/// 用法:
/// ```dart
/// final result = await runRustInIsolate(
///   const _ExpensiveComputation('payload'),
/// );
///
/// // 任务函数必须是 top-level 或 static, 因为要被序列化到子 isolate。
/// class _ExpensiveComputation extends RustIsolateTask<String> {
///   final String input;
///   const _ExpensiveComputation(this.input);
///   @override
///   Future<String> run() => transportSampleCompute(input: input);
/// }
/// ```
///
/// 何时不用:
/// - 任务总耗时 < 50ms: isolate 启动 + 桥重初始化开销得不偿失。
/// - 任务流式 (Stream): 子 isolate 的 StreamSink 跨 isolate 还不稳, 留在主 isolate。
library;

import 'package:flutter/foundation.dart';

import '../src/rust/frb_generated.dart';

/// 任务接口: 子类实现 `run()`。必须是顶层 const 类或被注解 `@immutable`,
/// 以便 `compute()` 跨 isolate 复制。
abstract class RustIsolateTask<T> {
  const RustIsolateTask();
  Future<T> run();
}

Future<T> runRustInIsolate<T>(RustIsolateTask<T> task) {
  return compute<RustIsolateTask<T>, T>(_isolateEntry<T>, task);
}

Future<T> _isolateEntry<T>(RustIsolateTask<T> task) async {
  // RustLib.init() 在 FRB 中是幂等的; 重复调用 (尤其在子 isolate) 是安全惯用法。
  await RustLib.init();
  return task.run();
}
