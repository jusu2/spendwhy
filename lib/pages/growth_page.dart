import 'package:flutter/material.dart';
import 'package:provider/provider.dart';

import '../i18n/strings.dart';
import '../state/fragments_provider.dart';
import '../widgets/growth_painter.dart';

class GrowthPage extends StatefulWidget {
  const GrowthPage({super.key});

  @override
  State<GrowthPage> createState() => _GrowthPageState();
}

class _GrowthPageState extends State<GrowthPage> {
  _Range _range = _Range.quarter;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final provider = context.watch<FragmentsProvider>();
    final now = DateTime.now();
    final start = now.subtract(_range.duration);
    final isDark = theme.brightness == Brightness.dark;
    final score = provider.growthScore.round();

    return ListView(
      padding: const EdgeInsets.fromLTRB(28, 32, 28, 60),
      children: [
        Text(S.growthTitle.toUpperCase(), style: theme.textTheme.titleMedium),
        const SizedBox(height: 18),
        Text(
          provider.fragments.isEmpty ? S.growthEmpty : '$score',
          style: theme.textTheme.displayLarge,
        ),
        const SizedBox(height: 6),
        Text(
          provider.fragments.isEmpty
              ? '光会慢慢出现的。'
              : '${provider.fragments.length} 块碎片  ·  ${provider.recoveries.length} 束光',
          style: theme.textTheme.bodySmall?.copyWith(
            fontStyle: FontStyle.italic,
          ),
        ),
        const SizedBox(height: 32),
        Container(
          height: 0.6,
          width: 24,
          color: theme.colorScheme.onSurface.withValues(alpha: 0.45),
        ),
        const SizedBox(height: 28),
        Row(
          children: [
            _RangeBtn(
              label: S.growthMonth,
              selected: _range == _Range.month,
              onTap: () => setState(() => _range = _Range.month),
            ),
            const SizedBox(width: 24),
            _RangeBtn(
              label: S.growthQuarter,
              selected: _range == _Range.quarter,
              onTap: () => setState(() => _range = _Range.quarter),
            ),
            const SizedBox(width: 24),
            _RangeBtn(
              label: S.growthYear,
              selected: _range == _Range.year,
              onTap: () => setState(() => _range = _Range.year),
            ),
          ],
        ),
        const SizedBox(height: 28),
        SizedBox(
          height: 240,
          child: CustomPaint(
            painter: GrowthPainter(
              fragments: provider.fragments,
              recoveries: provider.recoveries,
              start: start,
              end: now,
              textColor: theme.colorScheme.onSurface,
              isDark: isDark,
            ),
            child: const SizedBox.expand(),
          ),
        ),
      ],
    );
  }
}

enum _Range { month, quarter, year }

extension _RangeX on _Range {
  Duration get duration => switch (this) {
    _Range.month => const Duration(days: 30),
    _Range.quarter => const Duration(days: 90),
    _Range.year => const Duration(days: 365),
  };
}

class _RangeBtn extends StatelessWidget {
  final String label;
  final bool selected;
  final VoidCallback onTap;
  const _RangeBtn({
    required this.label,
    required this.selected,
    required this.onTap,
  });
  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return GestureDetector(
      onTap: onTap,
      behavior: HitTestBehavior.opaque,
      child: Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          Text(
            label,
            style: theme.textTheme.bodyMedium?.copyWith(
              color: selected
                  ? theme.colorScheme.onSurface
                  : theme.colorScheme.onSurface.withValues(alpha: 0.4),
              letterSpacing: 2,
            ),
          ),
          const SizedBox(height: 4),
          AnimatedContainer(
            duration: const Duration(milliseconds: 220),
            height: 0.6,
            width: selected ? 18 : 0,
            color: theme.colorScheme.onSurface,
          ),
        ],
      ),
    );
  }
}
