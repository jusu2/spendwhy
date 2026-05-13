import 'package:flutter/material.dart';

import 'app_colors.dart';
import 'app_typography.dart';

class AppTheme {
  AppTheme._();

  static ThemeData light() {
    final scheme = const ColorScheme.light(
      primary: AppColors.ink,
      onPrimary: AppColors.paper,
      secondary: AppColors.amber,
      onSecondary: AppColors.ink,
      surface: AppColors.paperSoft,
      onSurface: AppColors.ink,
      error: AppColors.rose,
      onError: AppColors.paper,
      outline: AppColors.paperEdge,
    );

    return _shared(
      scheme: scheme,
      bg: AppColors.paper,
      surface: AppColors.paperSoft,
      border: AppColors.paperEdge,
      text: AppColors.ink,
      textSoft: AppColors.inkSoft,
      textFaint: AppColors.inkFaint,
      typography: AppTypography.light,
    );
  }

  static ThemeData dark() {
    final scheme = const ColorScheme.dark(
      primary: AppColors.inkText,
      onPrimary: AppColors.inkBg,
      secondary: AppColors.amberDark,
      onSecondary: AppColors.inkBg,
      surface: AppColors.inkSurface,
      onSurface: AppColors.inkText,
      error: AppColors.rose,
      onError: AppColors.inkBg,
      outline: AppColors.inkBorder,
    );

    return _shared(
      scheme: scheme,
      bg: AppColors.inkBg,
      surface: AppColors.inkSurface,
      border: AppColors.inkBorder,
      text: AppColors.inkText,
      textSoft: AppColors.inkTextSoft,
      textFaint: AppColors.inkTextSoft,
      typography: AppTypography.dark,
    );
  }

  static ThemeData _shared({
    required ColorScheme scheme,
    required Color bg,
    required Color surface,
    required Color border,
    required Color text,
    required Color textSoft,
    required Color textFaint,
    required TextTheme typography,
  }) {
    return ThemeData(
      useMaterial3: true,
      colorScheme: scheme,
      scaffoldBackgroundColor: bg,
      canvasColor: bg,
      textTheme: typography,
      // 去掉 ripple，改用极轻的 highlight
      splashFactory: NoSplash.splashFactory,
      splashColor: Colors.transparent,
      highlightColor: text.withValues(alpha: 0.04),
      hoverColor: text.withValues(alpha: 0.03),
      focusColor: text.withValues(alpha: 0.05),
      pageTransitionsTheme: const PageTransitionsTheme(
        builders: {
          TargetPlatform.android: _FadePageTransitionsBuilder(),
          TargetPlatform.iOS: _FadePageTransitionsBuilder(),
        },
      ),
      appBarTheme: AppBarTheme(
        backgroundColor: bg,
        foregroundColor: text,
        elevation: 0,
        scrolledUnderElevation: 0,
        centerTitle: false,
        titleSpacing: 24,
      ),
      cardTheme: const CardThemeData(
        color: Colors.transparent,
        elevation: 0,
        margin: EdgeInsets.zero,
        shape: RoundedRectangleBorder(),
      ),
      dividerTheme: DividerThemeData(color: border, thickness: 0.4, space: 0.4),
      inputDecorationTheme: InputDecorationTheme(
        filled: false,
        contentPadding: EdgeInsets.zero,
        border: InputBorder.none,
        enabledBorder: InputBorder.none,
        focusedBorder: InputBorder.none,
        hintStyle: typography.bodyMedium?.copyWith(color: textFaint),
      ),
      iconTheme: IconThemeData(color: textSoft, size: 18),
      iconButtonTheme: IconButtonThemeData(
        style: IconButton.styleFrom(
          foregroundColor: textSoft,
          shape: const CircleBorder(),
        ),
      ),
      textButtonTheme: TextButtonThemeData(
        style: TextButton.styleFrom(
          foregroundColor: text,
          textStyle: typography.labelLarge,
          padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 6),
          shape: const RoundedRectangleBorder(),
        ),
      ),
      filledButtonTheme: FilledButtonThemeData(
        style: FilledButton.styleFrom(
          backgroundColor: text,
          foregroundColor: bg,
          textStyle: typography.labelLarge,
          padding: const EdgeInsets.symmetric(horizontal: 22, vertical: 14),
          shape: const RoundedRectangleBorder(),
          elevation: 0,
        ),
      ),
      outlinedButtonTheme: OutlinedButtonThemeData(
        style: OutlinedButton.styleFrom(
          foregroundColor: text,
          textStyle: typography.labelLarge,
          side: BorderSide(color: border, width: 0.6),
          padding: const EdgeInsets.symmetric(horizontal: 22, vertical: 14),
          shape: const RoundedRectangleBorder(),
        ),
      ),
      snackBarTheme: SnackBarThemeData(
        backgroundColor: bg,
        contentTextStyle: typography.bodyMedium,
        elevation: 0,
        behavior: SnackBarBehavior.floating,
        shape: const RoundedRectangleBorder(),
      ),
      dialogTheme: DialogThemeData(
        backgroundColor: bg,
        elevation: 0,
        shape: const RoundedRectangleBorder(),
        titleTextStyle: typography.headlineSmall,
        contentTextStyle: typography.bodyMedium,
      ),
    );
  }
}

class _FadePageTransitionsBuilder extends PageTransitionsBuilder {
  const _FadePageTransitionsBuilder();
  @override
  Widget buildTransitions<T>(
    PageRoute<T> route,
    BuildContext context,
    Animation<double> animation,
    Animation<double> secondaryAnimation,
    Widget child,
  ) {
    final curved = CurvedAnimation(
      parent: animation,
      curve: Curves.easeOutCubic,
    );
    return FadeTransition(opacity: curved, child: child);
  }
}
