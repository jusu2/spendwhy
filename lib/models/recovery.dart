/// 一束光：好转/和解的瞬间
class Recovery {
  final String id;
  final DateTime createdAt;
  final String description;

  /// 1..5
  final int intensity;

  /// 关联的碎片 ID 列表
  final List<String> relatedFragmentIds;

  const Recovery({
    required this.id,
    required this.createdAt,
    required this.description,
    required this.intensity,
    this.relatedFragmentIds = const [],
  });

  Map<String, Object?> toMap() => {
    'id': id,
    'created_at': createdAt.millisecondsSinceEpoch,
    'description': description,
    'intensity': intensity,
    'related_ids': relatedFragmentIds.join(','),
  };

  factory Recovery.fromMap(Map<String, Object?> m) {
    final raw = (m['related_ids'] as String?) ?? '';
    return Recovery(
      id: m['id'] as String,
      createdAt: DateTime.fromMillisecondsSinceEpoch(m['created_at'] as int),
      description: m['description'] as String? ?? '',
      intensity: (m['intensity'] as int?) ?? 3,
      relatedFragmentIds: raw.isEmpty ? const [] : raw.split(','),
    );
  }
}
