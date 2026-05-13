//! Flutter ↔ Rust 数据传输模式标本库。
//!
//! 本子模块是项目的"模式参考库"。它**不参与业务**，仅作为
//! 「我想把 X 从 Dart 送到 Rust（或反向）」时的可拷贝最小实现集合。
//!
//! 设计原则:
//! - 零业务耦合: 不引用 `crate::domain` / `crate::application`。
//! - 解耦边界类型: 公共信号类型（错误、进度、取消）放在 [`common`]。
//! - 每个 `pattern_*` 文件顶部 `//!` 第一行写"场景关键词 → 选我"，方便 grep。
//!
//! 选择哪个模式? 看 `docs/transport/decision-tree.md` 或下表:
//! - 一次性纯计算          → [`pattern_a_sync`]
//! - 一次性业务用例 IO     → [`pattern_b_async`]
//! - 持续事件流           → [`pattern_c_stream`]
//! - 可取消的长任务       → [`pattern_d_cancel`]
//! - 进度 + 取消          → [`pattern_e_progress`]
//! - 长生命周期句柄       → [`pattern_f_opaque`]
//! - 大二进制 / 零拷贝    → [`pattern_g_bytes`]
//! - 双向命令/事件流      → [`pattern_h_duplex`]
//! - 全局事件总线         → [`pattern_i_event_bus`]
//! - 分页 / Keyset Cursor → [`pattern_k_pagination`]
//! - 幂等接收方           → [`pattern_l_idempotent`]
//! - Dart 注入回调（DI）  → [`pattern_m_callback`]
//! - 并发限流             → [`pattern_n_semaphore`]
//! - 进程级单例           → [`pattern_p_singleton`]
//! - Mock / 测试          → [`pattern_q_mock`]
//! - JSON 动态 schema     → [`pattern_r_json`]
//! - 大文件路径握手       → [`pattern_s_file_handoff`]
//! - panic 安全 / FFI 保护 → [`pattern_t_panic_safety`]
//! - 结构化并发 / fan-out  → [`pattern_u_structured`]
//! - 请求横切元数据         → [`pattern_v_request_meta`]

pub mod common;

pub mod pattern_a_sync;
pub mod pattern_b_async;
pub mod pattern_c_stream;
pub mod pattern_d_cancel;
pub mod pattern_e_progress;
pub mod pattern_f_opaque;
pub mod pattern_g_bytes;
pub mod pattern_h_duplex;
pub mod pattern_i_event_bus;
pub mod pattern_k_pagination;
pub mod pattern_l_idempotent;
pub mod pattern_m_callback;
pub mod pattern_n_semaphore;
pub mod pattern_p_singleton;
pub mod pattern_q_mock;
pub mod pattern_r_json;
pub mod pattern_s_file_handoff;
pub mod pattern_t_panic_safety;
pub mod pattern_u_structured;
pub mod pattern_v_request_meta;
