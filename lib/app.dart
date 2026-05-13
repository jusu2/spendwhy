import 'dart:async';

import 'package:flutter/material.dart';
import 'package:provider/provider.dart';

import 'i18n/strings.dart';
import 'pages/growth_page.dart';
import 'pages/home_page.dart';
import 'pages/onboarding_page.dart';
import 'pages/settings_page.dart';
import 'services/app_settings.dart';
import 'state/fragments_provider.dart';
import 'theme/app_theme.dart';
import 'widgets/grain_background.dart';

class FragmentsApp extends StatelessWidget {
  final AppSettings settings;
  const FragmentsApp({super.key, required this.settings});

  @override
  Widget build(BuildContext context) {
    return MultiProvider(
      providers: [
        ChangeNotifierProvider.value(value: settings),
        ChangeNotifierProvider(
          create: (_) {
            final provider = FragmentsProvider();
            unawaited(provider.load());
            return provider;
          },
        ),
      ],
      child: MaterialApp(
        title: S.appName,
        debugShowCheckedModeBanner: false,
        theme: AppTheme.light(),
        darkTheme: AppTheme.dark(),
        home: Consumer<AppSettings>(
          builder: (_, s, child) =>
              s.onboarded ? const _RootShell() : const OnboardingPage(),
        ),
      ),
    );
  }
}

class _RootShell extends StatefulWidget {
  const _RootShell();

  @override
  State<_RootShell> createState() => _RootShellState();
}

class _RootShellState extends State<_RootShell> {
  int _index = 0;

  @override
  Widget build(BuildContext context) {
    const pages = [HomePage(), GrowthPage(), SettingsPage()];
    return Scaffold(
      body: GrainBackground(
        opacity: 0.05,
        child: SafeArea(
          bottom: false,
          child: Column(
            children: [
              Expanded(
                child: AnimatedSwitcher(
                  duration: const Duration(milliseconds: 280),
                  switchInCurve: Curves.easeOutCubic,
                  switchOutCurve: Curves.easeInCubic,
                  child: KeyedSubtree(
                    key: ValueKey(_index),
                    child: pages[_index],
                  ),
                ),
              ),
              _BottomNav(
                index: _index,
                onChange: (i) => setState(() => _index = i),
              ),
            ],
          ),
        ),
      ),
    );
  }
}

/// 极简文字底部导航：无图标，仅以衬线小字 + 下划线表达
class _BottomNav extends StatelessWidget {
  final int index;
  final ValueChanged<int> onChange;
  const _BottomNav({required this.index, required this.onChange});

  static const _labels = [S.navHome, S.navGrowth, S.navSettings];

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return SafeArea(
      top: false,
      child: Padding(
        padding: const EdgeInsets.fromLTRB(28, 18, 28, 22),
        child: Row(
          children: List.generate(_labels.length, (i) {
            final selected = i == index;
            return Expanded(
              child: GestureDetector(
                behavior: HitTestBehavior.opaque,
                onTap: () => onChange(i),
                child: Column(
                  mainAxisSize: MainAxisSize.min,
                  children: [
                    Text(
                      _labels[i],
                      style: theme.textTheme.titleLarge?.copyWith(
                        color: selected
                            ? theme.colorScheme.onSurface
                            : theme.colorScheme.onSurface.withValues(
                                alpha: 0.35,
                              ),
                        letterSpacing: 4,
                      ),
                    ),
                    const SizedBox(height: 6),
                    AnimatedContainer(
                      duration: const Duration(milliseconds: 240),
                      curve: Curves.easeOutCubic,
                      height: 0.6,
                      width: selected ? 16 : 0,
                      color: theme.colorScheme.onSurface,
                    ),
                  ],
                ),
              ),
            );
          }),
        ),
      ),
    );
  }
}
