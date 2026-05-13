import 'package:flutter_test/flutter_test.dart';
import 'package:fragments/data/domain_event.dart';
import 'package:fragments/models/enums.dart';
import 'package:fragments/models/fragment.dart';
import 'package:fragments/models/recovery.dart';

import '../fakes/in_memory_fragment_repository.dart';

/// 这些测试只验证仓储**接口契约**（ADR-0004/0005 在 Dart 侧最小可观察行为），
/// 真实 sqflite 行为由集成测试 + 手测覆盖。
void main() {
  group('FragmentRepository contract (ADR-0004 + 0005)', () {
    late InMemoryFragmentRepository repo;
    final t0 = DateTime.utc(2026, 5, 13, 10);

    setUp(() {
      repo = InMemoryFragmentRepository();
    });

    Fragment makeFragment(String id) => Fragment(
      id: id,
      createdAt: t0,
      content: 'c-$id',
      tags: const [FragmentTag.work],
      intensity: Intensity.hard,
    );

    test('insertFragment emits FragmentCreated', () async {
      await repo.insertFragment(makeFragment('f1'));
      final events = await repo.listEvents();
      expect(events, hasLength(1));
      expect(events.single.eventType, DomainEventType.fragmentCreated);
      expect(events.single.aggregateId, 'f1');
    });

    test(
      'updateFragment with stage change emits both Edited + StageAdvanced',
      () async {
        final f = makeFragment('f1');
        await repo.insertFragment(f);
        await repo.updateFragment(f.copyWith(stage: FragmentStage.recovery));
        final types = (await repo.listEvents())
            .map((e) => e.eventType)
            .toList();
        expect(
          types,
          containsAll([
            DomainEventType.fragmentCreated,
            DomainEventType.fragmentEdited,
            DomainEventType.fragmentStageAdvanced,
          ]),
        );
      },
    );

    test(
      'deleteFragment is soft: row vanishes from reads + emits Deleted',
      () async {
        await repo.insertFragment(makeFragment('f1'));
        await repo.deleteFragment('f1');
        expect(await repo.getFragment('f1'), isNull);
        expect(await repo.listFragments(), isEmpty);
        final types = (await repo.listEvents()).map((e) => e.eventType);
        expect(types, contains(DomainEventType.fragmentDeleted));
      },
    );

    test(
      'recordRecoveryTx emits RecoveryRecorded + per-advance StageAdvanced',
      () async {
        final f = makeFragment('f1');
        await repo.insertFragment(f);
        final r = Recovery(
          id: 'r1',
          createdAt: t0,
          description: 'better',
          intensity: Intensity.severe,
          relatedFragmentIds: const ['f1'],
        );
        await repo.recordRecoveryTx(
          recovery: r,
          advancedFragments: [f.copyWith(stage: FragmentStage.recovery)],
        );
        final events = await repo.listEvents();
        final types = events.map((e) => e.eventType).toList();
        expect(types, contains(DomainEventType.recoveryRecorded));
        expect(types, contains(DomainEventType.fragmentStageAdvanced));
      },
    );

    test('listEvents filters by aggregateId', () async {
      await repo.insertFragment(makeFragment('f1'));
      await repo.insertFragment(makeFragment('f2'));
      final only = await repo.listEvents(aggregateId: 'f2');
      expect(only, hasLength(1));
      expect(only.single.aggregateId, 'f2');
    });
  });
}
