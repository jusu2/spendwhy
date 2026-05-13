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
class FragmentsProvider extends ChangeNotifier {
  FragmentsProvider({AppDatabase? db, Uuid? uuid})
    : _db = db ?? AppDatabase.instance,
      _uuid = uuid ?? const Uuid();

  final AppDatabase _db;
  final Uuid _uuid;

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
      createdAt: DateTime.now().toUtc(),
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
      createdAt: DateTime.now().toUtc(),
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
      final old = fragments.firstWhere((x) => x.id == fid);
      final advanced = old.copyWith(stage: FragmentStage.recovery);
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

  void _applyView(List<Fragment> fragments, List<Recovery> recoveries) {
    try {
      final view = RustBackend.buildHomeView(
        fragments: fragments,
        recoveries: recoveries,
      );
      final byId = {for (final v in view.fragments) v.id: v.fadeLevel};
      _fragments = [
        for (final f in fragments) f.copyWith(fadeLevel: byId[f.id] ?? 1.0),
      ];
      _recoveries = recoveries;
      _growthScore = view.growthScore;
      _lastError = null;
    } catch (e, st) {
      _fragments = fragments;
      _recoveries = recoveries;
      _lastError = e;
      debugPrint('[FragmentsProvider._applyView] failed: $e\n$st');
    }
  }
}
