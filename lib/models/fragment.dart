import 'enums.dart';

/// 一块碎片
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

  const Fragment({
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
  });

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
}
