import 'package:flutter/material.dart';

/// 主标签：覆盖普通用户最常见的低谷来源
enum FragmentTag {
  relationships('relationships', '关系', Icons.favorite_outline),
  family('family', '家庭', Icons.home_outlined),
  school('school', '学业', Icons.school_outlined),
  work('work', '工作', Icons.work_outline),
  money('money', '金钱', Icons.account_balance_wallet_outlined),
  health('health', '身体', Icons.healing_outlined),
  identity('identity', '自我', Icons.self_improvement_outlined),
  loss('loss', '失去', Icons.cloud_outlined),
  social('social', '社交', Icons.groups_outlined),
  meaning('meaning', '意义', Icons.auto_awesome_outlined),
  other('other', '其它', Icons.more_horiz);

  final String code;
  final String label;
  final IconData icon;
  const FragmentTag(this.code, this.label, this.icon);

  static FragmentTag fromCode(String code) => FragmentTag.values.firstWhere(
    (t) => t.code == code,
    orElse: () => FragmentTag.other,
  );
}

/// 强度 1-5
enum Intensity {
  faint(1, '隐隐'),
  heavy(2, '有些沉'),
  hard(3, '挺难受'),
  severe(4, '很重'),
  overwhelming(5, '撑不住');

  final int value;
  final String label;
  const Intensity(this.value, this.label);

  static Intensity fromValue(int v) => Intensity.values.firstWhere(
    (i) => i.value == v,
    orElse: () => Intensity.hard,
  );
}

/// 阶段
enum FragmentStage {
  outburst('outburst', '爆发期'),
  recovery('recovery', '恢复期'),
  relapse('relapse', '反复期');

  final String code;
  final String label;
  const FragmentStage(this.code, this.label);

  static FragmentStage fromCode(String code) => FragmentStage.values.firstWhere(
    (s) => s.code == code,
    orElse: () => FragmentStage.outburst,
  );
}

/// 淡化周期
enum FadePeriod {
  sixMonths(180, '6 个月'),
  nineMonths(270, '9 个月'),
  twelveMonths(365, '12 个月');

  final int days;
  final String label;
  const FadePeriod(this.days, this.label);

  static FadePeriod fromDays(int d) {
    if (d <= 200) return FadePeriod.sixMonths;
    if (d <= 320) return FadePeriod.nineMonths;
    return FadePeriod.twelveMonths;
  }
}

/// 可见性
enum ShareVisibility {
  private('private', '私密'),
  anonymous('anonymous', '匿名可分享');

  final String code;
  final String label;
  const ShareVisibility(this.code, this.label);

  static ShareVisibility fromCode(String code) => ShareVisibility.values
      .firstWhere((v) => v.code == code, orElse: () => ShareVisibility.private);
}
