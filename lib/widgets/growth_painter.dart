import 'dart:math' as math;
import 'package:flutter/material.dart';

import '../models/fragment.dart';
import '../models/recovery.dart';
import '../theme/app_colors.dart';

/// 成长线 Canvas：在时间轴上绘制
/// - 灰色小圆点：碎片（按 fadeLevel 控制透明度）
/// - 暖色光晕：恢复事件（向四周扩散）
/// - 平滑曲线：和解分数走向
class GrowthPainter extends CustomPainter {
  final List<Fragment> fragments;
  final List<Recovery> recoveries;
  final DateTime start;
  final DateTime end;
  final Color textColor;
  final bool isDark;

  GrowthPainter({
    required this.fragments,
    required this.recoveries,
    required this.start,
    required this.end,
    required this.textColor,
    required this.isDark,
  });

  double _xFor(DateTime t, Size size, EdgeInsets pad) {
    final total = end.millisecondsSinceEpoch - start.millisecondsSinceEpoch;
    if (total <= 0) return pad.left;
    final frac =
        (t.millisecondsSinceEpoch - start.millisecondsSinceEpoch) / total;
    return pad.left + frac.clamp(0.0, 1.0) * (size.width - pad.horizontal);
  }

  @override
  void paint(Canvas canvas, Size size) {
    const pad = EdgeInsets.fromLTRB(8, 30, 8, 30);
    final centerY = pad.top + (size.height - pad.vertical) / 2;

    // === 中轴与上下描涿线 ===
    final axisPaint = Paint()
      ..color = (isDark ? AppColors.inkBorder : AppColors.paperEdge)
      ..strokeWidth = 0.4;
    canvas.drawLine(
      Offset(pad.left, centerY),
      Offset(size.width - pad.right, centerY),
      axisPaint,
    );

    // === 恢复事件的光晕 ===
    for (final r in recoveries) {
      final x = _xFor(r.createdAt, size, pad);
      final radius = 16.0 + r.intensity * 4.0;
      final glow = RadialGradient(
        colors: [
          (isDark ? AppColors.amberDark : AppColors.amber).withValues(
            alpha: 0.4,
          ),
          (isDark ? AppColors.amberDark : AppColors.amber).withValues(
            alpha: 0.0,
          ),
        ],
      );
      final rect = Rect.fromCircle(center: Offset(x, centerY), radius: radius);
      final paint = Paint()..shader = glow.createShader(rect);
      canvas.drawCircle(Offset(x, centerY), radius, paint);

      final corePaint = Paint()
        ..color = (isDark ? AppColors.amberDark : AppColors.amber);
      canvas.drawCircle(Offset(x, centerY), 2.5, corePaint);
    }

    // === 碎片点 ===
    for (final f in fragments) {
      final x = _xFor(f.createdAt, size, pad);
      final amplitude = (size.height - pad.vertical) * 0.4;
      final dy = amplitude * (f.intensity.value / 5.0);
      final y = centerY + dy;
      final clarity = f.fadeLevel.clamp(0.0, 1.0);
      final paint = Paint()
        ..color = (isDark ? AppColors.inkText : AppColors.ink).withValues(
          alpha: 0.12 + 0.4 * clarity,
        );
      canvas.drawCircle(Offset(x, y), 1.6 + clarity * 1.4, paint);

      // 与中轴的轻线
      final linePaint = Paint()
        ..color = paint.color.withValues(alpha: 0.10)
        ..strokeWidth = 0.4;
      canvas.drawLine(Offset(x, centerY), Offset(x, y), linePaint);
    }

    // === 和解分数曲线（基于时间窗的滑动平均淡化）===
    final samples = 80;
    final path = Path();
    final paint = Paint()
      ..color = (isDark ? AppColors.mistDark : AppColors.mist).withValues(
        alpha: 0.7,
      )
      ..strokeWidth = 1.0
      ..style = PaintingStyle.stroke;

    final totalMs = end.millisecondsSinceEpoch - start.millisecondsSinceEpoch;
    if (totalMs > 0) {
      for (var i = 0; i <= samples; i++) {
        final t = start.add(
          Duration(milliseconds: (totalMs * i / samples).round()),
        );
        final score = _snapshotGrowthScore(t);
        final x = pad.left + (i / samples) * (size.width - pad.horizontal);
        final y =
            centerY - (score / 100.0) * (size.height - pad.vertical) * 0.35;
        if (i == 0) {
          path.moveTo(x, y);
        } else {
          path.lineTo(x, y);
        }
      }
      canvas.drawPath(path, paint);
    }
  }

  double _snapshotGrowthScore(DateTime t) {
    if (fragments.isEmpty) return 0;
    double weightedClarity = 0;
    double weightSum = 0;
    for (final f in fragments) {
      if (f.createdAt.isAfter(t)) continue;
      final ageDays = t.difference(f.createdAt).inHours / 24.0;
      final timeFactor = (ageDays / f.fadePeriod.days).clamp(0.0, 1.0);
      double recoveryFactor = 0;
      for (final r in recoveries) {
        if (r.createdAt.isAfter(t)) continue;
        if (!r.relatedFragmentIds.contains(f.id)) continue;
        final since = t.difference(r.createdAt).inHours / 24.0;
        recoveryFactor += (r.intensity / 5.0) * 0.25 * math.exp(-since / 60.0);
      }
      final clarity = (1 - timeFactor - recoveryFactor).clamp(0.0, 1.0);
      final w = f.intensity.value.toDouble();
      weightedClarity += clarity * w;
      weightSum += w;
    }
    if (weightSum == 0) return 0;
    return ((1 - weightedClarity / weightSum) * 100).clamp(0, 100);
  }

  @override
  bool shouldRepaint(covariant GrowthPainter old) =>
      old.fragments != fragments ||
      old.recoveries != recoveries ||
      old.start != start ||
      old.end != end ||
      old.isDark != isDark;
}
