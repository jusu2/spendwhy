import 'package:flutter_test/flutter_test.dart';
import 'package:fragments/models/enums.dart';
import 'package:fragments/models/fragment.dart';
import 'package:fragments/models/recovery.dart';
import 'package:fragments/state/fragments_provider.dart';

import '../fakes/in_memory_fragment_repository.dart';

/// 一个所有读操作都抛错的仓储替身，用来验证 [`FragmentsProvider.load`] 的 catch 分支。
class _BrokenRepo extends InMemoryFragmentRepository {
  @override
  Future<List<Fragment>> listFragments({int? limit}) =>
      Future<List<Fragment>>.error(StateError('boom-fragments'));
  @override
  Future<List<Recovery>> listRecoveries({int? limit}) =>
      Future<List<Recovery>>.error(StateError('boom-recoveries'));
}

void main() {
  // FragmentsProvider 在 flutter_test 里不能真正调 Rust（FRB 未初始化），
  // _applyView 内部会 catch 并降级为 fadeLevel=1.0，_lastError 非空——
  // 这是预期行为，正好用来验证「数据流」正确性。
  TestWidgetsFlutterBinding.ensureInitialized();

  final fixedNow = DateTime.utc(2026, 5, 13, 10);
  late InMemoryFragmentRepository repo;
  late FragmentsProvider provider;

  setUp(() {
    repo = InMemoryFragmentRepository();
    provider = FragmentsProvider(db: repo, now: () => fixedNow);
  });

  group('load', () {
    test('initial load yields empty lists and no error', () async {
      await provider.load();
      expect(provider.fragments, isEmpty);
      expect(provider.recoveries, isEmpty);
      expect(provider.loading, isFalse);
    });

    test('repository read failure surfaces via lastError', () async {
      final broken = FragmentsProvider(db: _BrokenRepo(), now: () => fixedNow);
      await broken.load();
      expect(broken.lastError, isNotNull);
      expect(broken.loading, isFalse);
    });
  });

  group('addFragment', () {
    test('persists to repo and prepends to in-memory list', () async {
      await provider.load();
      final f = await provider.addFragment(
        content: 'hello',
        tags: const [FragmentTag.work],
        intensity: Intensity.hard,
      );

      expect(await repo.getFragment(f.id), isNotNull);
      expect(provider.fragments, hasLength(1));
      expect(provider.fragments.first.id, f.id);
      expect(f.createdAt, fixedNow);
    });

    test('multiple adds keep newest at index 0', () async {
      final a = await provider.addFragment(
        content: 'a',
        tags: const [],
        intensity: Intensity.hard,
      );
      final b = await provider.addFragment(
        content: 'b',
        tags: const [],
        intensity: Intensity.hard,
      );
      expect(provider.fragments.first.id, b.id);
      expect(provider.fragments.last.id, a.id);
    });
  });

  group('addRecovery', () {
    test(
      'does not write any DB row when Rust call fails (atomic guard)',
      () async {
        final f = await provider.addFragment(
          content: 'down',
          tags: const [],
          intensity: Intensity.hard,
        );

        Object? caught;
        try {
          await provider.addRecovery(
            description: 'better',
            intensity: Intensity.severe,
            relatedFragmentIds: [f.id],
          );
        } catch (e) {
          caught = e;
        }

        // Rust 在测试环境必抛；recovery 必须没落库，碎片阶段保持不变。
        expect(caught, isNotNull);
        expect(await repo.listRecoveries(), isEmpty);
        final reloaded = await repo.getFragment(f.id);
        expect(reloaded?.stage, FragmentStage.outburst);
      },
    );
  });

  group('updateFragment / deleteFragment', () {
    test('update replaces by id, keeping list length', () async {
      final f = await provider.addFragment(
        content: 'a',
        tags: const [],
        intensity: Intensity.hard,
      );
      await provider.updateFragment(f.copyWith(content: 'b'));
      expect(provider.fragments, hasLength(1));
      expect(provider.fragments.first.content, 'b');
    });

    test('delete removes by id', () async {
      final f = await provider.addFragment(
        content: 'a',
        tags: const [],
        intensity: Intensity.hard,
      );
      await provider.deleteFragment(f.id);
      expect(provider.fragments, isEmpty);
      expect(await repo.getFragment(f.id), isNull);
    });
  });

  group('dispose safety', () {
    test('does not throw if load completes after dispose', () async {
      final p = FragmentsProvider(db: repo, now: () => fixedNow);
      final pending = p.load();
      p.dispose();
      await pending; // 不应抛 "called after dispose"
    });
  });

  group('helpers', () {
    test('findById returns null for unknown id', () {
      expect(provider.findById('does-not-exist'), isNull);
    });

    test('recoveriesFor filters by related id', () async {
      // 直接通过 repo 注入数据避免 Rust 触发。
      final r = Recovery(
        id: 'rec-1',
        createdAt: fixedNow,
        description: 'desc',
        intensity: Intensity.hard,
        relatedFragmentIds: const ['frag-x'],
      );
      await repo.recordRecoveryTx(recovery: r, advancedFragments: const []);
      await provider.load();
      expect(provider.recoveriesFor('frag-x'), hasLength(1));
      expect(provider.recoveriesFor('frag-y'), isEmpty);
    });
  });
}
