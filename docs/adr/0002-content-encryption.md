# ADR 0002: 用户敏感内容的端侧字段级加密

## 状态

Proposed — 计划在 ADR-0003（值对象层）落地之后实施。

## 背景

`Fragment.content` 与 `Recovery.description` 是产品中最敏感的字段：用户的低谷自述与和解过程。当前实现：

- 两者以明文存储在 SQLite (`fragment.content TEXT`, `recovery.description TEXT`)。
- 数据库文件本身不加密。设备失窃、备份泄漏、未来若引入云同步，均会直接暴露明文。
- [EXCEPTIONS.md](../../EXCEPTIONS.md) 已记录该项，但尚未给出技术方案。

## 决策

采用「端侧应用层字段级加密」（field-level encryption at rest），**不**采用整库加密（SQLCipher），原因：

| 维度 | 整库加密 (SQLCipher) | 字段级加密（本方案）|
|---|---|---|
| 实现复杂度 | 接入即用 | 需自定义 mapper |
| 索引能力 | 全部明文存储与索引正常 | 加密字段不可索引，但 spendwhy 不需要全文检索 content |
| 备份/云同步 | 备份后仍是密文文件，可用 | 备份/同步可只同步密文 + nonce，**永不离开设备的明文** |
| 密钥位置 | 单一主密钥保存在 Keychain | 同 |
| 平台原生支持 | sqflite/sqlcipher 在 Android/iOS 良好，桌面端弱 | 纯 Rust + Ring/Aead，全平台一致 |
| 升级 / 轮换 | 整库 rekey 成本高 | 按记录 rekey，渐进 |

字段级加密的额外收益：未来若做"匿名分享"功能，可以只导出明文+签名而不连库；做"导出我的数据"时也更可控。

## 技术方案

### 算法

- **AES-256-GCM**：单一对称算法，足够。
- 实现库：Rust `aes-gcm` (RustCrypto)；Dart 侧不直接做加密，统一走 Rust。
- 每条记录每个字段独立 96-bit 随机 nonce（NIST SP 800-38D 推荐），nonce 与密文一并存储。
- AAD（Additional Authenticated Data）= `record_id || field_name || schema_version`，防止跨记录/跨字段重放。

### 密钥管理

- **主密钥（DEK, Data Encryption Key）**：32 字节随机，应用首次启动生成。
- **保管**：
  - iOS: Keychain (`kSecAttrAccessibleAfterFirstUnlockThisDeviceOnly`)
  - Android: Keystore (StrongBox if available, fall back to TEE) wrapping a software AES key
  - Windows: DPAPI (user scope)
  - macOS/Linux: 暂存配置目录（后续 ADR 单独处理）
- Rust 通过 `flutter_secure_storage` 已暴露的 channel 读取；如需更紧密的硬件后端，再开一道 native channel。
- **密钥轮换**：表 `crypto_meta(key_id, created_at, retired_at)`；密文 prefix 一个 `key_id`（1 byte），解密时按 id 查 key。
- **初始版本**只持有 `key_id = 1`，但代码路径预留多 key。

### 持久化模型

```sql
-- 替换原 content TEXT 列
ALTER TABLE fragment
  ADD COLUMN content_cipher BLOB,
  ADD COLUMN content_nonce  BLOB,
  ADD COLUMN content_key_id INTEGER;
-- 数据迁移：取出旧 content -> 加密 -> 写回 -> 删除旧列
```

迁移在 ADR-0004（DB 正规化）的迁移基础设施里执行，作为一个独立 migration step。

### Rust API 边界

```rust
// rust/src/crypto/mod.rs
pub struct Sealed { pub key_id: u8, pub nonce: [u8;12], pub cipher: Vec<u8> }

pub trait Vault {
    fn seal(&self, plain: &[u8], aad: &[u8]) -> AppResult<Sealed>;
    fn open(&self, sealed: &Sealed, aad: &[u8]) -> AppResult<Vec<u8>>;
}

pub struct LocalVault { keys: HashMap<u8, [u8;32]> }
```

DTO 层维持明文 String（FFI 上明文进出，密文只活在 SQLite + Rust 持久化内部），原因：

- UI 不应感知加密存在。
- FRB 处理 BLOB 麻烦且零收益。
- 安全边界 = SQLite 文件，不是 FFI 边界。

## 约束 / 风险

1. **迁移失败 = 数据丢失风险**：迁移必须在事务中，失败回滚原列保留。
2. **密钥丢失 = 数据永久不可读**：Keychain 在用户重置设备时可能丢失；后续需提供「导出主密钥到本地 PDF 二维码」的可选恢复机制。
3. **性能**：~10KB 文本 AES-GCM 单次 < 100µs，可忽略。
4. **AAD 校验严格**：record_id 更名 / 字段重命名将导致旧密文无法解密；schema 演进需走 ADR-0004 的迁移注册表。

## 验收

- 单元测试：seal/open roundtrip、tampered cipher 拒绝、错误 AAD 拒绝、不同 key_id 路由。
- 迁移测试：v0 明文库 → v1 加密库 → 全量读出明文相等。
- 性能基线：1000 条 content 解密 < 50ms（典型移动设备）。

## 退出 / 演进

- 后续可叠加 KEK/DEK 分层（让用户在「换设备」时输入纸质 24 字 BIP-39 助记词重建 KEK）。
- 不在本 ADR 范围内：消息传输加密、E2EE 同步。
