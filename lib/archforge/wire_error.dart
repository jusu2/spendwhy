/// Dart 镜像: `archforge-ffi` 的 `WireError` DTO。
///
/// 跨 ABI 边界 (Rust → Dart) 反序列化 archforge 错误的唯一入口。
/// 形状契约见 [`docs/ffi/error-contract.md`](../../docs/ffi/error-contract.md)。
///
/// 设计要点:
/// - [WireError.fromJson] **永不抛**。字段缺失走默认; 陌生 `kind` 走
///   [WireErrorKind.unknown]。这是跨语言边界对客户端最友好的约定 —
///   一个老版本 Dart 客户端不能因为 Rust 端新增了 variant 就崩。
/// - [WireException] 把 [WireError] 包装成可 `throw` 的对象, 调用方
///   习惯的 `try / on WireException catch (e)` 模式立刻能用。
/// - **绝对不实现 `toJson`**: Dart 端没有理由把错误发回 Rust;
///   错误的传递是单向的。详见 `docs/ffi/error-contract.md` §4。
library;

/// 稳定的错误分类。字符串值与 Rust 端 `WireErrorKind` 序列化形式逐一对应。
enum WireErrorKind {
  /// 资源不存在。
  notFound('not_found'),

  /// 状态冲突 (重复键 / 乐观锁失败 / 业务态不允许)。
  conflict('conflict'),

  /// 入参违反域不变量。
  invalid('invalid'),

  /// 外部依赖暂不可用; 可重试。
  unavailable('unavailable'),

  /// 调用方无权限。
  forbidden('forbidden'),

  /// `Context.deadline` 到期。
  deadlineExceeded('deadline_exceeded'),

  /// 不可恢复的内部错误。可能由 Rust 端 panic 触发, 看 [WireError.isPanic]。
  internal('internal'),

  /// Wire 上出现了本客户端不认识的 kind。一律按 [internal] 处理, 但保留原始字符串供日志。
  unknown('unknown');

  const WireErrorKind(this.wireValue);

  /// 序列化时使用的 `snake_case` 字符串。
  final String wireValue;

  /// 反向查表; 未知一律返回 [WireErrorKind.unknown]。
  static WireErrorKind fromWire(String? raw) {
    if (raw == null) return WireErrorKind.unknown;
    for (final v in WireErrorKind.values) {
      if (v.wireValue == raw) return v;
    }
    return WireErrorKind.unknown;
  }
}

/// Wire-safe 错误 DTO。和 `archforge-ffi::WireError` 字段一一对应。
class WireError {
  const WireError({
    required this.kind,
    required this.message,
    this.isPanic = false,
    this.rawKind,
  });

  /// 稳定分类。
  final WireErrorKind kind;

  /// 人类可读、已脱敏的错误文本。**不要**用作 match 条件 —
  /// 文案可能在 patch 版本里改; 真要分支请按 [kind]。
  final String message;

  /// `true` 表示 Rust 端用 `guard_*` 捕了一个 panic。客户端应该走崩溃上报通道。
  final bool isPanic;

  /// 当 `kind` 解码为 [WireErrorKind.unknown] 时, 这里保留 Rust 发来的原始字符串,
  /// 便于日志/告警里看到具体新分类。
  final String? rawKind;

  /// 解析 JSON map (通常是 FRB / HTTP 返回的 `Map<String, dynamic>`)。
  /// **永不抛**。坏数据走默认值。
  factory WireError.fromJson(Map<String, dynamic> json) {
    final rawKind = json['kind'];
    final kind = WireErrorKind.fromWire(rawKind is String ? rawKind : null);
    final message = json['message'];
    final isPanic = json['is_panic'];
    return WireError(
      kind: kind,
      message: message is String ? message : '',
      isPanic: isPanic is bool ? isPanic : false,
      rawKind: (kind == WireErrorKind.unknown && rawKind is String)
          ? rawKind
          : null,
    );
  }

  /// `unavailable` / `deadline_exceeded` 视为可重试; 其它种类 (含 `internal` /
  /// 含 panic) 不要自动重试 —— internal 通常意味着 bug, 重试只会放大问题。
  bool get isRetriable =>
      kind == WireErrorKind.unavailable ||
      kind == WireErrorKind.deadlineExceeded;

  /// 是否是用户输入校验错误 (适合直接显示在输入框旁)。
  bool get isUserInput => kind == WireErrorKind.invalid;

  /// 是否需要走崩溃上报通道 (Sentry / Crashlytics)。
  bool get shouldReportAsCrash => isPanic;

  @override
  String toString() {
    final tag = isPanic ? '${kind.wireValue}|panic' : kind.wireValue;
    final extra = rawKind != null ? ' (rawKind=$rawKind)' : '';
    return 'WireError[$tag]$extra: $message';
  }

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is WireError &&
          other.kind == kind &&
          other.message == message &&
          other.isPanic == isPanic &&
          other.rawKind == rawKind;

  @override
  int get hashCode => Object.hash(kind, message, isPanic, rawKind);
}

/// `throw`-able 包装。用于 `try { ... } on WireException catch (e) { ... }` 习惯写法。
class WireException implements Exception {
  WireException(this.error);

  final WireError error;

  WireErrorKind get kind => error.kind;
  String get message => error.message;
  bool get isPanic => error.isPanic;
  bool get isRetriable => error.isRetriable;

  @override
  String toString() => error.toString();
}
