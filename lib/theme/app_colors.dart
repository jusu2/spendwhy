import 'package:flutter/material.dart';

/// 碎片 - 文艺风格色板
/// 设计语言：低饱和暖灰、雾蓝、纸张、柔光晕
class AppColors {
  AppColors._();

  // === 浅色模式 ===
  static const paper = Color(0xFFF5F1EA); // 主背景，纸张色
  static const paperSoft = Color(0xFFFBF7F1); // 卡片表面
  static const paperEdge = Color(0xFFE8E2D8); // 描边、分割线
  static const ink = Color(0xFF2A2A28); // 主文本，墨色
  static const inkSoft = Color(0xFF6B6661); // 次文本
  static const inkFaint = Color(0xFFA59E94); // 提示、占位

  static const mist = Color(0xFF9FB4C7); // 雾蓝，强调色
  static const mistSoft = Color(0xFFC8D4DE); // 雾蓝浅
  static const amber = Color(0xFFD4B896); // 暖光，恢复事件
  static const rose = Color(0xFFB87C7C); // 提示色（非错误）

  // === 深色模式 ===
  static const inkBg = Color(0xFF1A1816);
  static const inkSurface = Color(0xFF242220);
  static const inkBorder = Color(0xFF35322E);
  static const inkText = Color(0xFFE8E2D8);
  static const inkTextSoft = Color(0xFFA59E94);
  static const mistDark = Color(0xFF6B8AA8);
  static const amberDark = Color(0xFFB89868);
}
