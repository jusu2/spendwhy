import 'package:fragments/data/domain_event.dart';
import 'package:fragments/data/fragment_repository.dart';
import 'package:fragments/models/fragment.dart';
import 'package:fragments/models/recovery.dart';

/// 测试用的内存版仓储：行为足以让 [`FragmentsProvider`] 单测覆盖写路径。
///
/// - 失败模拟通过 [throwOn] 控制：把方法名加入集合即可在下次调用时抛错。
/// - 事务语义：[recordRecoveryTx] 一次性写入；任一步抛错都不会留半成品。
/// - 软删（ADR-0004）：[deleteFragment] 后该行从读 API 消失（与 sqflite 实现等价）。
/// - 事件日志（ADR-0005）：写路径自动 append；测试可通过 [listEvents] 断言。
class InMemoryFragmentRepository implements FragmentRepository {
  final Map<String, Fragment> _fragments = {};
  final Set<String> _deletedFragmentIds = <String>{};
  final Map<String, Recovery> _recoveries = {};
  final List<DomainEvent> _events = <DomainEvent>[];
  int _seq = 0;

  /// 加入字符串如 `'insertFragment'` 即可在下次调用对应方法时抛 [StateError]。
  final Set<String> throwOn = <String>{};

  void _maybeThrow(String name) {
    if (throwOn.remove(name)) {
      throw StateError('test-induced failure: $name');
    }
  }

  void _append(String type, String aggregateId, Map<String, Object?> payload) {
    _seq += 1;
    _events.add(
      DomainEvent(
        seq: _seq,
        occurredAt: DateTime.now().toUtc(),
        eventType: type,
        aggregateId: aggregateId,
        payload: payload,
      ),
    );
  }

  @override
  Future<void> insertFragment(Fragment f) async {
    _maybeThrow('insertFragment');
    _fragments[f.id] = f;
    _deletedFragmentIds.remove(f.id);
    _append(DomainEventType.fragmentCreated, f.id, const {});
  }

  @override
  Future<void> updateFragment(Fragment f) async {
    _maybeThrow('updateFragment');
    final old = _fragments[f.id];
    _fragments[f.id] = f;
    _append(DomainEventType.fragmentEdited, f.id, const {});
    if (old != null && old.stage != f.stage) {
      _append(DomainEventType.fragmentStageAdvanced, f.id, {
        'from': old.stage.code,
        'to': f.stage.code,
      });
    }
  }

  @override
  Future<void> deleteFragment(String id) async {
    _maybeThrow('deleteFragment');
    if (_fragments.remove(id) != null) {
      _deletedFragmentIds.add(id);
      _append(DomainEventType.fragmentDeleted, id, const {});
    }
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
    snapshotRecoveries[recovery.id] = recovery;
    final stageChanges = <(String, String, String)>[]; // (id, from, to)
    for (final f in advancedFragments) {
      final prev = snapshotFragments[f.id];
      snapshotFragments[f.id] = f;
      if (prev != null && prev.stage != f.stage) {
        stageChanges.add((f.id, prev.stage.code, f.stage.code));
      }
    }
    _fragments
      ..clear()
      ..addAll(snapshotFragments);
    _recoveries
      ..clear()
      ..addAll(snapshotRecoveries);
    for (final (id, from, to) in stageChanges) {
      _append(DomainEventType.fragmentStageAdvanced, id, {
        'from': from,
        'to': to,
      });
    }
    _append(DomainEventType.recoveryRecorded, recovery.id, const {});
  }

  @override
  Future<List<DomainEvent>> listEvents({
    String? aggregateId,
    int? limit,
  }) async {
    Iterable<DomainEvent> it = _events;
    if (aggregateId != null) {
      it = it.where((e) => e.aggregateId == aggregateId);
    }
    final result = it.toList();
    if (limit != null && result.length > limit) {
      return result.sublist(0, limit);
    }
    return result;
  }
}
