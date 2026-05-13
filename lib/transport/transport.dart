/// `transport` barrel: 一处导入, 拿到所有模式入口 + Dart 侧助手。
///
/// 用法:
/// ```dart
/// import 'package:fragments/transport/transport.dart';
/// ```
///
/// 重导出 21 个 FRB 生成的模式模块, 加 8 个 Dart 侧辅助 (J/L/N/O/I/Q/V + 错误契约)。
library;

// FRB 生成的模式入口
export '../src/rust/api/transport/common.dart';
export '../src/rust/api/transport/pattern_a_sync.dart';
export '../src/rust/api/transport/pattern_b_async.dart';
export '../src/rust/api/transport/pattern_c_stream.dart';
export '../src/rust/api/transport/pattern_d_cancel.dart';
export '../src/rust/api/transport/pattern_e_progress.dart';
export '../src/rust/api/transport/pattern_f_opaque.dart';
export '../src/rust/api/transport/pattern_g_bytes.dart';
export '../src/rust/api/transport/pattern_h_duplex.dart';
export '../src/rust/api/transport/pattern_i_event_bus.dart';
export '../src/rust/api/transport/pattern_k_pagination.dart';
export '../src/rust/api/transport/pattern_l_idempotent.dart';
export '../src/rust/api/transport/pattern_m_callback.dart';
export '../src/rust/api/transport/pattern_n_semaphore.dart';
export '../src/rust/api/transport/pattern_p_singleton.dart';
export '../src/rust/api/transport/pattern_q_mock.dart';
export '../src/rust/api/transport/pattern_r_json.dart';
export '../src/rust/api/transport/pattern_s_file_handoff.dart';
export '../src/rust/api/transport/pattern_t_panic_safety.dart';
export '../src/rust/api/transport/pattern_u_structured.dart';
export '../src/rust/api/transport/pattern_v_request_meta.dart';

// Dart 侧助手
export 'coalescing.dart';
export 'error_contract.dart';
export 'event_bus.dart';
export 'isolate.dart';
export 'mock.dart';
export 'pool.dart';
export 'request_context.dart';
export 'retry.dart';
