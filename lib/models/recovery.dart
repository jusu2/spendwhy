import 'enums.dart';

/// 一束光：好转/和解的瞬间。
///
/// **不变式**（ADR-0003）：
/// - `id` 非空
/// - `description` 非空
/// - `intensity` 为 [Intensity] enum（由类型系统限定在 1..=5）
/// - `relatedFragmentIds` 中每个 id 非空
class Recovery {
  final String id;
  final DateTime createdAt;
  final String description;
  final Intensity intensity;
  final List<String> relatedFragmentIds;

  Recovery({
    required this.id,
    required this.createdAt,
    required this.description,
    required this.intensity,
    this.relatedFragmentIds = const [],
  }) {
    if (id.isEmpty) {
      throw ArgumentError.value(id, 'Recovery.id', 'must be non-empty');
    }
    if (description.isEmpty) {
      throw ArgumentError.value(
        description,
        'Recovery.description',
        'must be non-empty',
      );
    }
    for (final fid in relatedFragmentIds) {
      if (fid.isEmpty) {
        throw ArgumentError.value(
          fid,
          'Recovery.relatedFragmentIds',
          'must not contain empty id',
        );
      }
    }
  }

  Map<String, Object?> toMap() => {
    'id': id,
    'created_at': createdAt.millisecondsSinceEpoch,
    'description': description,
    'intensity': intensity.value,
    'related_ids': relatedFragmentIds.join(','),
  };

  factory Recovery.fromMap(Map<String, Object?> m) {
    final raw = (m['related_ids'] as String?) ?? '';
    final intensityRaw = (m['intensity'] as int?) ?? Intensity.hard.value;
    return Recovery(
      id: m['id'] as String,
      createdAt: DateTime.fromMillisecondsSinceEpoch(m['created_at'] as int),
      description: m['description'] as String? ?? '',
      intensity: Intensity.fromValue(intensityRaw),
      relatedFragmentIds: raw.isEmpty ? const [] : raw.split(','),
    );
  }

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is Recovery &&
          other.id == id &&
          other.createdAt == createdAt &&
          other.description == description &&
          other.intensity == intensity &&
          _listEq(other.relatedFragmentIds, relatedFragmentIds);

  @override
  int get hashCode => Object.hash(
    id,
    createdAt,
    description,
    intensity,
    Object.hashAll(relatedFragmentIds),
  );

  @override
  String toString() =>
      'Recovery(id: $id, intensity: $intensity, related: ${relatedFragmentIds.length})';
}

bool _listEq(List<String> a, List<String> b) {
  if (a.length != b.length) return false;
  for (var i = 0; i < a.length; i++) {
    if (a[i] != b[i]) return false;
  }
  return true;
}
