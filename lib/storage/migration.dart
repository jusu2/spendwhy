/// 模式 M (Dart 侧): Schema 版本化迁移。
///
/// `MigrationRegistry` 维护 `version -> (up, down)` 映射:
/// - `runUp(currentVersion)`: 把数据库从 currentVersion 升到最高已注册版本
/// - `runDown(targetVersion)`: 回滚到 targetVersion
/// - 每次迁移在事务里 + 写 `schema_migrations(version, applied_at_ms)`
///
/// 适用: 复杂 schema 演化, 需要双向 (回滚) 支持。
/// 简单单次升级用 sqflite 内置的 `onUpgrade` 即可。
library;

import 'package:sqflite/sqflite.dart';

typedef MigrationFn = Future<void> Function(Transaction txn);

class StorageSampleMigration {
  final int version;
  final MigrationFn up;
  final MigrationFn? down;
  final String? label;

  const StorageSampleMigration({
    required this.version,
    required this.up,
    this.down,
    this.label,
  });
}

class StorageSampleMigrationRegistry {
  final List<StorageSampleMigration> _all = [];

  void register(StorageSampleMigration m) {
    if (_all.any((x) => x.version == m.version)) {
      throw StateError('duplicate migration version: ${m.version}');
    }
    if (m.version < 1) {
      throw ArgumentError.value(m.version, 'version', 'must be >= 1');
    }
    _all.add(m);
    _all.sort((a, b) => a.version.compareTo(b.version));
  }

  int get latestVersion => _all.isEmpty ? 0 : _all.last.version;

  /// 升级到最高已注册版本。返回执行的 migration 数量。
  Future<int> runUp(Database db) async {
    await _ensureTable(db);
    final current = await _currentVersion(db);
    var applied = 0;
    for (final m in _all.where((m) => m.version > current)) {
      await db.transaction((txn) async {
        await m.up(txn);
        await txn.insert('schema_migrations', {
          'version': m.version,
          'applied_at_ms': DateTime.now().millisecondsSinceEpoch,
          'label': m.label,
        });
      });
      applied++;
    }
    return applied;
  }

  /// 回滚到 `target` (含): 执行所有 version > target 的 down (倒序)。
  Future<int> runDown(Database db, int target) async {
    await _ensureTable(db);
    final current = await _currentVersion(db);
    if (target >= current) return 0;
    final toRollback =
        _all.where((m) => m.version > target && m.version <= current).toList()
          ..sort((a, b) => b.version.compareTo(a.version));
    var done = 0;
    for (final m in toRollback) {
      if (m.down == null) {
        throw StateError(
          'migration v${m.version} has no down(); cannot rollback',
        );
      }
      await db.transaction((txn) async {
        await m.down!(txn);
        await txn.delete(
          'schema_migrations',
          where: 'version = ?',
          whereArgs: [m.version],
        );
      });
      done++;
    }
    return done;
  }

  Future<int> _currentVersion(Database db) async {
    final rows = await db.query(
      'schema_migrations',
      columns: ['MAX(version) AS v'],
    );
    if (rows.isEmpty) return 0;
    return (rows.first['v'] as int?) ?? 0;
  }

  Future<void> _ensureTable(Database db) => db.execute('''
        CREATE TABLE IF NOT EXISTS schema_migrations (
          version INTEGER PRIMARY KEY,
          applied_at_ms INTEGER NOT NULL,
          label TEXT
        )
      ''');
}
