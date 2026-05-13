import 'package:flutter/foundation.dart';
import 'package:uuid/uuid.dart';

import '../data/database.dart';
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
  FragmentsProvider({AppDatabase? db, Uuid? uuid, DateTime Function()? now})
    : _db = db ?? AppDatabase.instance,
      _uuid = uuid ?? const Uuid(),
      _now = now ?? _systemNowUtc;

  static DateTime _systemNowUtc() => DateTime.now().toUtc();

  final AppDatabase _db;
  final Uuid _uuid;
  final DateTime Function() _now;

  List<Fragment> _fragments = const [];
  List<Recovery> _recoveries = const [];
  double _growthScore = 0;
  bool _loading = false;
  Object? _lastError;

  List<Fragment> get fragments => _fragments;
  List<Recovery> get recoveries => _recoveries;
  double get growthScore => _growthScore;
  bool get loading => _loading;
  Object? get lastError => _lastError;

  /// 首次加载或下拉刷新时调用。其它写操作不要再调用 [load]。
  Future<void> load() async {
    _loading = true;
    _lastError = null;
    notifyListeners();
    try {
      final fragments = await _db.listFragments();
      final recoveries = await _db.listRecoveries();
      _applyView(fragments, recoveries);
    } catch (e, st) {
      _lastError = e;
      debugPrint('[FragmentsProvider.load] failed: $e\n$st');
    } finally {
      _loading = false;
      notifyListeners();
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
    final f = Fragment(
      id: _uuid.v4(),
      createdAt: _now(),
      content: content,
      tags: tags,
      intensity: intensity,
      fadePeriod: fadePeriod,
      visibility: visibility,
      imagePaths: imagePaths,
    );
    await _db.insertFragment(f);
    _applyView([f, ..._fragments], _recoveries);
    notifyListeners();
    return f;
  }

  Future<void> updateFragment(Fragment f) async {
    await _db.updateFragment(f);
    final updated = [
      for (final x in _fragments)
        if (x.id == f.id) f else x,
    ];
    _applyView(updated, _recoveries);
    notifyListeners();
  }

  Future<void> deleteFragment(String id) async {
    await _db.deleteFragment(id);
    final updated = _fragments.where((f) => f.id != id).toList();
    _applyView(updated, _recoveries);
    notifyListeners();
  }

  Future<Recovery> addRecovery({
    required String description,
    required int intensity,
    List<String> relatedFragmentIds = const [],
  }) async {
    final recovery = Recovery(
      id: _uuid.v4(),
      createdAt: _now(),
      description: description,
      intensity: intensity,
      relatedFragmentIds: relatedFragmentIds,
    );

    final relatedFragments = <Fragment>[
      for (final id in relatedFragmentIds)
        ..._fragments.where((f) => f.id == id),
    ];

    final outcome = RustBackend.recordRecovery(
      recovery: recovery,
      relatedFragments: relatedFragments,
    );

    await _db.insertRecovery(recovery);

    var fragments = _fragments;
    for (final fid in outcome.fragmentsToAdvance) {
      final idx = fragments.indexWhere((x) => x.id == fid);
      if (idx < 0) {
        // Rust 返回了本地内存中不存在的碎片 id（数据漂移或并发删除）。
        // 跳过该条，避免 firstWhere 直接抛 StateError。
        debugPrint(
          '[FragmentsProvider.addRecovery] skipped advance for unknown id $fid',
        );
        continue;
      }
      final advanced = fragments[idx].copyWith(stage: FragmentStage.recovery);
      await _db.updateFragment(advanced);
      fragments = [
        for (final x in fragments)
          if (x.id == fid) advanced else x,
      ];
    }

    _applyView(fragments, [recovery, ..._recoveries]);
    notifyListeners();
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
  void _applyView(List<Fragment> fragments, List<Recovery> recoveries) {
    try {
      final now = _now();
      final view = RustBackend.buildHomeView(
        fragments: fragments,
        recoveries: recoveries,
        now: now,
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
