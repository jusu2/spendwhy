import 'package:flutter/material.dart';
import 'package:provider/provider.dart';

import '../i18n/strings.dart';
import '../services/app_settings.dart';
import '../widgets/grain_background.dart';
import '../widgets/underline_button.dart';

/// 首次启动欢迎流：3 步——问候 / 隐私承诺 / 鼓励第一句
class OnboardingPage extends StatefulWidget {
  const OnboardingPage({super.key});

  @override
  State<OnboardingPage> createState() => _OnboardingPageState();
}

class _OnboardingPageState extends State<OnboardingPage> {
  final _ctrl = PageController();
  int _index = 0;

  static const _slides = [
    _Slide(title: S.onboardWelcomeTitle, body: S.onboardWelcomeBody),
    _Slide(title: S.onboardPrivacyTitle, body: S.onboardPrivacyBody),
    _Slide(title: S.onboardStartTitle, body: S.onboardStartBody),
  ];

  @override
  void dispose() {
    _ctrl.dispose();
    super.dispose();
  }

  Future<void> _next() async {
    if (_index < _slides.length - 1) {
      await _ctrl.nextPage(
        duration: const Duration(milliseconds: 360),
        curve: Curves.easeInOutCubic,
      );
    } else {
      await context.read<AppSettings>().markOnboarded();
    }
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final last = _index == _slides.length - 1;
    return Scaffold(
      body: GrainBackground(
        opacity: 0.05,
        child: SafeArea(
          child: Column(
            children: [
              // 顶部三点指示器
              Padding(
                padding: const EdgeInsets.fromLTRB(28, 24, 28, 0),
                child: Row(
                  mainAxisAlignment: MainAxisAlignment.center,
                  children: List.generate(_slides.length, (i) {
                    final on = i == _index;
                    return AnimatedContainer(
                      duration: const Duration(milliseconds: 220),
                      margin: const EdgeInsets.symmetric(horizontal: 6),
                      width: on ? 16 : 4,
                      height: 0.6,
                      color: theme.colorScheme.onSurface.withValues(
                        alpha: on ? 1.0 : 0.3,
                      ),
                    );
                  }),
                ),
              ),
              Expanded(
                child: PageView.builder(
                  controller: _ctrl,
                  itemCount: _slides.length,
                  onPageChanged: (i) => setState(() => _index = i),
                  itemBuilder: (_, i) => _SlideView(slide: _slides[i]),
                ),
              ),
              Padding(
                padding: const EdgeInsets.fromLTRB(28, 12, 28, 36),
                child: Align(
                  alignment: Alignment.centerRight,
                  child: UnderlineButton(
                    label: last ? S.onboardEnter : S.onboardNext,
                    onTap: _next,
                  ),
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}

class _Slide {
  final String title;
  final String body;
  const _Slide({required this.title, required this.body});
}

class _SlideView extends StatelessWidget {
  final _Slide slide;
  const _SlideView({required this.slide});

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Padding(
      padding: const EdgeInsets.fromLTRB(40, 24, 40, 24),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        mainAxisAlignment: MainAxisAlignment.center,
        children: [
          Text(slide.title, style: theme.textTheme.displayMedium),
          const SizedBox(height: 28),
          Container(
            height: 0.6,
            width: 32,
            color: theme.colorScheme.onSurface.withValues(alpha: 0.5),
          ),
          const SizedBox(height: 28),
          Text(
            slide.body,
            style: theme.textTheme.bodyLarge?.copyWith(
              height: 2.0,
              color: theme.colorScheme.onSurface.withValues(alpha: 0.8),
            ),
          ),
        ],
      ),
    );
  }
}
