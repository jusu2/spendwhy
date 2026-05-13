import 'package:fragments/data/fragment_repository.dart';
import 'package:fragments/models/fragment.dart';
import 'package:fragments/models/recovery.dart';

/// 测试用的内存版仓储：行为足以让 [`FragmentsProvider`] 单测覆盖写路径。
///
/// - 失败模拟通过 [throwOn] 控制：把方法名加入集合即可在下次调用时抛错。
/// - 事务语义：[recordRecoveryTx] 一次性写入；任一步抛错都不会留半成品。
class InMemoryFragmentRepository implements FragmentRepository {
  final Map<String, Fragment> _fragments = {};
  final Map<String, Recovery> _recoveries = {};

  /// 加入字符串如 `'insertFragment'` 即可在下次调用对应方法时抛 [StateError]。
  final Set<String> throwOn = <String>{};

  void _maybeThrow(String name) {
    if (throwOn.remove(name)) {
      throw StateError('test-induced failure: $name');
    }
  }

  @override
  Future<void> insertFragment(Fragment f) async {
    _maybeThrow('insertFragment');
    _fragments[f.id] = f;
  }

  @override
  Future<void> updateFragment(Fragment f) async {
    _maybeThrow('updateFragment');
    _fragments[f.id] = f;
  }

  @override
  Future<void> deleteFragment(String id) async {
    _maybeThrow('deleteFragment');
    _fragments.remove(id);
  }

  @override
  Future<Fragment?> getFragment(String id) async => _fragments[id];

  @override
  Future<List<Fragment>> listFragments({int? limit}) async {
    final all = _fragments.values.toList()
      ..sort((a, b) => b.createdAt.compareTo(a.createdAt));
    return limit == null ? all : all.take(limit).toList();
  }

  @override
  Future<List<Recovery>> listRecoveries({int? limit}) async {
    final all = _recoveries.values.toList()
      ..sort((a, b) => b.createdAt.compareTo(a.createdAt));
    return limit == null ? all : all.take(limit).toList();
  }

  @override
  Future<List<Recovery>> recoveriesForFragment(String fragmentId) async {
    return _recoveries.values
        .where((r) => r.relatedFragmentIds.contains(fragmentId))
        .toList();
  }

  @override
  Future<void> recordRecoveryTx({
    required Recovery recovery,
    required List<Fragment> advancedFragments,
  }) async {
    _maybeThrow('recordRecoveryTx');
    // 模拟事务：先做一份快照，全部成功后再 commit。
    final snapshotFragments = Map<String, Fragment>.from(_fragments);
    final snapshotRecoveries = Map<String, Recovery>.from(_recoveries);
    try {
      snapshotRecoveries[recovery.id] = recovery;
      for (final f in advancedFragments) {
        snapshotFragments[f.id] = f;
      }
      _fragments
        ..clear()
        ..addAll(snapshotFragments);
      _recoveries
        ..clear()
        ..addAll(snapshotRecoveries);
    } catch (_) {
      // 模拟回滚：什么都不做即可（原始 map 未被修改）。
      rethrow;
    }
  }
}
