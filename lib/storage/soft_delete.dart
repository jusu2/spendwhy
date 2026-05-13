/// 模式 Q (Dart 侧): 软删 + tombstone GC。
///
/// 在表上加 `deleted_at_ms INTEGER NULL`; 删除 = 置 `deleted_at_ms = now`,
/// 不实际 DELETE。这样:
/// - 撤销: 把 deleted_at_ms 置 NULL。
/// - 同步: 远端能看到"已删除"的 tombstone, 不会"幽灵复活"。
/// - GC: 老到某阈值的 tombstone 才真删。
///
/// 适用: 需要 undo / 同步 / 合规留存的表。
/// 不适用: 不能容忍存储增长 (用硬删); 实时排他 (deleted_at + unique 索引冲突)。
library;

import 'package:sqflite/sqflite.dart';

class StorageSampleSoftDelete {
  final DatabaseExecutor db;
  final String table;
  final String idColumn;
  final String deletedAtColumn;

  StorageSampleSoftDelete({
    required this.db,
    required this.table,
    this.idColumn = 'id',
    this.deletedAtColumn = 'deleted_at_ms',
  });

  /// 软删 (idempotent: 已删的再删不报错)。返回受影响行数。
  Future<int> softDelete(Object id) {
    final now = DateTime.now().millisecondsSinceEpoch;
    return db.update(
      table,
      {deletedAtColumn: now},
      where: '$idColumn = ? AND $deletedAtColumn IS NULL',
      whereArgs: [id],
    );
  }

  /// 撤销软删。
  Future<int> undoDelete(Object id) {
    return db.update(
      table,
      {deletedAtColumn: null},
      where: '$idColumn = ? AND $deletedAtColumn IS NOT NULL',
      whereArgs: [id],
    );
  }

  /// 列出所有"已软删且 deleted_at_ms < beforeMs"的行 — 给 GC 用。
  Future<List<Map<String, Object?>>> listDeletedBefore(int beforeMs) {
    return db.query(
      table,
      where: '$deletedAtColumn IS NOT NULL AND $deletedAtColumn < ?',
      whereArgs: [beforeMs],
    );
  }

  /// 真删: 把保留期外的 tombstone 物理删除。返回删除行数。
  Future<int> gcOlderThan(Duration retention) {
    final cutoff =
        DateTime.now().millisecondsSinceEpoch - retention.inMilliseconds;
    return db.delete(
      table,
      where: '$deletedAtColumn IS NOT NULL AND $deletedAtColumn < ?',
      whereArgs: [cutoff],
    );
  }

  /// 给业务查询用的 WHERE 片段, 自动排除 tombstone。
  /// 例: `db.query(table, where: softDelete.activeWhereClause)`。
  String get activeWhereClause => '$deletedAtColumn IS NULL';
}
