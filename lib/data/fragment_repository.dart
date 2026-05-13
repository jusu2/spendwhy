import '../models/fragment.dart';
import '../models/recovery.dart';

/// 碎片/恢复持久化层的抽象边界。
///
/// 默认实现是 sqflite（[`AppDatabase`]），单元测试可使用内存版替身。
/// Provider 只依赖此接口，从而：
/// - 不绑死 sqflite，将来切到 Drift / Rust 端 SQLite 也只换实现；
/// - 测试无需依赖 path_provider / platform channel。
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
}
