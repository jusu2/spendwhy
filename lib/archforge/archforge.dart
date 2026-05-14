/// ArchForge 跨 ABI 边界对应的 Dart 公开门面。
///
/// 当前包含:
/// - [WireError] / [WireErrorKind] / [WireException] — 跨边界错误 DTO 镜像。
///
/// 后续 phase 会加入: state 管理脚手架、use case 生成器钩子、context 序列化等。
library;

export 'wire_error.dart';
