import '../models/fragment.dart';
import '../models/recovery.dart';
import 'domain_event.dart';

/// 碎片/恢复持久化层的抽象边界。
///
/// 默认实现是 sqflite（[`AppDatabase`]），单元测试可使用内存版替身。
/// Provider 只依赖此接口，从而：
/// - 不绑死 sqflite，将来切到 Drift / Rust 端 SQLite 也只换实现；
/// - 测试无需依赖 path_provider / platform channel。
///
/// **ADR-0004 语义**：
/// - 读 API 默认排除已软删（`deleted_at IS NOT NULL`）的行。
/// - `deleteFragment` 为**软删**（保留行 + 设置 `deleted_at`），
///   以便事件日志、撤销与同步在未来仍能引用。
/// - 写 API 在事务内追加 [DomainEvent]（ADR-0005）。
abstract class FragmentRepository {
  Future<List<Fragment>> listFragments({int? limit});
  Future<Fragment?> getFragment(String id);
  Future<void> insertFragment(Fragment f);
  Future<void> updateFragment(Fragment f);
  Future<void> deleteFragment(String id);

  Future<List<Recovery>> listRecoveries({int? limit});
  Future<List<Recovery>> recoveriesForFragment(String fragmentId);

  /// 事务：一次性写入一条 recovery + N 条已推进阶段的 fragment。
  /// 任一失败应整体回滚。
  Future<void> recordRecoveryTx({
    required Recovery recovery,
    required List<Fragment> advancedFragments,
  });

  /// ADR-0005 — 读事件日志（按 seq 升序）。
  ///
  /// - [aggregateId] 非空时只返回该聚合的事件（时间线视图）。
  /// - [limit] 不传时返回全部。
  Future<List<DomainEvent>> listEvents({String? aggregateId, int? limit});
}
