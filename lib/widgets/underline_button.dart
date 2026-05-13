import 'package:flutter/material.dart';

/// 文艺风原生主按钮：纯文字 + 细下划线，没有色块
class UnderlineButton extends StatelessWidget {
  final String label;
  final VoidCallback? onTap;
  final bool primary;

  const UnderlineButton({
    super.key,
    required this.label,
    required this.onTap,
    this.primary = true,
  });

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final enabled = onTap != null;
    final color = primary
        ? theme.colorScheme.onSurface
        : theme.colorScheme.onSurface.withValues(alpha: 0.55);
    return GestureDetector(
      behavior: HitTestBehavior.opaque,
      onTap: onTap,
      child: Opacity(
        opacity: enabled ? 1.0 : 0.35,
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            Padding(
              padding: const EdgeInsets.symmetric(vertical: 6, horizontal: 8),
              child: Text(
                label,
                style: theme.textTheme.bodyMedium?.copyWith(
                  color: color,
                  letterSpacing: 3,
                  height: 1.0,
                ),
              ),
            ),
            Container(height: 0.6, width: 28, color: color),
          ],
        ),
      ),
    );
  }
}
