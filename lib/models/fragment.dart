import 'enums.dart';

/// 一块碎片。
///
/// **不变式**（ADR-0003）：
/// - `id` 非空
/// - `intensity` 为 [Intensity] enum（1..=5 受类型系统约束）
/// - `fadePeriod` 为 [FadePeriod] enum
/// - `tags` 中每个元素均为枚举值（不可能传入非法字符串）
class Fragment {
  final String id;
  final DateTime createdAt;
  final String content;
  final List<FragmentTag> tags;
  final Intensity intensity;
  final FragmentStage stage;
  final FadePeriod fadePeriod;
  final ShareVisibility visibility;
  final List<String> imagePaths;

  /// 由淡化引擎计算出的 0..1，1=最清晰，0=几乎不可见
  final double fadeLevel;

  Fragment({
    required this.id,
    required this.createdAt,
    required this.content,
    required this.tags,
    required this.intensity,
    this.stage = FragmentStage.outburst,
    this.fadePeriod = FadePeriod.nineMonths,
    this.visibility = ShareVisibility.private,
    this.imagePaths = const [],
    this.fadeLevel = 1.0,
  }) {
    if (id.isEmpty) {
      throw ArgumentError.value(id, 'Fragment.id', 'must be non-empty');
    }
    if (fadeLevel.isNaN || fadeLevel < 0 || fadeLevel > 1) {
      throw ArgumentError.value(
        fadeLevel,
        'Fragment.fadeLevel',
        'must be in [0, 1]',
      );
    }
  }

  Fragment copyWith({
    String? content,
    List<FragmentTag>? tags,
    Intensity? intensity,
    FragmentStage? stage,
    FadePeriod? fadePeriod,
    ShareVisibility? visibility,
    List<String>? imagePaths,
    double? fadeLevel,
  }) {
    return Fragment(
      id: id,
      createdAt: createdAt,
      content: content ?? this.content,
      tags: tags ?? this.tags,
      intensity: intensity ?? this.intensity,
      stage: stage ?? this.stage,
      fadePeriod: fadePeriod ?? this.fadePeriod,
      visibility: visibility ?? this.visibility,
      imagePaths: imagePaths ?? this.imagePaths,
      fadeLevel: fadeLevel ?? this.fadeLevel,
    );
  }

  Map<String, Object?> toMap() => {
    'id': id,
    'created_at': createdAt.millisecondsSinceEpoch,
    'content': content,
    'tags': tags.map((t) => t.code).join(','),
    'intensity': intensity.value,
    'stage': stage.code,
    'fade_days': fadePeriod.days,
    'visibility': visibility.code,
    'image_paths': imagePaths.join('|'),
  };

  factory Fragment.fromMap(Map<String, Object?> m) {
    final tagsRaw = (m['tags'] as String?) ?? '';
    final tagList = tagsRaw.isEmpty
        ? <FragmentTag>[]
        : tagsRaw.split(',').map(FragmentTag.fromCode).toList();
    final imagesRaw = (m['image_paths'] as String?) ?? '';
    return Fragment(
      id: m['id'] as String,
      createdAt: DateTime.fromMillisecondsSinceEpoch(m['created_at'] as int),
      content: m['content'] as String? ?? '',
      tags: tagList,
      intensity: Intensity.fromValue((m['intensity'] as int?) ?? 3),
      stage: FragmentStage.fromCode(m['stage'] as String? ?? 'outburst'),
      fadePeriod: FadePeriod.fromDays((m['fade_days'] as int?) ?? 270),
      visibility: ShareVisibility.fromCode(
        m['visibility'] as String? ?? 'private',
      ),
      imagePaths: imagesRaw.isEmpty ? const [] : imagesRaw.split('|'),
    );
  }

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is Fragment &&
          other.id == id &&
          other.createdAt == createdAt &&
          other.content == content &&
          other.intensity == intensity &&
          other.stage == stage &&
          other.fadePeriod == fadePeriod &&
          other.visibility == visibility &&
          other.fadeLevel == fadeLevel &&
          _listEq(other.tags, tags) &&
          _listEq(other.imagePaths, imagePaths);

  @override
  int get hashCode => Object.hash(
    id,
    createdAt,
    content,
    intensity,
    stage,
    fadePeriod,
    visibility,
    fadeLevel,
    Object.hashAll(tags),
    Object.hashAll(imagePaths),
  );

  @override
  String toString() =>
      'Fragment(id: $id, intensity: $intensity, stage: $stage, fadeLevel: ${fadeLevel.toStringAsFixed(2)})';
}

bool _listEq(List<Object?> a, List<Object?> b) {
  if (a.length != b.length) return false;
  for (var i = 0; i < a.length; i++) {
    if (a[i] != b[i]) return false;
  }
  return true;
}
