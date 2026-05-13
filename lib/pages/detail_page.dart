import 'package:flutter/material.dart';
import 'package:provider/provider.dart';

import '../i18n/strings.dart';
import '../models/enums.dart';
import '../models/fragment.dart';
import '../state/fragments_provider.dart';
import '../utils/date_utils.dart';
import '../widgets/recovery_dialog.dart';
import '../widgets/underline_button.dart';

class DetailPage extends StatelessWidget {
  final String fragmentId;
  const DetailPage({super.key, required this.fragmentId});

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final provider = context.watch<FragmentsProvider>();
    final f = provider.findById(fragmentId);
    if (f == null) {
      return Scaffold(
        body: Center(
          child: Text(
            '已不在了。',
            style: theme.textTheme.bodyMedium?.copyWith(
              fontStyle: FontStyle.italic,
            ),
          ),
        ),
      );
    }
    final recs = provider.recoveriesFor(f.id);
    return Scaffold(
      appBar: AppBar(
        leadingWidth: 64,
        leading: Padding(
          padding: const EdgeInsets.only(left: 24),
          child: IconButton(
            icon: const Icon(Icons.arrow_back, size: 18),
            onPressed: () => Navigator.of(context).pop(),
          ),
        ),
        actions: [
          Padding(
            padding: const EdgeInsets.only(right: 24),
            child: IconButton(
              icon: const Icon(Icons.delete_outline, size: 18),
              onPressed: () => _confirmDelete(context, f),
            ),
          ),
        ],
      ),
      body: SafeArea(
        child: ListView(
          padding: const EdgeInsets.fromLTRB(28, 8, 28, 100),
          children: [
            Text(
              DateText.full(f.createdAt),
              style: theme.textTheme.labelMedium?.copyWith(letterSpacing: 2),
            ),
            const SizedBox(height: 24),
            Text(f.content, style: theme.textTheme.bodyLarge),
            const SizedBox(height: 28),
            Container(
              height: 0.6,
              width: 24,
              color: theme.colorScheme.onSurface.withValues(alpha: 0.45),
            ),
            if (f.tags.isNotEmpty) ...[
              const SizedBox(height: 24),
              Text(
                f.tags.map((t) => t.label).join('  ·  '),
                style: theme.textTheme.labelMedium?.copyWith(letterSpacing: 2),
              ),
            ],
            const SizedBox(height: 40),
            Text(
              S.detailFadePeriod.toUpperCase(),
              style: theme.textTheme.titleMedium,
            ),
            const SizedBox(height: 14),
            Row(
              children: [
                _PeriodOption(
                  label: S.detailFadePeriod6,
                  selected: f.fadePeriod == FadePeriod.sixMonths,
                  onTap: () => context.read<FragmentsProvider>().updateFragment(
                    f.copyWith(fadePeriod: FadePeriod.sixMonths),
                  ),
                ),
                const SizedBox(width: 24),
                _PeriodOption(
                  label: S.detailFadePeriod9,
                  selected: f.fadePeriod == FadePeriod.nineMonths,
                  onTap: () => context.read<FragmentsProvider>().updateFragment(
                    f.copyWith(fadePeriod: FadePeriod.nineMonths),
                  ),
                ),
                const SizedBox(width: 24),
                _PeriodOption(
                  label: S.detailFadePeriod12,
                  selected: f.fadePeriod == FadePeriod.twelveMonths,
                  onTap: () => context.read<FragmentsProvider>().updateFragment(
                    f.copyWith(fadePeriod: FadePeriod.twelveMonths),
                  ),
                ),
              ],
            ),
            const SizedBox(height: 40),
            Text(
              S.homeRecentRecoveries.toUpperCase(),
              style: theme.textTheme.titleMedium,
            ),
            const SizedBox(height: 14),
            if (recs.isEmpty)
              Text(
                '光还没有出现。',
                style: theme.textTheme.bodySmall?.copyWith(
                  fontStyle: FontStyle.italic,
                ),
              )
            else
              Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: recs.map((r) {
                  return Padding(
                    padding: const EdgeInsets.symmetric(vertical: 12),
                    child: Row(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: [
                        SizedBox(
                          width: 50,
                          child: Text(
                            DateText.relative(r.createdAt),
                            style: theme.textTheme.labelMedium,
                          ),
                        ),
                        const SizedBox(width: 8),
                        Expanded(
                          child: Text(
                            '— ${r.description}',
                            style: theme.textTheme.bodyMedium?.copyWith(
                              fontStyle: FontStyle.italic,
                            ),
                          ),
                        ),
                      ],
                    ),
                  );
                }).toList(),
              ),
          ],
        ),
      ),
      bottomNavigationBar: SafeArea(
        child: Padding(
          padding: const EdgeInsets.fromLTRB(28, 8, 28, 24),
          child: Align(
            alignment: Alignment.centerRight,
            child: UnderlineButton(
              label: S.detailAddRecovery,
              onTap: () => showDialog<void>(
                context: context,
                builder: (_) => RecoveryDialog(prefilledFragment: f),
              ),
            ),
          ),
        ),
      ),
    );
  }

  Future<void> _confirmDelete(BuildContext context, Fragment f) async {
    final ok = await showDialog<bool>(
      context: context,
      builder: (_) => AlertDialog(
        backgroundColor: Theme.of(context).scaffoldBackgroundColor,
        shape: const RoundedRectangleBorder(),
        content: Text(
          S.detailDeleteConfirm,
          style: Theme.of(context).textTheme.bodyMedium,
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.of(context).pop(false),
            child: const Text(S.cancel),
          ),
          TextButton(
            onPressed: () => Navigator.of(context).pop(true),
            child: Text(
              S.delete,
              style: TextStyle(color: const Color(0xFFB87C7C)),
            ),
          ),
        ],
      ),
    );
    if (ok != true) return;
    if (!context.mounted) return;
    await context.read<FragmentsProvider>().deleteFragment(f.id);
    if (context.mounted) Navigator.of(context).pop();
  }
}

class _PeriodOption extends StatelessWidget {
  final String label;
  final bool selected;
  final VoidCallback onTap;
  const _PeriodOption({
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
