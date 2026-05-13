/// 模式 O (Dart 侧): 多租户 namespace 分区。
///
/// 把任意 KV 后端 (`Future<V?> Function(K)` getter + `Future<void> Function(K, V)`
/// setter) 包一层 tenant 前缀, 同一物理存储里隔离多个用户/工作区/账号。
///
/// 适用: workspace 切换、试用账号、SaaS 多租户隔离。
/// 不适用: 强隔离 (用单独的 DB 文件 + 密钥); 跨租户查询 (这层会拦截)。
///
/// 关键约束:
/// - tenantId 校验 (`^[a-z0-9_-]{1,64}$`), 防 path-traversal / 注入。
/// - key 前缀 `tenant:{id}:` — 即便底层 key 是 bytes, 也走 utf-8 编码前缀。
library;

import 'dart:convert';

class StorageSampleTenantContext {
  final String tenantId;
  late final List<int> _prefixBytes;
  late final String _prefixStr;

  StorageSampleTenantContext(this.tenantId) {
    _validate(tenantId);
    _prefixStr = 'tenant:$tenantId:';
    _prefixBytes = utf8.encode(_prefixStr);
  }

  String wrapKey(String key) => '$_prefixStr$key';

  List<int> wrapKeyBytes(List<int> key) =>
      [..._prefixBytes, ...key];

  /// 反向解出业务 key; 若不属于本租户, 抛 [StateError]。
  String unwrapKey(String stored) {
    if (!stored.startsWith(_prefixStr)) {
      throw StateError('key does not belong to tenant $tenantId: $stored');
    }
    return stored.substring(_prefixStr.length);
  }

  bool belongs(String stored) => stored.startsWith(_prefixStr);

  static void _validate(String id) {
    if (id.isEmpty || id.length > 64) {
      throw ArgumentError.value(id, 'tenantId', 'length 1..=64');
    }
    final ok = RegExp(r'^[a-z0-9_-]+$').hasMatch(id);
    if (!ok) {
      throw ArgumentError.value(
        id,
        'tenantId',
        'must match [a-z0-9_-]',
      );
    }
  }
}

/// 包装任意 KV<String, V> 后端, 强制所有读写带 tenant 前缀。
class StorageSampleTenantedKv<V> {
  final StorageSampleTenantContext tenant;
  final Future<V?> Function(String key) _read;
  final Future<void> Function(String key, V value) _write;
  final Future<bool> Function(String key)? _delete;

  StorageSampleTenantedKv({
    required this.tenant,
    required Future<V?> Function(String key) read,
    required Future<void> Function(String key, V value) write,
    Future<bool> Function(String key)? delete,
  })  : _read = read,
        _write = write,
        _delete = delete;

  Future<V?> get(String key) => _read(tenant.wrapKey(key));

  Future<void> put(String key, V value) => _write(tenant.wrapKey(key), value);

  Future<bool> delete(String key) async {
    final fn = _delete;
    if (fn == null) {
      throw UnsupportedError('delete not configured for this TenantedKv');
    }
    return fn(tenant.wrapKey(key));
  }
}
