/// 模式 P (Dart 侧): 幂等批量 backfill。
///
/// 给一个数据集 (List<T> 或 cursor-based stream), 按 batch 执行业务函数 +
/// 持久化 cursor (每批后), 实现"断电可续跑"。
///
/// 适用: schema 迁移后补算字段、跨表数据补齐、首次同步。
/// 不适用: 在线写路径 (这里假设批处理, 慢但稳)。
///
/// 关键约束:
/// - 业务函数必须**幂等** (重复执行同一 item 不出错): 因为 cursor 持久化在
///   "每批后", 中途崩溃会导致最后一批重跑。
library;

import 'dart:async';

import 'package:sqflite/sqflite.dart';

class StorageSampleBackfillStatsDto {
  final int processed;
  final int batches;
  final int durationMs;
  const StorageSampleBackfillStatsDto({
    required this.processed,
    required this.batches,
    required this.durationMs,
  });
}

class StorageSampleBackfillRunner<T> {
  final Database db;
  final String table; // checkpoint 表 (任务 id 唯一)
  final String jobId;
  final int batchSize;

  StorageSampleBackfillRunner({
    required this.db,
    required this.jobId,
    this.table = 'backfill_checkpoints',
    this.batchSize = 100,
  })  : assert(batchSize > 0);

  Future<void> ensureSchema() => db.execute('''
        CREATE TABLE IF NOT EXISTS $table (
          job_id TEXT PRIMARY KEY,
          cursor INTEGER NOT NULL,
          updated_at_ms INTEGER NOT NULL
        )
      ''');

  Future<int> currentCursor() async {
    final rows = await db.query(
      table,
      columns: ['cursor'],
      where: 'job_id = ?',
      whereArgs: [jobId],
    );
    if (rows.isEmpty) return 0;
    return (rows.first['cursor'] as int?) ?? 0;
  }

  Future<void> _saveCursor(int c) async {
    await db.insert(
      table,
      {
        'job_id': jobId,
        'cursor': c,
        'updated_at_ms': DateTime.now().millisecondsSinceEpoch,
      },
      conflictAlgorithm: ConflictAlgorithm.replace,
    );
  }

  /// 跑一次 backfill 直到 `fetch` 返回空。
  ///
  /// - `fetch(offset, limit)`: 返回一批 (offset 起 limit 条; 不足则少返回)。
  /// - `handle(batch, txn)`: 在事务里处理一批; 抛错则该批回滚, cursor 不前进。
  Future<StorageSampleBackfillStatsDto> run({
    required Future<List<T>> Function(int offset, int limit) fetch,
    required Future<void> Function(List<T> batch, Transaction txn) handle,
  }) async {
    await ensureSchema();
    final sw = Stopwatch()..start();
    var processed = 0;
    var batches = 0;
    var cursor = await currentCursor();
    while (true) {
      final batch = await fetch(cursor, batchSize);
      if (batch.isEmpty) break;
      // 抛错: 不 catch — 由调用方决定重试; cursor 不前进, 下次从同位置续跑。
      await db.transaction((txn) async {
        await handle(batch, txn);
      });
      cursor += batch.length;
      await _saveCursor(cursor);
      processed += batch.length;
      batches++;
    }
    sw.stop();
    return StorageSampleBackfillStatsDto(
      processed: processed,
      batches: batches,
      durationMs: sw.elapsedMilliseconds,
    );
  }

  /// 重置: 把 cursor 置 0 (强制下次重头跑)。
  Future<void> reset() async {
    await db.delete(table, where: 'job_id = ?', whereArgs: [jobId]);
  }
}
