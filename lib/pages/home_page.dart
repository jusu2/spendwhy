import 'package:flutter/material.dart';
import 'package:intl/intl.dart';
import 'package:provider/provider.dart';

import '../i18n/strings.dart';
import '../models/fragment.dart';
import '../state/fragments_provider.dart';
import '../widgets/fragment_item.dart';
import '../widgets/recovery_dialog.dart';
import '../widgets/underline_button.dart';
import 'detail_page.dart';
import 'record_page.dart';

/// 首页：以 CustomScrollView + Sliver 构成，
/// 碎片/恢复列表通过 SliverList.builder 懒构建，避免大列表全量映射。
class HomePage extends StatelessWidget {
  const HomePage({super.key});

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final provider = context.watch<FragmentsProvider>();
    final fragments = provider.fragments;
    final recoveries = provider.recoveries;
    final score = provider.growthScore.round();
    final today = DateFormat('yyyy.MM.dd').format(DateTime.now());

    return Stack(
      children: [
        RefreshIndicator(
          onRefresh: provider.load,
          child: CustomScrollView(
            physics: const AlwaysScrollableScrollPhysics(),
            slivers: [
              SliverToBoxAdapter(
                child: _Header(today: today, fragments: fragments, score: score, recoveryCount: recoveries.length),
              ),
              if (fragments.isEmpty)
                const SliverToBoxAdapter(child: _Empty())
              else ...[
                SliverToBoxAdapter(child: _SectionTitle(text: S.homeRecentFragments)),
                SliverList.builder(
                  itemCount: fragments.length,
                  itemBuilder: (context, index) {
                    final f = fragments[index];
                    return _FragmentRow(fragment: f);
                  },
                ),
              ],
              if (recoveries.isNotEmpty) ...[
                const SliverToBoxAdapter(child: SizedBox(height: 40)),
                SliverToBoxAdapter(child: _SectionTitle(text: S.homeRecentRecoveries)),
                SliverList.builder(
                  itemCount: recoveries.length.clamp(0, 3),
                  itemBuilder: (context, index) {
                    final r = recoveries[index];
                    return _RecoveryRow(
                      date: DateFormat('M.d').format(r.createdAt),
                      description: r.description,
                    );
                  },
                ),
              ],
              const SliverToBoxAdapter(child: SizedBox(height: 140)),
            ],
          ),
        ),
        Positioned(
          left: 0,
          right: 0,
          bottom: 0,
          child: Container(
            padding: const EdgeInsets.fromLTRB(28, 24, 28, 16),
            decoration: BoxDecoration(
              gradient: LinearGradient(
                begin: Alignment.topCenter,
                end: Alignment.bottomCenter,
                colors: [
                  theme.scaffoldBackgroundColor.withValues(alpha: 0),
                  theme.scaffoldBackgroundColor,
                  theme.scaffoldBackgroundColor,
                ],
              ),
            ),
            child: Row(
              mainAxisAlignment: MainAxisAlignment.spaceBetween,
              children: [
                UnderlineButton(
                  label: S.detailAddRecovery,
                  primary: false,
                  onTap: () => showDialog<void>(
                    context: context,
                    builder: (_) => const RecoveryDialog(),
                  ),
                ),
                UnderlineButton(
                  label: S.homeQuickRecord,
                  onTap: () => Navigator.of(context).push(
                    MaterialPageRoute<void>(
                      builder: (_) => const RecordPage(),
                    ),
                  ),
                ),
              ],
            ),
          ),
        ),
      ],
    );
  }
}

class _Header extends StatelessWidget {
  final String today;
  final List<Fragment> fragments;
  final int score;
  final int recoveryCount;
  const _Header({
    required this.today,
    required this.fragments,
    required this.score,
    required this.recoveryCount,
  });

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Padding(
      padding: const EdgeInsets.fromLTRB(28, 32, 28, 0),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(
            today,
            style: theme.textTheme.labelMedium?.copyWith(letterSpacing: 4),
          ),
          const SizedBox(height: 28),
          Text(
            S.appTagline,
            style: theme.textTheme.displaySmall?.copyWith(height: 1.5),
          ),
          const SizedBox(height: 24),
          Container(
            height: 0.6,
            width: 48,
            color: theme.colorScheme.onSurface.withValues(alpha: 0.45),
          ),
          const SizedBox(height: 28),
          Text(
            fragments.isEmpty
                ? S.homeEmptyHint
                : '${fragments.length} 块碎片  ·  $recoveryCount 束光  ·  和解 $score',
            style: theme.textTheme.bodySmall?.copyWith(letterSpacing: 1.6),
          ),
          const SizedBox(height: 40),
        ],
      ),
    );
  }
}

class _SectionTitle extends StatelessWidget {
  final String text;
  const _SectionTitle({required this.text});
  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Padding(
          padding: const EdgeInsets.symmetric(horizontal: 28),
          child: Text(text.toUpperCase(), style: theme.textTheme.titleMedium),
        ),
        const SizedBox(height: 12),
        Divider(color: theme.dividerTheme.color, indent: 28, endIndent: 28),
      ],
    );
  }
}

class _FragmentRow extends StatelessWidget {
  final Fragment fragment;
  const _FragmentRow({required this.fragment});
  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Column(
      children: [
        FragmentItem(
          fragment: fragment,
          onTap: () => Navigator.of(context).push(
            MaterialPageRoute<void>(
              builder: (_) => DetailPage(fragmentId: fragment.id),
            ),
          ),
        ),
        Divider(color: theme.dividerTheme.color, indent: 28, endIndent: 28),
      ],
    );
  }
}

class _RecoveryRow extends StatelessWidget {
  final String date;
  final String description;
  const _RecoveryRow({required this.date, required this.description});
  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Column(
      children: [
        Padding(
          padding: const EdgeInsets.fromLTRB(28, 22, 28, 22),
          child: Row(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              SizedBox(
                width: 56,
                child: Text(
                  date,
                  style: theme.textTheme.labelMedium
                      ?.copyWith(letterSpacing: 1.6),
                ),
              ),
              const SizedBox(width: 8),
              Expanded(
                child: Text(
                  '— $description',
                  style: theme.textTheme.bodyLarge
                      ?.copyWith(fontStyle: FontStyle.italic),
                ),
              ),
            ],
          ),
        ),
        Divider(color: theme.dividerTheme.color, indent: 28, endIndent: 28),
      ],
    );
  }
}

class _Empty extends StatelessWidget {
  const _Empty();
  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Padding(
      padding: const EdgeInsets.fromLTRB(28, 80, 28, 60),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(S.homeEmptyTitle, style: theme.textTheme.headlineMedium),
          const SizedBox(height: 14),
          Text(
            '在你愿意的时候，\n写一句也可以。',
            style: theme.textTheme.bodyMedium?.copyWith(
              fontStyle: FontStyle.italic,
              color: theme.colorScheme.onSurface.withValues(alpha: 0.55),
            ),
          ),
        ],
      ),
    );
  }
}
