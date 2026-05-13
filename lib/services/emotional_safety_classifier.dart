/// 文本风险预分类器（不做诊断，仅做温和提示）
///
/// 原则：
/// - 不阻断保存
/// - 不评判内容
/// - 在 [SafetyLevel.elevated] 时静静露出资源入口
/// - 全部为纯字符串规则，跨平台、可单测
library;

enum SafetyLevel {
  /// 普通：不显示任何提示。
  none,

  /// 关注：可能正在难受。底部出现一行温柔陪伴句。
  gentle,

  /// 高关注：可能涉及伤害自己或他人的表达。露出资源入口。
  elevated,
}

class SafetySignal {
  final SafetyLevel level;

  /// 命中规则的关键词集合（用于将来日志/调参，UI 上不展示）
  final List<String> matched;

  const SafetySignal(this.level, this.matched);

  static const SafetySignal none = SafetySignal(SafetyLevel.none, []);
}

/// 仅基于关键词模式做粗筛。**不会**告诉用户"你抑郁了/危险"，
/// 也不会做正/负情绪打分；只是在最重的几类表达出现时温和回应。
class SafetyClassifier {
  /// 高关注：自伤 / 轻生 / 伤害他人意图的明确表达
  static const List<String> _elevated = [
    '自杀',
    '轻生',
    '不想活',
    '不想再活',
    '活不下去',
    '不想活了',
    '想死',
    '去死',
    '寻死',
    '了结自己',
    '结束生命',
    '结束自己',
    '自残',
    '割腕',
    '划自己',
    '伤害自己',
    '杀了',
    '杀死他',
    '杀死她',
    '同归于尽',
  ];

  /// 关注：长期低落、绝望感的常见说法
  static const List<String> _gentle = [
    '撑不住',
    '撑不下去',
    '熬不住',
    '崩溃了',
    '快崩了',
    '没意义',
    '没希望',
    '看不到希望',
    '没出路',
    '一无是处',
    '没人在乎',
    '没有人会想我',
    '太累了',
    '好累',
    '一直哭',
    '停不下来',
    '害怕活着',
  ];

  static SafetySignal classify(String text) {
    if (text.trim().isEmpty) return SafetySignal.none;
    final lower = text.toLowerCase();
    final matched = <String>[];

    for (final w in _elevated) {
      if (lower.contains(w)) matched.add(w);
    }
    if (matched.isNotEmpty) {
      return SafetySignal(SafetyLevel.elevated, matched);
    }

    for (final w in _gentle) {
      if (lower.contains(w)) matched.add(w);
    }
    if (matched.isNotEmpty) {
      return SafetySignal(SafetyLevel.gentle, matched);
    }
    return SafetySignal.none;
  }
}
