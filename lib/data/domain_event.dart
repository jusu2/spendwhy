/// ADR-0005 — 仅做最小实现：在 Dart 端持久化 append-only `event_log` 表，
/// 由 [FragmentRepository] 在每次写路径内部追加，业务调用方无需感知。
///
/// 此处不引入完整事件溯源：聚合状态仍由 fragments / recoveries 表持有，
/// 事件日志用于审计、时间线视图、未来的同步与撤销。
library;

/// 与 `event_log.event_type` 列对应的常量集合。
/// 字符串值是协议——一旦写入磁盘就不能再改，只能新增。
class DomainEventType {
  DomainEventType._();

  static const fragmentCreated = 'FragmentCreated';
  static const fragmentEdited = 'FragmentEdited';
  static const fragmentStageAdvanced = 'FragmentStageAdvanced';
  static const fragmentDeleted = 'FragmentDeleted';
  static const recoveryRecorded = 'RecoveryRecorded';
  static const recoveryDeleted = 'RecoveryDeleted';

  /// 当前协议版本；不向后兼容的字段调整时 +1。
  static const protocolVersion = 1;
}

/// 一条事件日志记录的不可变投影。
class DomainEvent {
  final int seq;
  final DateTime occurredAt;
  final String eventType;
  final String aggregateId;

  /// JSON-decoded payload。结构由 [eventType] 决定，详见 ADR-0005。
  final Map<String, Object?> payload;

  const DomainEvent({
    required this.seq,
    required this.occurredAt,
    required this.eventType,
    required this.aggregateId,
    required this.payload,
  });

  @override
  String toString() =>
      'DomainEvent(#$seq $eventType aggregate=$aggregateId at=$occurredAt)';
}
