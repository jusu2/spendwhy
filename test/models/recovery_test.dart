import 'package:flutter_test/flutter_test.dart';
import 'package:fragments/models/recovery.dart';

void main() {
  group('Recovery.toMap / fromMap', () {
    test('round-trip preserves all fields', () {
      final original = Recovery(
        id: 'rec-1',
        createdAt: DateTime.fromMillisecondsSinceEpoch(1_700_000_000_000),
        description: '今天和朋友聊了聊',
        intensity: 4,
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
        intensity: 3,
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
      expect(r.intensity, 3);
      expect(r.relatedFragmentIds, isEmpty);
    });
  });
}
