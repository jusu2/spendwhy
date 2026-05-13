/// `storage` barrel: 一处导入, 拿到所有 18 个存储模式入口 + Dart 侧助手。
///
/// 用法:
/// ```dart
/// import 'package:fragments/storage/storage.dart';
/// ```
///
/// 重导出 10 个 FRB 生成的 Rust 模式模块 (A..J), 加 8 个 Dart 侧模式 (K..R)
/// 与错误契约。
library;

// FRB 生成的模式入口 (A..J)
export '../src/rust/api/storage/common.dart';
export '../src/rust/api/storage/pattern_a_memory.dart';
export '../src/rust/api/storage/pattern_b_atomic_file.dart';
export '../src/rust/api/storage/pattern_c_blob.dart';
export '../src/rust/api/storage/pattern_d_snapshot.dart';
export '../src/rust/api/storage/pattern_e_event_log.dart';
export '../src/rust/api/storage/pattern_f_ordered_kv.dart';
export '../src/rust/api/storage/pattern_g_settings.dart';
export '../src/rust/api/storage/pattern_h_persistent_cache.dart';
export '../src/rust/api/storage/pattern_i_encryption.dart';
export '../src/rust/api/storage/pattern_j_backup.dart';

// Dart 侧模式与助手 (K..R + 错误契约镜像)
export 'backfill.dart';
export 'cache_combinator.dart';
export 'error_contract.dart';
export 'migration.dart';
export 'outbox.dart';
export 'secure_storage.dart';
export 'soft_delete.dart';
export 'sql.dart';
export 'tenant.dart';
