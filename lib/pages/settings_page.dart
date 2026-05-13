import 'package:flutter/material.dart';
import 'package:provider/provider.dart';

import '../i18n/strings.dart';
import '../models/enums.dart';
import '../services/app_settings.dart';
import 'safety_resources_page.dart';

class SettingsPage extends StatelessWidget {
  const SettingsPage({super.key});

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final s = context.watch<AppSettings>();
    return ListView(
      padding: const EdgeInsets.fromLTRB(28, 32, 28, 40),
      children: [
        Text(S.settingsTitle.toUpperCase(), style: theme.textTheme.titleMedium),
        const SizedBox(height: 18),
        Text(S.appName, style: theme.textTheme.displaySmall),
        const SizedBox(height: 6),
        Text(
          S.appTagline,
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

        _Section(
          title: S.settingsLockTitle,
          children: [
            _Switch(
              label: S.settingsLockBio,
              value: s.biometricLock,
              onChanged: (v) => s.setBiometricLock(v),
            ),
          ],
        ),
        _Section(
          title: S.settingsFadeDefault,
          children: [
            Row(
              children: [
                _Choice(
                  label: S.detailFadePeriod6,
                  selected: s.defaultPeriod == FadePeriod.sixMonths,
                  onTap: () => s.setDefaultPeriod(FadePeriod.sixMonths),
                ),
                const SizedBox(width: 24),
                _Choice(
                  label: S.detailFadePeriod9,
                  selected: s.defaultPeriod == FadePeriod.nineMonths,
                  onTap: () => s.setDefaultPeriod(FadePeriod.nineMonths),
                ),
                const SizedBox(width: 24),
                _Choice(
                  label: S.detailFadePeriod12,
                  selected: s.defaultPeriod == FadePeriod.twelveMonths,
                  onTap: () => s.setDefaultPeriod(FadePeriod.twelveMonths),
                ),
              ],
            ),
          ],
        ),
        _Section(
          title: S.settingsSocial,
          children: [
            _Choice(
              label: S.settingsSocialPrivate,
              selected: s.visibility == ShareVisibility.private,
              onTap: () => s.setVisibility(ShareVisibility.private),
            ),
            const SizedBox(height: 14),
            Opacity(
              opacity: 0.4,
              child: _Choice(
                label: '${S.settingsSocialAnonymous}（v1.5）',
                selected: false,
                onTap: () {},
              ),
            ),
          ],
        ),
        _Section(
          title: S.settingsNotifications,
          children: [
            _Switch(
              label: S.settingsNotifications,
              subtitle: S.settingsNotificationsHint,
              value: s.notifications,
              onChanged: (v) => s.setNotifications(v),
            ),
          ],
        ),
        _Section(
          title: S.settingsAbout,
          children: [
            Text(
              S.settingsAboutContent,
              style: theme.textTheme.bodyMedium?.copyWith(
                height: 2.0,
                fontStyle: FontStyle.italic,
              ),
            ),
            const SizedBox(height: 24),
            GestureDetector(
              behavior: HitTestBehavior.opaque,
              onTap: () => Navigator.of(context).push(
                MaterialPageRoute<void>(
                  builder: (_) => const SafetyResourcesPage(),
                ),
              ),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text(
                    S.safetyResourceTitle,
                    style: theme.textTheme.bodyMedium,
                  ),
                  const SizedBox(height: 4),
                  Container(
                    height: 0.6,
                    width: 18,
                    color: theme.colorScheme.onSurface,
                  ),
                ],
              ),
            ),
          ],
        ),
      ],
    );
  }
}

class _Section extends StatelessWidget {
  final String title;
  final List<Widget> children;
  const _Section({required this.title, required this.children});
  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Padding(
      padding: const EdgeInsets.only(bottom: 36),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(title.toUpperCase(), style: theme.textTheme.titleMedium),
          const SizedBox(height: 16),
          ...children,
        ],
      ),
    );
  }
}

class _Switch extends StatelessWidget {
  final String label;
  final String? subtitle;
  final bool value;
  final ValueChanged<bool> onChanged;
  const _Switch({
    required this.label,
    this.subtitle,
    required this.value,
    required this.onChanged,
  });
  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return GestureDetector(
      onTap: () => onChanged(!value),
      behavior: HitTestBehavior.opaque,
      child: Padding(
        padding: const EdgeInsets.symmetric(vertical: 4),
        child: Row(
          children: [
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text(label, style: theme.textTheme.bodyMedium),
                  if (subtitle != null) ...[
                    const SizedBox(height: 2),
                    Text(
                      subtitle!,
                      style: theme.textTheme.bodySmall?.copyWith(
                        fontStyle: FontStyle.italic,
                      ),
                    ),
                  ],
                ],
              ),
            ),
            // 文艺风开关：两个圆点 + 一根细横线
            AnimatedContainer(
              duration: const Duration(milliseconds: 220),
              width: 36,
              height: 16,
              alignment: value ? Alignment.centerRight : Alignment.centerLeft,
              child: Container(
                width: 8,
                height: 8,
                decoration: BoxDecoration(
                  shape: BoxShape.circle,
                  color: value
                      ? theme.colorScheme.onSurface
                      : theme.colorScheme.onSurface.withValues(alpha: 0.35),
                ),
              ),
            ),
          ],
        ),
      ),
    );
  }
}

class _Choice extends StatelessWidget {
  final String label;
  final bool selected;
  final VoidCallback onTap;
  const _Choice({
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
                  : theme.colorScheme.onSurface.withValues(alpha: 0.45),
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
