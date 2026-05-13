/// 模式 N (Dart 侧): Outbox / 离线写队列。
///
/// 把本地操作先写 outbox 表 (作 source of truth), 后台 worker 按顺序消费 →
/// 发到远端。失败按指数退避重试。`idempotency_key` 防重发。
///
/// 适用: 离线优先应用、最终一致同步、不能丢失的客户端发起操作。
/// 不适用: 强一致写 (同步直写); 顺序无关任务 (用消息队列)。
///
/// 表 schema:
/// ```sql
/// CREATE TABLE outbox (
///   id INTEGER PRIMARY KEY AUTOINCREMENT,
///   op TEXT NOT NULL,
///   payload TEXT NOT NULL,
///   idempotency_key TEXT NOT NULL UNIQUE,
///   created_at_ms INTEGER NOT NULL,
///   attempts INTEGER NOT NULL DEFAULT 0,
///   next_retry_at_ms INTEGER NOT NULL,
///   last_error TEXT
/// );
/// ```
library;

import 'package:sqflite/sqflite.dart';

class StorageSampleOutboxItem {
  final int id;
  final String op;
  final String payload;
  final String idempotencyKey;
  final int attempts;
  final int createdAtMs;

  const StorageSampleOutboxItem({
    required this.id,
    required this.op,
    required this.payload,
    required this.idempotencyKey,
    required this.attempts,
    required this.createdAtMs,
  });
}

class StorageSampleOutbox {
  final Database db;
  final String table;
  final int maxAttempts;
  final Duration baseDelay;

  StorageSampleOutbox({
    required this.db,
    this.table = 'outbox',
    this.maxAttempts = 8,
    this.baseDelay = const Duration(seconds: 1),
  });

  Future<void> ensureSchema() => db.execute('''
        CREATE TABLE IF NOT EXISTS $table (
          id INTEGER PRIMARY KEY AUTOINCREMENT,
          op TEXT NOT NULL,
          payload TEXT NOT NULL,
          idempotency_key TEXT NOT NULL UNIQUE,
          created_at_ms INTEGER NOT NULL,
          attempts INTEGER NOT NULL DEFAULT 0,
          next_retry_at_ms INTEGER NOT NULL,
          last_error TEXT
        )
      ''');

  /// 入队; 若 idempotency_key 已存在, 返回 false (不重复入队)。
  Future<bool> enqueue({
    required String op,
    required String payload,
    required String idempotencyKey,
  }) async {
    if (op.isEmpty) throw ArgumentError.value(op, 'op', 'must not be empty');
    if (idempotencyKey.isEmpty) {
      throw ArgumentError.value(
        idempotencyKey,
        'idempotencyKey',
        'must not be empty',
      );
    }
    final now = DateTime.now().millisecondsSinceEpoch;
    try {
      await db.insert(table, {
        'op': op,
        'payload': payload,
        'idempotency_key': idempotencyKey,
        'created_at_ms': now,
        'attempts': 0,
        'next_retry_at_ms': now,
      }, conflictAlgorithm: ConflictAlgorithm.abort);
      return true;
    } on DatabaseException catch (e) {
      if (e.isUniqueConstraintError()) return false;
      rethrow;
    }
  }

  /// 取出最多 `max` 条到期可重试的项。
  Future<List<StorageSampleOutboxItem>> dequeueBatch({int max = 16}) async {
    final now = DateTime.now().millisecondsSinceEpoch;
    final rows = await db.query(
      table,
      where: 'next_retry_at_ms <= ? AND attempts < ?',
      whereArgs: [now, maxAttempts],
      orderBy: 'id ASC',
      limit: max,
    );
    return rows.map((r) => StorageSampleOutboxItem(
          id: r['id'] as int,
          op: r['op'] as String,
          payload: r['payload'] as String,
          idempotencyKey: r['idempotency_key'] as String,
          attempts: r['attempts'] as int,
          createdAtMs: r['created_at_ms'] as int,
        )).toList(growable: false);
  }

  /// 成功 → 删除。
  Future<void> ack(int id) => db.delete(table, where: 'id = ?', whereArgs: [id]);

  /// 失败 → 增加 attempts + 计算下次重试时间 (指数退避 + 抖动)。
  Future<void> nack(int id, String error) async {
    final rows = await db
        .query(table, columns: ['attempts'], where: 'id = ?', whereArgs: [id]);
    if (rows.isEmpty) return;
    final attempts = (rows.first['attempts'] as int) + 1;
    final delayMs = baseDelay.inMilliseconds *
        (1 << (attempts - 1).clamp(0, 12)); // cap at base*4096
    final now = DateTime.now().millisecondsSinceEpoch;
    await db.update(
      table,
      {
        'attempts': attempts,
        'next_retry_at_ms': now + delayMs,
        'last_error': error,
      },
      where: 'id = ?',
      whereArgs: [id],
    );
  }

  Future<int> deadLetterCount() async {
    final rows = await db.rawQuery(
      'SELECT COUNT(*) AS c FROM $table WHERE attempts >= ?',
      [maxAttempts],
    );
    return (rows.first['c'] as int?) ?? 0;
  }
}
