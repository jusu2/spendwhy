import 'package:flutter/foundation.dart';
import 'package:uuid/uuid.dart';

import '../data/database.dart';
import '../data/fragment_repository.dart';
import '../models/enums.dart';
import '../models/fragment.dart';
import '../models/recovery.dart';
import '../services/rust_backend.dart';

/// 应用层视图状态：
/// - 持有当前 fragments/recoveries 的内存快照与 Rust 计算出的 fade_level
/// - 业务规则统一通过 [RustBackend] 调用 Rust use case
/// - 写后局部更新，不再每次 reload 全表
///
/// 时间源 [_now] 默认使用系统时钟，但允许测试注入固定时间，
/// 让 fade_level、createdAt、growth_score 等基于时间的逻辑可断言。
class FragmentsProvider extends ChangeNotifier {
  FragmentsProvider({
    FragmentRepository? db,
    Uuid? uuid,
    DateTime Function()? now,
  }) : _db = db ?? AppDatabase.instance,
       _uuid = uuid ?? const Uuid(),
       _now = now ?? _systemNowUtc;

  static DateTime _systemNowUtc() => DateTime.now().toUtc();

  final FragmentRepository _db;
  final Uuid _uuid;
  final DateTime Function() _now;

  List<Fragment> _fragments = const [];
  List<Recovery> _recoveries = const [];
  double _growthScore = 0;
  bool _loading = false;
  Object? _lastError;
  bool _disposed = false;

  @override
  void dispose() {
    _disposed = true;
    super.dispose();
  }

  /// 仅在未释放时通知监听者，避免异步任务在 widget 销毁后触发
  /// `setState() called after dispose()` 异常。
  void _safeNotify() {
    if (_disposed) return;
    notifyListeners();
  }

  List<Fragment> get fragments => _fragments;
  List<Recovery> get recoveries => _recoveries;
  double get growthScore => _growthScore;
  bool get loading => _loading;
  Object? get lastError => _lastError;

  /// 首次加载或下拉刷新时调用。其它写操作不要再调用 [load]。
  Future<void> load() async {
    _loading = true;
    _lastError = null;
    _safeNotify();
    try {
      // 两个表互不依赖，并发读减少 I/O 阻塞。
      final results = await Future.wait([
        _db.listFragments(),
        _db.listRecoveries(),
      ]);
      final fragments = results[0] as List<Fragment>;
      final recoveries = results[1] as List<Recovery>;
      _applyView(fragments, recoveries);
    } catch (e, st) {
      _lastError = e;
      debugPrint('[FragmentsProvider.load] failed: $e\n$st');
    } finally {
      _loading = false;
      _safeNotify();
    }
  }

  Future<Fragment> addFragment({
    required String content,
    required List<FragmentTag> tags,
    required Intensity intensity,
    FadePeriod fadePeriod = FadePeriod.nineMonths,
    ShareVisibility visibility = ShareVisibility.private,
    List<String> imagePaths = const [],
  }) async {
    // 单次写路径只读一次时钟，让 createdAt 与 fade_level 计算共享同一时间点。
    final now = _now();
    final f = Fragment(
      id: _uuid.v4(),
      createdAt: now,
      content: content,
      tags: tags,
      intensity: intensity,
      fadePeriod: fadePeriod,
      visibility: visibility,
      imagePaths: imagePaths,
    );
    await _db.insertFragment(f);
    _applyView([f, ..._fragments], _recoveries, now: now);
    _safeNotify();
    return f;
  }

  Future<void> updateFragment(Fragment f) async {
    await _db.updateFragment(f);
    final updated = [
      for (final x in _fragments)
        if (x.id == f.id) f else x,
    ];
    _applyView(updated, _recoveries);
    _safeNotify();
  }

  Future<void> deleteFragment(String id) async {
    await _db.deleteFragment(id);
    final updated = _fragments.where((f) => f.id != id).toList();
    _applyView(updated, _recoveries);
    _safeNotify();
  }

  Future<Recovery> addRecovery({
    required String description,
    required Intensity intensity,
    List<String> relatedFragmentIds = const [],
  }) async {
    final now = _now();
    final recovery = Recovery(
      id: _uuid.v4(),
      createdAt: now,
      description: description,
      intensity: intensity,
      relatedFragmentIds: relatedFragmentIds,
    );

    // 用 id->Fragment 索引一次，避免对每个 id 做 O(N) 扫描。
    final byId = {for (final f in _fragments) f.id: f};
    final relatedFragments = <Fragment>[
      for (final id in relatedFragmentIds) ?byId[id],
    ];

    final outcome = RustBackend.recordRecovery(
      recovery: recovery,
      relatedFragments: relatedFragments,
    );

    // 先在内存里算好需要推进的碎片列表，再交给 DB 做单事务写入。
    final advancedById = <String, Fragment>{};
    for (final fid in outcome.fragmentsToAdvance) {
      final current = byId[fid];
      if (current == null) {
        // Rust 返回了本地内存中不存在的碎片 id（数据漂移或并发删除），跳过。
        debugPrint(
          '[FragmentsProvider.addRecovery] skipped advance for unknown id $fid',
        );
        continue;
      }
      advancedById[fid] = current.copyWith(stage: FragmentStage.recovery);
    }

    await _db.recordRecoveryTx(
      recovery: recovery,
      advancedFragments: advancedById.values.toList(growable: false),
    );

    final fragments = advancedById.isEmpty
        ? _fragments
        : [for (final f in _fragments) advancedById[f.id] ?? f];

    _applyView(fragments, [recovery, ..._recoveries], now: now);
    _safeNotify();
    return recovery;
  }

  Fragment? findById(String id) {
    for (final f in _fragments) {
      if (f.id == id) return f;
    }
    return null;
  }

  List<Recovery> recoveriesFor(String fragmentId) => _recoveries
      .where((r) => r.relatedFragmentIds.contains(fragmentId))
      .toList(growable: false);

  // === 内部 =================================================================

  /// 用传入的 [fragments] / [recoveries] 调 Rust 重新计算 fade_level 与 growth_score。
  ///
  /// - 成功：原子替换内存快照，清空 [lastError]。
  /// - 失败：仍然替换列表（用默认 fade=1.0），保证用户刚写入的数据立即可见；
  ///   错误通过 [lastError] 暴露给 UI 提示，不静默吞掉。
  void _applyView(
    List<Fragment> fragments,
    List<Recovery> recoveries, {
    DateTime? now,
  }) {
    try {
      final at = now ?? _now();
      final view = RustBackend.buildHomeView(
        fragments: fragments,
        recoveries: recoveries,
        now: at,
      );
      final byId = {for (final v in view.fragments) v.id: v.fadeLevel};
      _fragments = [
        for (final f in fragments) f.copyWith(fadeLevel: byId[f.id] ?? 1.0),
      ];
      _recoveries = recoveries;
      _growthScore = view.growthScore;
      _lastError = null;
    } catch (e, st) {
      // Rust 计算失败：保留用户数据可见，但 fade_level 退化为默认 1.0，
      // 同时记录 lastError 供 UI 展示降级提示。
      _fragments = [for (final f in fragments) f.copyWith(fadeLevel: 1.0)];
      _recoveries = recoveries;
      _lastError = e;
      debugPrint('[FragmentsProvider._applyView] failed: $e\n$st');
    }
  }
}
