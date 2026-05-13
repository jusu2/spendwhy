/// 模式 L (Dart 侧): SQL 事务 + savepoint 助手。
///
/// 包 [sqflite] 的 `transaction()`, 提供:
/// - 统一的 `inTx<T>` 入口, 错误时自动 rollback
/// - 嵌套事务 = savepoint (sqflite 内部已支持, 这里再加显式 API)
/// - 可选超时 (超时后取消, 但 sqflite 本身不支持中断, 仅在外层拒绝继续)
///
/// 适用: 多表更新需要原子性 (例如 fragments + event_log 同时写)。
/// 不适用: 跨 db 事务 (sqflite 单连接, 跨 db 用 2PC 但 sqflite 不支持)。
library;

import 'dart:async';

import 'package:sqflite/sqflite.dart';

/// 在事务中运行 `body`。任何抛出 → rollback。
///
/// `timeout` 仅在事务开始**之前**起作用 — 一旦 sqflite 拿到锁, 无法主动中断。
/// 这里的实现是: 用 `Future.any` 等到 body 完成或超时, 超时后让事务继续但
/// 调用方收到 [TimeoutException]; rollback 由 sqflite 的"未提交即丢弃"语义兜底。
Future<T> inTx<T>(
  DatabaseExecutor db,
  Future<T> Function(Transaction txn) body, {
  Duration? timeout,
  bool? exclusive,
}) async {
  Future<T> run() {
    if (db is Transaction) {
      // 已经在事务中: 用 savepoint 模拟嵌套。
      return _withSavepoint(db, body);
    }
    return (db as Database)
        .transaction((txn) => body(txn), exclusive: exclusive);
  }

  if (timeout == null) return run();
  return run().timeout(timeout);
}

int _savepointCounter = 0;

Future<T> _withSavepoint<T>(
  Transaction outer,
  Future<T> Function(Transaction) body,
) async {
  final id = ++_savepointCounter;
  final name = 'sp_$id';
  await outer.execute('SAVEPOINT $name');
  try {
    final r = await body(outer);
    await outer.execute('RELEASE SAVEPOINT $name');
    return r;
  } catch (_) {
    await outer.execute('ROLLBACK TO SAVEPOINT $name');
    await outer.execute('RELEASE SAVEPOINT $name');
    rethrow;
  }
}
