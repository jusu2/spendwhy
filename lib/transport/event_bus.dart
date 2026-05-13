/// 模式 I (Dart 侧): 事件总线订阅辅助。
///
/// Rust 侧 [`subscribeEvents`] 返回一个 Stream; 直接 `listen` 即可。
/// 此助手包装常见需求: 按 topic 过滤、单订阅多分发、自动重连、lagged 上报、dispose。
///
/// 用法:
/// ```dart
/// final bus = TransportEventBus()..connect();
/// bus.on('toast').listen((ev) => showToast(ev.payload));
/// bus.lagged.listen((ev) => log.warn('event bus lagged: ${ev.payload}'));
/// // 应用退出:
/// await bus.dispose();
/// ```
library;

import 'dart:async';

import '../src/rust/api/transport/pattern_i_event_bus.dart'
    show TransportSampleEventDto, subscribeEvents;

/// 与 Rust 侧 `LAGGED_TOPIC` 常量保持一致。
const String kTransportLaggedTopic = '__lagged__';

class TransportEventBus {
  StreamSubscription<TransportSampleEventDto>? _sub;
  final _controller = StreamController<TransportSampleEventDto>.broadcast();
  bool _connecting = false;
  bool _disposed = false;
  final bool _includeSnapshot;

  TransportEventBus({bool includeSnapshot = true})
      : _includeSnapshot = includeSnapshot;

  /// 启动连接。并发调用安全 (用 `_connecting` 互斥)。
  void connect() {
    if (_disposed) {
      throw StateError('TransportEventBus disposed');
    }
    if (_sub != null || _connecting) return;
    _connecting = true;
    try {
      _sub = subscribeEvents(includeSnapshot: _includeSnapshot).listen(
        _controller.add,
        onError: (Object e, StackTrace st) {
          if (!_controller.isClosed) _controller.addError(e, st);
        },
        onDone: () {
          // Rust 侧关闭 (hot-restart 等)。不自动关 controller, 让调用方决定。
          _sub = null;
        },
        cancelOnError: false,
      );
    } finally {
      _connecting = false;
    }
  }

  /// 所有事件 (含 lagged 系统事件)。多订阅者共享同一上游。
  Stream<TransportSampleEventDto> get all => _controller.stream;

  /// 业务事件 (过滤掉系统 lagged)。
  Stream<TransportSampleEventDto> get events =>
      all.where((ev) => ev.topic != kTransportLaggedTopic);

  /// 慢消费者上报。监听此流可在 UI 出现卡顿时告警。
  Stream<TransportSampleEventDto> get lagged =>
      all.where((ev) => ev.topic == kTransportLaggedTopic);

  /// 按 topic 过滤。
  Stream<TransportSampleEventDto> on(String topic) =>
      all.where((ev) => ev.topic == topic);

  bool get isConnected => _sub != null;
  bool get isDisposed => _disposed;

  Future<void> dispose() async {
    if (_disposed) return;
    _disposed = true;
    await _sub?.cancel();
    _sub = null;
    if (!_controller.isClosed) await _controller.close();
  }
}
