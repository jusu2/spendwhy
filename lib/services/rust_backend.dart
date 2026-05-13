import 'package:flutter/foundation.dart' show debugPrint, kDebugMode;
import 'package:flutter_rust_bridge/flutter_rust_bridge_for_generated.dart'
    show PlatformInt64Util;

import '../models/fragment.dart';
import '../models/recovery.dart';
import '../src/rust/api/dto.dart' as dto;
import '../src/rust/api/fade.dart' as fade_api;
import '../src/rust/api/recovery.dart' as recovery_api;
import '../src/rust/api/view.dart' as view_api;
import '../src/rust/frb_generated.dart';

/// Rust 业务核心运行时门面：把 Dart 模型转成 DTO，并集中处理 schema 校验。
class RustBackend {
  RustBackend._();

  /// 当前 Dart 端期望的 DTO schema 版本。提升时需要同时升级 Rust 端常量。
  static const int expectedFragmentSchema = 1;
  static const int expectedRecoverySchema = 1;

  /// 应用启动时调用一次。失败即抛出，阻断启动。
  static Future<void> init() async {
    await RustLib.init();
    final fv = dto.supportedFragmentSchemaVersion();
    final rv = dto.supportedRecoverySchemaVersion();
    if (fv != expectedFragmentSchema || rv != expectedRecoverySchema) {
      throw StateError(
        '[RustBackend] FRB DTO schema mismatch: '
        'fragment=$fv (expected $expectedFragmentSchema), '
        'recovery=$rv (expected $expectedRecoverySchema). '
        '请重新运行 flutter_rust_bridge_codegen generate。',
      );
    }
    if (kDebugMode) {
      debugPrint('[RustBackend] initialized (fragment=$fv, recovery=$rv)');
    }
  }

  // === DTO 转换：领域 -> FRB DTO ============================================

  static dto.FragmentDto toFragmentDto(Fragment f) => dto.FragmentDto(
        schemaVersion: expectedFragmentSchema,
        id: f.id,
        createdAtMs: PlatformInt64Util.from(
          f.createdAt.toUtc().millisecondsSinceEpoch,
        ),
        intensity: f.intensity.value,
        fadePeriodDays: f.fadePeriod.days,
        stage: f.stage.code,
      );

  static dto.RecoveryDto toRecoveryDto(Recovery r) => dto.RecoveryDto(
        schemaVersion: expectedRecoverySchema,
        id: r.id,
        createdAtMs: PlatformInt64Util.from(
          r.createdAt.toUtc().millisecondsSinceEpoch,
        ),
        intensity: r.intensity,
        description: r.description,
        relatedFragmentIds: r.relatedFragmentIds,
      );

  // === Use cases ============================================================

  /// 把当前碎片/恢复列表喂给 Rust，返回带 fade_level 的视图模型 + 和解分数。
  static dto.HomeViewDto buildHomeView({
    required List<Fragment> fragments,
    required List<Recovery> recoveries,
    DateTime? now,
  }) {
    final n = (now ?? DateTime.now()).toUtc().millisecondsSinceEpoch;
    return view_api.buildHomeView(
      fragments: fragments.map(toFragmentDto).toList(growable: false),
      recoveries: recoveries.map(toRecoveryDto).toList(growable: false),
      nowMs: PlatformInt64Util.from(n),
    );
  }

  /// 单条碎片清晰度。
  static double fadeLevel(
    Fragment fragment,
    List<Recovery> recoveries, {
    DateTime? now,
  }) {
    final n = (now ?? DateTime.now()).toUtc().millisecondsSinceEpoch;
    return fade_api.fadeLevel(
      fragment: toFragmentDto(fragment),
      recoveries: recoveries.map(toRecoveryDto).toList(growable: false),
      nowMs: PlatformInt64Util.from(n),
    );
  }

  /// 整体和解分数。
  static double growthScore({
    required List<Fragment> fragments,
    required List<Recovery> recoveries,
    DateTime? now,
  }) {
    final n = (now ?? DateTime.now()).toUtc().millisecondsSinceEpoch;
    return fade_api.growthScore(
      fragments: fragments.map(toFragmentDto).toList(growable: false),
      recoveries: recoveries.map(toRecoveryDto).toList(growable: false),
      nowMs: PlatformInt64Util.from(n),
    );
  }

  /// 记录一次恢复事件。Rust 端校验输入并应用业务规则
  /// （例如 outburst 阶段的相关碎片应推进到 recovery）。
  static dto.RecordRecoveryOutcomeDto recordRecovery({
    required Recovery recovery,
    required List<Fragment> relatedFragments,
  }) {
    return recovery_api.recordRecovery(
      recovery: toRecoveryDto(recovery),
      relatedFragments:
          relatedFragments.map(toFragmentDto).toList(growable: false),
    );
  }
}
