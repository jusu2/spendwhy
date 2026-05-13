import 'package:flutter/material.dart';

import '../models/enums.dart';

/// 文艺标签选择：纯文字、点选高亮下划线，无圆角胶囊
class TagPicker extends StatelessWidget {
  final Set<FragmentTag> selected;
  final ValueChanged<Set<FragmentTag>> onChanged;

  const TagPicker({super.key, required this.selected, required this.onChanged});

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Wrap(
      spacing: 22,
      runSpacing: 16,
      children: FragmentTag.values.map((t) {
        final picked = selected.contains(t);
        final color = picked
            ? theme.colorScheme.onSurface
            : theme.colorScheme.onSurface.withValues(alpha: 0.4);
        return GestureDetector(
          behavior: HitTestBehavior.opaque,
          onTap: () {
            final next = Set<FragmentTag>.from(selected);
            if (picked) {
              next.remove(t);
            } else {
              next.add(t);
            }
            onChanged(next);
          },
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              Text(
                t.label,
                style: theme.textTheme.bodyMedium?.copyWith(
                  color: color,
                  letterSpacing: 2.0,
                ),
              ),
              const SizedBox(height: 4),
              AnimatedContainer(
                duration: const Duration(milliseconds: 220),
                curve: Curves.easeOutCubic,
                height: 0.6,
                width: picked ? 18 : 0,
                color: theme.colorScheme.onSurface,
              ),
            ],
          ),
        );
      }).toList(),
    );
  }
}
