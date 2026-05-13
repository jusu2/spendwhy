import 'dart:math';
import 'package:flutter/material.dart';

/// 纸张颗粒背景：在主背景上叠一层柔和的颗粒和淡纹理
/// 不依赖图片资源，纯 Canvas 绘制
class GrainBackground extends StatelessWidget {
  final Widget child;
  final double opacity;

  const GrainBackground({super.key, required this.child, this.opacity = 0.04});

  @override
  Widget build(BuildContext context) {
    return Stack(
      children: [
        Positioned.fill(
          child: IgnorePointer(
            child: CustomPaint(painter: _GrainPainter(opacity: opacity)),
          ),
        ),
        child,
      ],
    );
  }
}

class _GrainPainter extends CustomPainter {
  final double opacity;
  _GrainPainter({required this.opacity});

  @override
  void paint(Canvas canvas, Size size) {
    final rng = Random(42); // 固定种子保证不抖动
    final paint = Paint()
      ..color = const Color(0xFF2A2A28).withValues(alpha: opacity);
    final count = (size.width * size.height / 220).round();
    for (var i = 0; i < count; i++) {
      final x = rng.nextDouble() * size.width;
      final y = rng.nextDouble() * size.height;
      final r = rng.nextDouble() * 0.5 + 0.15;
      canvas.drawCircle(Offset(x, y), r, paint);
    }
  }

  @override
  bool shouldRepaint(covariant _GrainPainter old) => old.opacity != opacity;
}
