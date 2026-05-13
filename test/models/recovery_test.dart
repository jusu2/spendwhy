import 'package:flutter_test/flutter_test.dart';
import 'package:fragments/models/enums.dart';
import 'package:fragments/models/recovery.dart';

void main() {
  group('Recovery.toMap / fromMap', () {
    test('round-trip preserves all fields', () {
      final original = Recovery(
        id: 'rec-1',
        createdAt: DateTime.fromMillisecondsSinceEpoch(1_700_000_000_000),
        description: '今天和朋友聊了聊',
        intensity: Intensity.severe,
        relatedFragmentIds: const ['f1', 'f2'],
      );

      final restored = Recovery.fromMap(original.toMap());

      expect(restored.id, original.id);
      expect(restored.createdAt, original.createdAt);
      expect(restored.description, original.description);
      expect(restored.intensity, original.intensity);
      expect(restored.relatedFragmentIds, original.relatedFragmentIds);
    });

    test('empty related ids stay empty', () {
      final original = Recovery(
        id: 'rec-2',
        createdAt: DateTime.utc(2026, 5, 1),
        description: '一个人散步',
        intensity: Intensity.hard,
      );

      final restored = Recovery.fromMap(original.toMap());

      expect(restored.relatedFragmentIds, isEmpty);
    });

    test('fromMap tolerates missing optional columns', () {
      final m = <String, Object?>{
        'id': 'legacy-rec',
        'created_at': 1_600_000_000_000,
        'description': '一束光',
        'intensity': null,
      };

      final r = Recovery.fromMap(m);

      expect(r.id, 'legacy-rec');
      expect(r.intensity, Intensity.hard);
      expect(r.relatedFragmentIds, isEmpty);
    });

    test('rejects empty id', () {
      expect(
        () => Recovery(
          id: '',
          createdAt: DateTime.utc(2026, 5, 1),
          description: 'x',
          intensity: Intensity.hard,
        ),
        throwsA(isA<ArgumentError>()),
      );
    });

    test('rejects empty description', () {
      expect(
        () => Recovery(
          id: 'r',
          createdAt: DateTime.utc(2026, 5, 1),
          description: '',
          intensity: Intensity.hard,
        ),
        throwsA(isA<ArgumentError>()),
      );
    });

    test('equality is structural', () {
      final t = DateTime.utc(2026, 5, 1);
      final a = Recovery(
        id: 'r',
        createdAt: t,
        description: 'x',
        intensity: Intensity.hard,
        relatedFragmentIds: const ['a', 'b'],
      );
      final b = Recovery(
        id: 'r',
        createdAt: t,
        description: 'x',
        intensity: Intensity.hard,
        relatedFragmentIds: const ['a', 'b'],
      );
      expect(a, equals(b));
      expect(a.hashCode, b.hashCode);
    });
  });
}
