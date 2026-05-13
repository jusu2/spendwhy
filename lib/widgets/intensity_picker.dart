import 'package:flutter/material.dart';

import '../models/enums.dart';

/// 极简强度选择：5 个圆点，从隐隐到撑不住，可点选
class IntensityPicker extends StatelessWidget {
  final Intensity value;
  final ValueChanged<Intensity> onChanged;

  const IntensityPicker({
    super.key,
    required this.value,
    required this.onChanged,
  });

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Row(
          children: Intensity.values.map((i) {
            final picked = i.value <= value.value;
            final size = 8.0 + i.value * 1.6;
            return Padding(
              padding: const EdgeInsets.only(right: 14),
              child: GestureDetector(
                behavior: HitTestBehavior.opaque,
                onTap: () => onChanged(i),
                child: Container(
                  width: 28,
                  height: 28,
                  alignment: Alignment.center,
                  child: AnimatedContainer(
                    duration: const Duration(milliseconds: 220),
                    width: size,
                    height: size,
                    decoration: BoxDecoration(
                      shape: BoxShape.circle,
                      color: picked
                          ? theme.colorScheme.onSurface
                          : theme.colorScheme.onSurface.withValues(alpha: 0.18),
                    ),
                  ),
                ),
              ),
            );
          }).toList(),
        ),
        const SizedBox(height: 10),
        Text(value.label, style: theme.textTheme.bodySmall),
      ],
    );
  }
}
