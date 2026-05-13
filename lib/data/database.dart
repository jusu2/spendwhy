import 'dart:async';

import 'package:flutter/foundation.dart';
import 'package:path/path.dart' as p;
import 'package:path_provider/path_provider.dart';
import 'package:sqflite/sqflite.dart';

import '../models/fragment.dart';
import '../models/recovery.dart';
import 'fragment_repository.dart';

/// SQLite 本地数据层。
/// V1 仅做明文 + sqflite，V1.5 之后再加 SQLCipher 或文件级加密。
class AppDatabase implements FragmentRepository {
  AppDatabase._();
  static final AppDatabase instance = AppDatabase._();

  static const _dbName = 'fragments.db';
  static const _dbVersion = 1;

  Database? _db;

  Future<Database> get db async {
    if (_db != null) return _db!;
    final String path;
    if (kIsWeb) {
      path = _dbName;
    } else {
      final dir = await getApplicationDocumentsDirectory();
      path = p.join(dir.path, _dbName);
    }
    _db = await openDatabase(path, version: _dbVersion, onCreate: _onCreate);
    return _db!;
  }

  Future<void> _onCreate(Database db, int version) async {
    await db.execute('''
      CREATE TABLE fragments (
        id TEXT PRIMARY KEY,
        created_at INTEGER NOT NULL,
        content TEXT NOT NULL,
        tags TEXT,
        intensity INTEGER NOT NULL DEFAULT 3,
        stage TEXT NOT NULL DEFAULT 'outburst',
        fade_days INTEGER NOT NULL DEFAULT 270,
        visibility TEXT NOT NULL DEFAULT 'private',
        image_paths TEXT
      );
    ''');
    await db.execute('''
      CREATE INDEX idx_fragments_created_at ON fragments(created_at DESC);
    ''');
    await db.execute('''
      CREATE TABLE recoveries (
        id TEXT PRIMARY KEY,
        created_at INTEGER NOT NULL,
        description TEXT NOT NULL,
        intensity INTEGER NOT NULL DEFAULT 3,
        related_ids TEXT
      );
    ''');
    await db.execute('''
      CREATE INDEX idx_recoveries_created_at ON recoveries(created_at DESC);
    ''');
  }

  // === Fragments ===
  @override
  Future<void> insertFragment(Fragment f) async {
    final database = await db;
    await database.insert(
      'fragments',
      f.toMap(),
      conflictAlgorithm: ConflictAlgorithm.replace,
    );
  }

  @override
  Future<void> updateFragment(Fragment f) async {
    final database = await db;
    await database.update(
      'fragments',
      f.toMap(),
      where: 'id = ?',
      whereArgs: [f.id],
    );
  }

  @override
  Future<void> deleteFragment(String id) async {
    final database = await db;
    await database.delete('fragments', where: 'id = ?', whereArgs: [id]);
  }

  @override
  Future<List<Fragment>> listFragments({int? limit}) async {
    final database = await db;
    final rows = await database.query(
      'fragments',
      orderBy: 'created_at DESC',
      limit: limit,
    );
    return rows.map(Fragment.fromMap).toList();
  }

  @override
  Future<Fragment?> getFragment(String id) async {
    final database = await db;
    final rows = await database.query(
      'fragments',
      where: 'id = ?',
      whereArgs: [id],
    );
    if (rows.isEmpty) return null;
    return Fragment.fromMap(rows.first);
  }

  // === Recoveries ===
  Future<void> insertRecovery(Recovery r) async {
    final database = await db;
    await database.insert(
      'recoveries',
      r.toMap(),
      conflictAlgorithm: ConflictAlgorithm.replace,
    );
  }

  @override
  Future<List<Recovery>> listRecoveries({int? limit}) async {
    final database = await db;
    final rows = await database.query(
      'recoveries',
      orderBy: 'created_at DESC',
      limit: limit,
    );
    return rows.map(Recovery.fromMap).toList();
  }

  @override
  Future<List<Recovery>> recoveriesForFragment(String fragmentId) async {
    final all = await listRecoveries();
    return all.where((r) => r.relatedFragmentIds.contains(fragmentId)).toList();
  }

  // === Cross-table operations ================================================

  /// 在单个事务里写入一条 [recovery] 并把 [advancedFragments] 的 stage 推进。
  ///
  /// 任一步失败都会回滚，避免出现「恢复已记录但相关碎片未推进」之类的半完成状态。
  @override
  Future<void> recordRecoveryTx({
    required Recovery recovery,
    required List<Fragment> advancedFragments,
  }) async {
    final database = await db;
    await database.transaction((txn) async {
      await txn.insert(
        'recoveries',
        recovery.toMap(),
        conflictAlgorithm: ConflictAlgorithm.replace,
      );
      for (final f in advancedFragments) {
        await txn.update(
          'fragments',
          f.toMap(),
          where: 'id = ?',
          whereArgs: [f.id],
        );
      }
    });
  }
}
