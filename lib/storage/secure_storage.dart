/// 模式 K (Dart 侧): 系统安全存储封装。
///
/// 包 [FlutterSecureStorage], 提供:
/// - 命名空间隔离 (`namespace` 前缀)
/// - 平台默认配置 (iOS `accessibility=first_unlock`, Android `encryptedSharedPreferences=true`)
/// - 与 storage 库统一的错误类型 (转 [StorageError])
///
/// 适用: Token、用户密码、生物锁后才能解开的值、模式 I 用的主密钥。
/// 不适用: 频繁读写的小数据 (iOS Keychain 在频繁读写下性能差) — 用模式 G。
library;

import 'package:flutter_secure_storage/flutter_secure_storage.dart';

/// 命名空间化的安全存储句柄。多模块共享同一 [FlutterSecureStorage]
/// 时, 用不同 namespace 防 key 冲突。
class StorageSampleSecureStore {
  final FlutterSecureStorage _store;
  final String namespace;

  StorageSampleSecureStore({
    required this.namespace,
    FlutterSecureStorage? store,
  })  : assert(namespace.isNotEmpty, 'namespace must not be empty'),
        _store = store ??
            const FlutterSecureStorage(
              iOptions: IOSOptions(
                accessibility: KeychainAccessibility.first_unlock,
              ),
            );

  String _k(String key) => '$namespace::$key';

  Future<String?> get(String key) => _store.read(key: _k(key));

  Future<void> set(String key, String value) =>
      _store.write(key: _k(key), value: value);

  Future<bool> delete(String key) async {
    await _store.delete(key: _k(key));
    return true;
  }

  /// 列出本 namespace 下所有 key (剥前缀); 用于诊断 / 迁移。
  Future<List<String>> listKeys() async {
    final all = await _store.readAll();
    final prefix = '$namespace::';
    return all.keys
        .where((k) => k.startsWith(prefix))
        .map((k) => k.substring(prefix.length))
        .toList(growable: false);
  }

  /// 清空本 namespace 下所有键。**慎用**: 不影响其他 namespace。
  Future<int> clearNamespace() async {
    final keys = await listKeys();
    for (final k in keys) {
      await _store.delete(key: _k(k));
    }
    return keys.length;
  }
}
