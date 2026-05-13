import 'dart:async';
import 'dart:convert';

import 'package:flutter/foundation.dart';
import 'package:path/path.dart' as p;
import 'package:path_provider/path_provider.dart';
import 'package:sqflite/sqflite.dart';

import '../models/enums.dart';
import '../models/fragment.dart';
import '../models/recovery.dart';
import 'domain_event.dart';
import 'fragment_repository.dart';

/// SQLite 本地数据层。
///
/// **v2 schema**（ADR-0004 + ADR-0005，预留 ADR-0002 加密列）：
/// - STRICT 表 + CHECK 约束 + 外键 ON DELETE CASCADE
/// - `fragment_tag` / `fragment_image` / `recovery_fragment_link` 三张多对多表
/// - 软删（`deleted_at`）+ 乐观锁（`revision`）+ `updated_at`
/// - `event_log` append-only 表
/// - `crypto_meta` + `content_cipher/content_nonce/content_key_id` 预留列（暂不写）
/// - `schema_migrations` 自描述迁移注册表
class AppDatabase implements FragmentRepository {
  AppDatabase._();
  static final AppDatabase instance = AppDatabase._();

  static const _dbName = 'fragments.db';
  static const _dbVersion = 2;

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
    _db = await openDatabase(
      path,
      version: _dbVersion,
      onConfigure: _onConfigure,
      onCreate: _onCreate,
      onUpgrade: _onUpgrade,
    );
    return _db!;
  }

  /// 启用外键级联 — SQLite 默认关闭。
  Future<void> _onConfigure(Database db) async {
    await db.execute('PRAGMA foreign_keys = ON');
  }

  Future<void> _onCreate(Database db, int version) async {
    await db.transaction((txn) async {
      await _createSchemaV2(txn);
      await txn.insert('schema_migrations', {
        'version': 2,
        'applied_at': DateTime.now().toUtc().millisecondsSinceEpoch,
      });
    });
  }

  /// v1 → v2 数据迁移。
  ///
  /// v1 把 tags / image_paths / related_ids 用分隔符塞进 TEXT 列，
  /// v2 把它们拆到 normalized 表，并加约束/审计/事件列。
  Future<void> _onUpgrade(Database db, int oldVersion, int newVersion) async {
    if (oldVersion < 2 && newVersion >= 2) {
      await _migrateV1ToV2(db);
    }
  }

  Future<void> _migrateV1ToV2(Database db) async {
    final nowMs = DateTime.now().toUtc().millisecondsSinceEpoch;
    await db.transaction((txn) async {
      // 1. 读出 v1 主表数据（fragments / recoveries 仍是旧形态）。
      final oldFragments = await txn.query('fragments');
      final oldRecoveries = await txn.query('recoveries');

      // 2. 拆掉旧索引（重建表前必须 drop，否则名称冲突）。
      await txn.execute('DROP INDEX IF EXISTS idx_fragments_created_at');
      await txn.execute('DROP INDEX IF EXISTS idx_recoveries_created_at');
      await txn.execute('ALTER TABLE fragments RENAME TO _fragments_v1');
      await txn.execute('ALTER TABLE recoveries RENAME TO _recoveries_v1');

      // 3. 建 v2 完整 schema。
      await _createSchemaV2(txn);

      // 4. 搬数据。
      for (final row in oldFragments) {
        final id = row['id'] as String;
        final createdAt = row['created_at'] as int;
        await txn.insert('fragments', {
          'id': id,
          'created_at': createdAt,
          'updated_at': createdAt,
          'deleted_at': null,
          'revision': 1,
          'content': (row['content'] as String?) ?? '',
          'intensity': (row['intensity'] as int?) ?? 3,
          'stage': (row['stage'] as String?) ?? 'outburst',
          'fade_days': (row['fade_days'] as int?) ?? 270,
          'visibility': (row['visibility'] as String?) ?? 'private',
        });
        final tagsRaw = (row['tags'] as String?) ?? '';
        if (tagsRaw.isNotEmpty) {
          for (final code in tagsRaw.split(',').where((s) => s.isNotEmpty)) {
            await txn.insert('fragment_tag', {
              'fragment_id': id,
              'tag_code': code,
            }, conflictAlgorithm: ConflictAlgorithm.ignore);
          }
        }
        final imagesRaw = (row['image_paths'] as String?) ?? '';
        if (imagesRaw.isNotEmpty) {
          final paths = imagesRaw
              .split('|')
              .where((s) => s.isNotEmpty)
              .toList();
          for (var i = 0; i < paths.length; i++) {
            await txn.insert('fragment_image', {
              'fragment_id': id,
              'ordinal': i,
              'path': paths[i],
            }, conflictAlgorithm: ConflictAlgorithm.ignore);
          }
        }
      }

      for (final row in oldRecoveries) {
        final id = row['id'] as String;
        final createdAt = row['created_at'] as int;
        await txn.insert('recoveries', {
          'id': id,
          'created_at': createdAt,
          'updated_at': createdAt,
          'deleted_at': null,
          'revision': 1,
          'description': (row['description'] as String?) ?? '',
          'intensity': (row['intensity'] as int?) ?? 3,
        });
        final relatedRaw = (row['related_ids'] as String?) ?? '';
        if (relatedRaw.isNotEmpty) {
          for (final fid in relatedRaw.split(',').where((s) => s.isNotEmpty)) {
            // FK 校验：旧数据可能引用已删 fragment，落 link 前先看是否存在。
            final exists = await txn.query(
              'fragments',
              columns: ['id'],
              where: 'id = ?',
              whereArgs: [fid],
              limit: 1,
            );
            if (exists.isEmpty) continue;
            await txn.insert(
              'recovery_fragment_link',
              {'recovery_id': id, 'fragment_id': fid},
              conflictAlgorithm: ConflictAlgorithm.ignore,
            );
          }
        }
      }

      // 5. 删旧表。
      await txn.execute('DROP TABLE _fragments_v1');
      await txn.execute('DROP TABLE _recoveries_v1');

      // 6. 注册迁移版本。
      await txn.insert('schema_migrations', {
        'version': 2,
        'applied_at': nowMs,
      });
    });
  }

  Future<void> _createSchemaV2(DatabaseExecutor t) async {
    // schema_migrations
    await t.execute('''
      CREATE TABLE IF NOT EXISTS schema_migrations (
        version    INTEGER PRIMARY KEY,
        applied_at INTEGER NOT NULL
      ) STRICT
    ''');

    // fragments
    await t.execute('''
      CREATE TABLE fragments (
        id              TEXT    PRIMARY KEY NOT NULL,
        created_at      INTEGER NOT NULL,
        updated_at      INTEGER NOT NULL,
        deleted_at      INTEGER,
        revision        INTEGER NOT NULL DEFAULT 1,
        content         TEXT    NOT NULL DEFAULT '',
        intensity       INTEGER NOT NULL CHECK(intensity BETWEEN 1 AND 5),
        stage           TEXT    NOT NULL CHECK(stage IN ('outburst','recovery','relapse')),
        fade_days       INTEGER NOT NULL CHECK(fade_days > 0),
        visibility      TEXT    NOT NULL CHECK(visibility IN ('private','anonymous')),
        content_cipher  BLOB,
        content_nonce   BLOB,
        content_key_id  TEXT
      ) STRICT
    ''');
    await t.execute(
      'CREATE INDEX idx_fragments_created_at ON fragments(created_at DESC)',
    );
    await t.execute(
      'CREATE INDEX idx_fragments_alive ON fragments(deleted_at) WHERE deleted_at IS NULL',
    );

    // recoveries
    await t.execute('''
      CREATE TABLE recoveries (
        id              TEXT    PRIMARY KEY NOT NULL,
        created_at      INTEGER NOT NULL,
        updated_at      INTEGER NOT NULL,
        deleted_at      INTEGER,
        revision        INTEGER NOT NULL DEFAULT 1,
        description     TEXT    NOT NULL DEFAULT '',
        intensity       INTEGER NOT NULL CHECK(intensity BETWEEN 1 AND 5),
        content_cipher  BLOB,
        content_nonce   BLOB,
        content_key_id  TEXT
      ) STRICT
    ''');
    await t.execute(
      'CREATE INDEX idx_recoveries_created_at ON recoveries(created_at DESC)',
    );

    // fragment_tag (M2M)
    await t.execute('''
      CREATE TABLE fragment_tag (
        fragment_id TEXT NOT NULL,
        tag_code    TEXT NOT NULL,
        PRIMARY KEY (fragment_id, tag_code),
        FOREIGN KEY (fragment_id) REFERENCES fragments(id) ON DELETE CASCADE
      ) STRICT
    ''');
    await t.execute(
      'CREATE INDEX idx_fragment_tag_tag ON fragment_tag(tag_code)',
    );

    // fragment_image (有序)
    await t.execute('''
      CREATE TABLE fragment_image (
        fragment_id TEXT    NOT NULL,
        ordinal     INTEGER NOT NULL,
        path        TEXT    NOT NULL,
        PRIMARY KEY (fragment_id, ordinal),
        FOREIGN KEY (fragment_id) REFERENCES fragments(id) ON DELETE CASCADE
      ) STRICT
    ''');

    // recovery <-> fragment 关联
    await t.execute('''
      CREATE TABLE recovery_fragment_link (
        recovery_id TEXT NOT NULL,
        fragment_id TEXT NOT NULL,
        PRIMARY KEY (recovery_id, fragment_id),
        FOREIGN KEY (recovery_id) REFERENCES recoveries(id) ON DELETE CASCADE,
        FOREIGN KEY (fragment_id) REFERENCES fragments(id) ON DELETE CASCADE
      ) STRICT
    ''');
    await t.execute(
      'CREATE INDEX idx_rfl_fragment ON recovery_fragment_link(fragment_id)',
    );

    // 事件日志（ADR-0005）
    await t.execute('''
      CREATE TABLE event_log (
        seq          INTEGER PRIMARY KEY AUTOINCREMENT,
        occurred_at  INTEGER NOT NULL,
        event_type   TEXT    NOT NULL,
        aggregate_id TEXT    NOT NULL,
        payload      TEXT    NOT NULL
      ) STRICT
    ''');
    await t.execute(
      'CREATE INDEX idx_event_log_aggregate ON event_log(aggregate_id)',
    );
    await t.execute(
      'CREATE INDEX idx_event_log_occurred ON event_log(occurred_at DESC)',
    );

    // 加密元数据（ADR-0002 预留）
    await t.execute('''
      CREATE TABLE crypto_meta (
        key_id     TEXT    PRIMARY KEY NOT NULL,
        created_at INTEGER NOT NULL,
        status     TEXT    NOT NULL CHECK(status IN ('active','rotating','retired')),
        algorithm  TEXT    NOT NULL
      ) STRICT
    ''');
  }

  // === Fragments ============================================================

  @override
  Future<void> insertFragment(Fragment f) async {
    final database = await db;
    final nowMs = DateTime.now().toUtc().millisecondsSinceEpoch;
    await database.transaction((txn) async {
      await txn.insert('fragments', _fragmentRow(f, nowMs: nowMs, revision: 1));
      await _replaceTags(txn, f);
      await _replaceImages(txn, f);
      await _appendEvent(
        txn,
        occurredAt: nowMs,
        type: DomainEventType.fragmentCreated,
        aggregateId: f.id,
        payload: _fragmentPayload(f),
      );
    });
  }

  @override
  Future<void> updateFragment(Fragment f) async {
    final database = await db;
    final nowMs = DateTime.now().toUtc().millisecondsSinceEpoch;
    await database.transaction((txn) async {
      // 读旧 revision + stage（用于乐观锁和 stage-advanced 事件）。
      final old = await txn.query(
        'fragments',
        columns: ['revision', 'stage'],
        where: 'id = ? AND deleted_at IS NULL',
        whereArgs: [f.id],
        limit: 1,
      );
      if (old.isEmpty) {
        throw StateError('updateFragment: id ${f.id} not found or deleted');
      }
      final oldRevision = old.first['revision'] as int;
      final oldStage = old.first['stage'] as String;

      final updated = await txn.update(
        'fragments',
        {
          ..._fragmentRow(f, nowMs: nowMs, revision: oldRevision + 1),
          // created_at 不可改：用 UPDATE 列表里没它即可，但我们的 row 里包含——剔除。
        }..remove('created_at'),
        where: 'id = ? AND revision = ?',
        whereArgs: [f.id, oldRevision],
      );
      if (updated == 0) {
        throw StateError(
          'updateFragment: optimistic lock conflict on ${f.id} '
          '(expected revision $oldRevision)',
        );
      }
      await _replaceTags(txn, f);
      await _replaceImages(txn, f);
      await _appendEvent(
        txn,
        occurredAt: nowMs,
        type: DomainEventType.fragmentEdited,
        aggregateId: f.id,
        payload: _fragmentPayload(f),
      );
      if (oldStage != f.stage.code) {
        await _appendEvent(
          txn,
          occurredAt: nowMs,
          type: DomainEventType.fragmentStageAdvanced,
          aggregateId: f.id,
          payload: {'from': oldStage, 'to': f.stage.code},
        );
      }
    });
  }

  @override
  Future<void> deleteFragment(String id) async {
    final database = await db;
    final nowMs = DateTime.now().toUtc().millisecondsSinceEpoch;
    await database.transaction((txn) async {
      final n = await txn.update(
        'fragments',
        {'deleted_at': nowMs, 'updated_at': nowMs},
        where: 'id = ? AND deleted_at IS NULL',
        whereArgs: [id],
      );
      if (n == 0) return; // 已删或不存在，幂等。
      await _appendEvent(
        txn,
        occurredAt: nowMs,
        type: DomainEventType.fragmentDeleted,
        aggregateId: id,
        payload: const {},
      );
    });
  }

  @override
  Future<List<Fragment>> listFragments({int? limit}) async {
    final database = await db;
    final rows = await database.query(
      'fragments',
      where: 'deleted_at IS NULL',
      orderBy: 'created_at DESC',
      limit: limit,
    );
    if (rows.isEmpty) return const [];
    final ids = rows.map((r) => r['id'] as String).toList();
    final tags = await _loadTags(database, ids);
    final images = await _loadImages(database, ids);
    return [
      for (final r in rows)
        _hydrateFragment(
          r,
          tags[r['id']] ?? const <FragmentTag>[],
          images[r['id']] ?? const <String>[],
        ),
    ];
  }

  @override
  Future<Fragment?> getFragment(String id) async {
    final database = await db;
    final rows = await database.query(
      'fragments',
      where: 'id = ? AND deleted_at IS NULL',
      whereArgs: [id],
      limit: 1,
    );
    if (rows.isEmpty) return null;
    final tags = await _loadTags(database, [id]);
    final images = await _loadImages(database, [id]);
    return _hydrateFragment(
      rows.first,
      tags[id] ?? const <FragmentTag>[],
      images[id] ?? const <String>[],
    );
  }

  // === Recoveries ===========================================================

  @override
  Future<List<Recovery>> listRecoveries({int? limit}) async {
    final database = await db;
    final rows = await database.query(
      'recoveries',
      where: 'deleted_at IS NULL',
      orderBy: 'created_at DESC',
      limit: limit,
    );
    if (rows.isEmpty) return const [];
    final ids = rows.map((r) => r['id'] as String).toList();
    final links = await _loadRecoveryLinks(database, ids);
    return [
      for (final r in rows)
        _hydrateRecovery(r, links[r['id']] ?? const <String>[]),
    ];
  }

  @override
  Future<List<Recovery>> recoveriesForFragment(String fragmentId) async {
    final database = await db;
    final rows = await database.rawQuery(
      '''
      SELECT r.* FROM recoveries r
      INNER JOIN recovery_fragment_link l ON l.recovery_id = r.id
      WHERE l.fragment_id = ? AND r.deleted_at IS NULL
      ORDER BY r.created_at DESC
      ''',
      [fragmentId],
    );
    if (rows.isEmpty) return const [];
    final ids = rows.map((r) => r['id'] as String).toList();
    final links = await _loadRecoveryLinks(database, ids);
    return [
      for (final r in rows)
        _hydrateRecovery(r, links[r['id']] ?? const <String>[]),
    ];
  }

  // === Cross-table 事务 =====================================================

  @override
  Future<void> recordRecoveryTx({
    required Recovery recovery,
    required List<Fragment> advancedFragments,
  }) async {
    final database = await db;
    final nowMs = DateTime.now().toUtc().millisecondsSinceEpoch;
    await database.transaction((txn) async {
      await txn.insert(
        'recoveries',
        _recoveryRow(recovery, nowMs: nowMs, revision: 1),
        conflictAlgorithm: ConflictAlgorithm.replace,
      );
      // 重建关联（先删后插，简单且与 v2 schema 的 PK 约束兼容）。
      await txn.delete(
        'recovery_fragment_link',
        where: 'recovery_id = ?',
        whereArgs: [recovery.id],
      );
      for (final fid in recovery.relatedFragmentIds) {
        await txn.insert('recovery_fragment_link', {
          'recovery_id': recovery.id,
          'fragment_id': fid,
        }, conflictAlgorithm: ConflictAlgorithm.ignore);
      }

      for (final f in advancedFragments) {
        final old = await txn.query(
          'fragments',
          columns: ['revision', 'stage'],
          where: 'id = ? AND deleted_at IS NULL',
          whereArgs: [f.id],
          limit: 1,
        );
        if (old.isEmpty) continue; // 跳过已删
        final oldRevision = old.first['revision'] as int;
        final oldStage = old.first['stage'] as String;
        final n = await txn.update(
          'fragments',
          {..._fragmentRow(f, nowMs: nowMs, revision: oldRevision + 1)}
            ..remove('created_at'),
          where: 'id = ? AND revision = ?',
          whereArgs: [f.id, oldRevision],
        );
        if (n == 0) {
          throw StateError(
            'recordRecoveryTx: optimistic lock conflict on ${f.id}',
          );
        }
        await _replaceTags(txn, f);
        await _replaceImages(txn, f);
        if (oldStage != f.stage.code) {
          await _appendEvent(
            txn,
            occurredAt: nowMs,
            type: DomainEventType.fragmentStageAdvanced,
            aggregateId: f.id,
            payload: {'from': oldStage, 'to': f.stage.code},
          );
        }
      }

      await _appendEvent(
        txn,
        occurredAt: nowMs,
        type: DomainEventType.recoveryRecorded,
        aggregateId: recovery.id,
        payload: _recoveryPayload(recovery),
      );
    });
  }

  // === 事件日志 =============================================================

  @override
  Future<List<DomainEvent>> listEvents({
    String? aggregateId,
    int? limit,
  }) async {
    final database = await db;
    final rows = await database.query(
      'event_log',
      where: aggregateId == null ? null : 'aggregate_id = ?',
      whereArgs: aggregateId == null ? null : [aggregateId],
      orderBy: 'seq ASC',
      limit: limit,
    );
    return [
      for (final r in rows)
        DomainEvent(
          seq: r['seq'] as int,
          occurredAt: DateTime.fromMillisecondsSinceEpoch(
            r['occurred_at'] as int,
            isUtc: true,
          ),
          eventType: r['event_type'] as String,
          aggregateId: r['aggregate_id'] as String,
          payload: jsonDecode(r['payload'] as String) as Map<String, Object?>,
        ),
    ];
  }

  // === 内部 helpers =========================================================

  Map<String, Object?> _fragmentRow(
    Fragment f, {
    required int nowMs,
    required int revision,
  }) {
    return {
      'id': f.id,
      'created_at': f.createdAt.toUtc().millisecondsSinceEpoch,
      'updated_at': nowMs,
      'deleted_at': null,
      'revision': revision,
      'content': f.content,
      'intensity': f.intensity.value,
      'stage': f.stage.code,
      'fade_days': f.fadePeriod.days,
      'visibility': f.visibility.code,
    };
  }

  Map<String, Object?> _recoveryRow(
    Recovery r, {
    required int nowMs,
    required int revision,
  }) {
    return {
      'id': r.id,
      'created_at': r.createdAt.toUtc().millisecondsSinceEpoch,
      'updated_at': nowMs,
      'deleted_at': null,
      'revision': revision,
      'description': r.description,
      'intensity': r.intensity.value,
    };
  }

  Future<void> _replaceTags(DatabaseExecutor txn, Fragment f) async {
    await txn.delete(
      'fragment_tag',
      where: 'fragment_id = ?',
      whereArgs: [f.id],
    );
    for (final tag in f.tags) {
      await txn.insert('fragment_tag', {
        'fragment_id': f.id,
        'tag_code': tag.code,
      }, conflictAlgorithm: ConflictAlgorithm.ignore);
    }
  }

  Future<void> _replaceImages(DatabaseExecutor txn, Fragment f) async {
    await txn.delete(
      'fragment_image',
      where: 'fragment_id = ?',
      whereArgs: [f.id],
    );
    for (var i = 0; i < f.imagePaths.length; i++) {
      await txn.insert('fragment_image', {
        'fragment_id': f.id,
        'ordinal': i,
        'path': f.imagePaths[i],
      });
    }
  }

  Future<Map<String, List<FragmentTag>>> _loadTags(
    DatabaseExecutor db,
    List<String> ids,
  ) async {
    if (ids.isEmpty) return const {};
    final placeholders = List.filled(ids.length, '?').join(',');
    final rows = await db.rawQuery(
      'SELECT fragment_id, tag_code FROM fragment_tag '
      'WHERE fragment_id IN ($placeholders)',
      ids,
    );
    final out = <String, List<FragmentTag>>{};
    for (final row in rows) {
      final fid = row['fragment_id'] as String;
      final code = row['tag_code'] as String;
      (out[fid] ??= <FragmentTag>[]).add(FragmentTag.fromCode(code));
    }
    return out;
  }

  Future<Map<String, List<String>>> _loadImages(
    DatabaseExecutor db,
    List<String> ids,
  ) async {
    if (ids.isEmpty) return const {};
    final placeholders = List.filled(ids.length, '?').join(',');
    final rows = await db.rawQuery(
      'SELECT fragment_id, path FROM fragment_image '
      'WHERE fragment_id IN ($placeholders) '
      'ORDER BY fragment_id, ordinal',
      ids,
    );
    final out = <String, List<String>>{};
    for (final row in rows) {
      final fid = row['fragment_id'] as String;
      (out[fid] ??= <String>[]).add(row['path'] as String);
    }
    return out;
  }

  Future<Map<String, List<String>>> _loadRecoveryLinks(
    DatabaseExecutor db,
    List<String> recoveryIds,
  ) async {
    if (recoveryIds.isEmpty) return const {};
    final placeholders = List.filled(recoveryIds.length, '?').join(',');
    final rows = await db.rawQuery(
      'SELECT recovery_id, fragment_id FROM recovery_fragment_link '
      'WHERE recovery_id IN ($placeholders)',
      recoveryIds,
    );
    final out = <String, List<String>>{};
    for (final row in rows) {
      final rid = row['recovery_id'] as String;
      (out[rid] ??= <String>[]).add(row['fragment_id'] as String);
    }
    return out;
  }

  Fragment _hydrateFragment(
    Map<String, Object?> row,
    List<FragmentTag> tags,
    List<String> imagePaths,
  ) {
    return Fragment(
      id: row['id'] as String,
      createdAt: DateTime.fromMillisecondsSinceEpoch(
        row['created_at'] as int,
        isUtc: true,
      ),
      content: (row['content'] as String?) ?? '',
      tags: tags,
      intensity: Intensity.fromValue((row['intensity'] as int?) ?? 3),
      stage: FragmentStage.fromCode((row['stage'] as String?) ?? 'outburst'),
      fadePeriod: FadePeriod.fromDays((row['fade_days'] as int?) ?? 270),
      visibility: ShareVisibility.fromCode(
        (row['visibility'] as String?) ?? 'private',
      ),
      imagePaths: imagePaths,
    );
  }

  Recovery _hydrateRecovery(Map<String, Object?> row, List<String> relatedIds) {
    return Recovery(
      id: row['id'] as String,
      createdAt: DateTime.fromMillisecondsSinceEpoch(
        row['created_at'] as int,
        isUtc: true,
      ),
      description: (row['description'] as String?) ?? '',
      intensity: Intensity.fromValue(
        (row['intensity'] as int?) ?? Intensity.hard.value,
      ),
      relatedFragmentIds: relatedIds,
    );
  }

  Map<String, Object?> _fragmentPayload(Fragment f) => {
    'id': f.id,
    'created_at_ms': f.createdAt.toUtc().millisecondsSinceEpoch,
    'content': f.content,
    'tags': [for (final t in f.tags) t.code],
    'intensity': f.intensity.value,
    'stage': f.stage.code,
    'fade_days': f.fadePeriod.days,
    'visibility': f.visibility.code,
    'image_paths': f.imagePaths,
    'protocol_version': DomainEventType.protocolVersion,
  };

  Map<String, Object?> _recoveryPayload(Recovery r) => {
    'id': r.id,
    'created_at_ms': r.createdAt.toUtc().millisecondsSinceEpoch,
    'description': r.description,
    'intensity': r.intensity.value,
    'related_fragment_ids': r.relatedFragmentIds,
    'protocol_version': DomainEventType.protocolVersion,
  };

  Future<void> _appendEvent(
    DatabaseExecutor txn, {
    required int occurredAt,
    required String type,
    required String aggregateId,
    required Map<String, Object?> payload,
  }) async {
    await txn.insert('event_log', {
      'occurred_at': occurredAt,
      'event_type': type,
      'aggregate_id': aggregateId,
      'payload': jsonEncode(payload),
    });
  }
}
