import 'package:flutter_test/flutter_test.dart';
import 'package:fragments/models/enums.dart';
import 'package:fragments/models/fragment.dart';

void main() {
  group('Fragment.toMap / fromMap', () {
    test('round-trip preserves all persisted fields', () {
      final original = Fragment(
        id: 'abc-123',
        createdAt: DateTime.fromMillisecondsSinceEpoch(1_700_000_000_000),
        content: '今天有点低落',
        tags: const [FragmentTag.work, FragmentTag.health],
        intensity: Intensity.severe,
        stage: FragmentStage.recovery,
        fadePeriod: FadePeriod.sixMonths,
        visibility: ShareVisibility.anonymous,
        imagePaths: const ['a.jpg', 'b.png'],
      );

      final restored = Fragment.fromMap(original.toMap());

      expect(restored.id, original.id);
      expect(restored.createdAt, original.createdAt);
      expect(restored.content, original.content);
      expect(restored.tags, original.tags);
      expect(restored.intensity, original.intensity);
      expect(restored.stage, original.stage);
      expect(restored.fadePeriod, original.fadePeriod);
      expect(restored.visibility, original.visibility);
      expect(restored.imagePaths, original.imagePaths);
    });

    test('round-trip with empty tags and images yields empty const lists', () {
      final original = Fragment(
        id: 'no-tags',
        createdAt: DateTime.fromMillisecondsSinceEpoch(0),
        content: '',
        tags: const [],
        intensity: Intensity.faint,
      );

      final restored = Fragment.fromMap(original.toMap());

      expect(restored.tags, isEmpty);
      expect(restored.imagePaths, isEmpty);
    });

    test('fromMap is tolerant to missing optional columns', () {
      // 模拟旧版本数据库行（缺少 stage / fade_days / visibility / image_paths）。
      final m = <String, Object?>{
        'id': 'legacy',
        'created_at': 1_600_000_000_000,
        'content': 'legacy row',
        'tags': '',
        'intensity': 3,
      };

      final f = Fragment.fromMap(m);

      expect(f.id, 'legacy');
      expect(f.intensity, Intensity.hard);
      expect(f.stage, FragmentStage.outburst);
      expect(f.fadePeriod, FadePeriod.nineMonths);
      expect(f.visibility, ShareVisibility.private);
      expect(f.imagePaths, isEmpty);
    });
  });

  group('Fragment.copyWith', () {
    final base = Fragment(
      id: 'id',
      createdAt: DateTime.utc(2026, 1, 1),
      content: 'hi',
      tags: const [FragmentTag.work],
      intensity: Intensity.hard,
    );

    test('preserves identity fields (id, createdAt)', () {
      final updated = base.copyWith(content: 'bye');
      expect(updated.id, base.id);
      expect(updated.createdAt, base.createdAt);
      expect(updated.content, 'bye');
    });

    test('overrides only the supplied fields', () {
      final updated = base.copyWith(
        stage: FragmentStage.recovery,
        fadeLevel: 0.2,
      );
      expect(updated.stage, FragmentStage.recovery);
      expect(updated.fadeLevel, 0.2);
      expect(updated.content, base.content);
      expect(updated.intensity, base.intensity);
    });
  });
}
