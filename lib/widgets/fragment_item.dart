import 'package:flutter/material.dart';

import '../models/fragment.dart';
import '../utils/date_utils.dart';

/// 文艺日记式的碎片项：左侧细日期、右侧正文，纯文字 + 极细分割线
class FragmentItem extends StatelessWidget {
  final Fragment fragment;
  final VoidCallback? onTap;

  const FragmentItem({super.key, required this.fragment, this.onTap});

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final clarity = fragment.fadeLevel.clamp(0.0, 1.0);
    // 文字透明度：0.45..1.0
    final textColor = theme.colorScheme.onSurface.withValues(
      alpha: 0.45 + 0.55 * clarity,
    );
    final dateColor = theme.colorScheme.onSurface.withValues(
      alpha: 0.25 + 0.35 * clarity,
    );

    return InkWell(
      onTap: onTap,
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: 28, vertical: 22),
        child: Row(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            // 左侧细日期柱
            SizedBox(
              width: 56,
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text(
                    DateText.monthDay(fragment.createdAt),
                    style: theme.textTheme.labelMedium?.copyWith(
                      color: dateColor,
                      letterSpacing: 1.6,
                    ),
                  ),
                  const SizedBox(height: 4),
                  Text(
                    _intensityMark(fragment.intensity.value),
                    style: theme.textTheme.labelSmall?.copyWith(
                      color: dateColor,
                      letterSpacing: 1.0,
                    ),
                  ),
                ],
              ),
            ),
            const SizedBox(width: 8),
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text(
                    fragment.content,
                    maxLines: 4,
                    overflow: TextOverflow.ellipsis,
                    style: theme.textTheme.bodyLarge?.copyWith(
                      color: textColor,
                      height: 1.85,
                    ),
                  ),
                  if (fragment.tags.isNotEmpty) ...[
                    const SizedBox(height: 10),
                    Text(
                      fragment.tags.map((t) => t.label).join('  ·  '),
                      style: theme.textTheme.labelSmall?.copyWith(
                        color: dateColor,
                        letterSpacing: 2.0,
                      ),
                    ),
                  ],
                ],
              ),
            ),
          ],
        ),
      ),
    );
  }

  String _intensityMark(int v) {
    // 用极简符号代替进度条
    return '·' * v;
  }
}
