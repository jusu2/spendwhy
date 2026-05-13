import 'package:flutter/material.dart';
import 'package:google_fonts/google_fonts.dart';

import 'app_colors.dart';

/// 字体策略（文艺日记感）：
/// - 标题与正文均以衬线（Noto Serif SC）为主，建立"纸本日记"基调
/// - 时间戳/标签用极小字号无衬线（Noto Sans SC），形成层级对比
class AppTypography {
  AppTypography._();

  static TextTheme buildTextTheme({required Color text, required Color soft}) {
    final serif = GoogleFonts.notoSerifScTextTheme();
    final sans = GoogleFonts.notoSansScTextTheme();

    TextStyle s(
      TextStyle? base, {
      required double size,
      FontWeight weight = FontWeight.w400,
      double height = 1.5,
      double letter = 0.0,
      Color? color,
    }) {
      return (base ?? const TextStyle()).copyWith(
        fontSize: size,
        fontWeight: weight,
        height: height,
        letterSpacing: letter,
        color: color ?? text,
      );
    }

    return TextTheme(
      displayLarge: s(
        serif.displayLarge,
        size: 36,
        weight: FontWeight.w400,
        height: 1.25,
        letter: 1.2,
      ),
      displayMedium: s(
        serif.displayMedium,
        size: 28,
        weight: FontWeight.w400,
        height: 1.3,
        letter: 1.0,
      ),
      displaySmall: s(
        serif.displaySmall,
        size: 24,
        weight: FontWeight.w400,
        height: 1.35,
        letter: 0.6,
      ),
      headlineLarge: s(
        serif.headlineLarge,
        size: 22,
        weight: FontWeight.w400,
        height: 1.4,
        letter: 0.4,
      ),
      headlineMedium: s(
        serif.headlineMedium,
        size: 19,
        weight: FontWeight.w400,
        height: 1.5,
        letter: 0.4,
      ),
      headlineSmall: s(
        serif.headlineSmall,
        size: 17,
        weight: FontWeight.w400,
        height: 1.5,
        letter: 0.3,
      ),
      titleLarge: s(
        serif.titleLarge,
        size: 15,
        weight: FontWeight.w500,
        height: 1.4,
        letter: 0.6,
      ),
      titleMedium: s(
        sans.titleMedium,
        size: 11,
        weight: FontWeight.w500,
        height: 1.4,
        letter: 2.0,
        color: soft,
      ),
      titleSmall: s(
        sans.titleSmall,
        size: 10,
        weight: FontWeight.w500,
        height: 1.4,
        letter: 1.6,
        color: soft,
      ),
      bodyLarge: s(
        serif.bodyLarge,
        size: 16,
        weight: FontWeight.w400,
        height: 1.85,
        letter: 0.4,
      ),
      bodyMedium: s(
        serif.bodyMedium,
        size: 14.5,
        weight: FontWeight.w400,
        height: 1.85,
        letter: 0.35,
      ),
      bodySmall: s(
        sans.bodySmall,
        size: 12,
        weight: FontWeight.w400,
        height: 1.6,
        letter: 0.6,
        color: soft,
      ),
      labelLarge: s(
        sans.labelLarge,
        size: 12,
        weight: FontWeight.w500,
        height: 1.3,
        letter: 1.4,
      ),
      labelMedium: s(
        sans.labelMedium,
        size: 11,
        weight: FontWeight.w400,
        height: 1.3,
        letter: 1.4,
        color: soft,
      ),
      labelSmall: s(
        sans.labelSmall,
        size: 10,
        weight: FontWeight.w400,
        height: 1.3,
        letter: 1.6,
        color: soft,
      ),
    );
  }

  static TextTheme get light =>
      buildTextTheme(text: AppColors.ink, soft: AppColors.inkSoft);

  static TextTheme get dark =>
      buildTextTheme(text: AppColors.inkText, soft: AppColors.inkTextSoft);
}
