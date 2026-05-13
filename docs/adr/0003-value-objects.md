# ADR 0003: 领域值对象与 "parse, don't validate"

## 状态

Accepted — 与本 ADR 一同提交的代码即为首版实施。

## 背景

当前领域类型用原始类型表达约束，校验是事后步骤：

```rust
// Rust (旧)
pub struct Fragment {
    pub id: String,          // 可以为空
    pub intensity: u8,       // 可以是 0 或 200
    pub fade_period_days: u32, // 可以是 0
    pub stage: Stage,
}
impl Fragment { pub fn validate(&self) -> AppResult<()> { ... } }
```

```dart
// Dart (旧)
class Recovery {
  final int intensity;  // 1..5? 谁知道
  final List<String> relatedFragmentIds;
}
```

问题：

1. 调用方拿到一个 `Fragment` 不能假设它合法 — 必须随时 `.validate()`。
2. 不变式分散在 `validate()` 里，构造路径多，容易绕过。
3. `intensity: u8` 与 `intensity: int` 跨边界丢失语义；Dart `Recovery.intensity` 比 `Fragment.intensity` 还退化为 `int`。
4. `id: String` 不带类型信息，`FragmentId` 与 `RecoveryId` 互可赋值。

## 决策

采用 *parse, don't validate*（Alexis King, 2019）的设计哲学：

> 把"原始数据 → 受约束类型"视为一次性的解析操作。解析成功的对象天然合法，类型本身即文档。

### 引入的值对象

| 名字 | 约束 | Rust | Dart |
|---|---|---|---|
| `FragmentId` | 非空 String | `pub struct FragmentId(String)` | `extension type FragmentId(String)` |
| `RecoveryId` | 非空 String | 同上 | 同上 |
| `Intensity` | 1..=5 | `pub struct Intensity(u8)` | 已有 enum，加严校验 |
| `FadePeriodDays` | > 0 (实际 {180,270,365}) | `pub struct FadePeriodDays(u32)` | 已有 `FadePeriod` enum |
| `AppTime` | UTC ms i64，禁负 | `pub struct AppTime(i64)` | `extension type AppTime(int)` |
| `NonEmptyText` | trim 后非空 | `pub struct NonEmptyText(String)` | 内联 ArgumentError |
| `Stage` | 已 enum，OK | 不变 | 不变 |

### 构造路径

所有值对象**只能通过 `try_new` / `fromValue` 构造**，私有字段，禁止直接字面量构造。Rust:

```rust
impl Intensity {
    pub fn try_new(v: u8) -> AppResult<Self> {
        if !(1..=5).contains(&v) {
            return Err(AppError::invalid_input("intensity", "must be 1..=5"));
        }
        Ok(Self(v))
    }
    pub fn value(self) -> u8 { self.0 }
}
```

Dart：

```dart
extension type const FragmentId._(String value) {
  factory FragmentId(String raw) {
    if (raw.isEmpty) throw ArgumentError.value(raw, 'FragmentId', 'must be non-empty');
    return FragmentId._(raw);
  }
}
```

### 实体构造

`Fragment::try_new(...)` 返回 `AppResult<Fragment>`；旧 `validate()` 删除。  
DTO `into_domain` 改为组合 `try_new` 调用 + Stage::from_code。  
**净效果**：领域内任何方法签名出现 `&Fragment`，调用方都不需要再做合法性检查 —— 类型已经保证。

### Dart 侧妥协

Dart 缺乏 `Result` 语义；本 ADR 选择：

- 公共构造器在非法输入时**抛 `ArgumentError`**（fail-fast，bug 必现）
- 同时提供 `tryFrom...` 返回 `T?` 用于解析用户输入路径
- `Fragment` / `Recovery` 的构造器加 `assert` + 运行时校验（release 模式仍校验）

### `==` / `hashCode`

Dart 实体加上 `==` / `hashCode`（`Object.hash`）；Rust 实体加 `#[derive(PartialEq, Eq)]`。  
这是后续单元测试 / 状态比较 / Provider 去抖的基础。

## 不在本 ADR 范围

- 加密（ADR-0002）：值对象 plain 持有明文，不感知加密。
- 持久化正规化（ADR-0004）：mapper 内调用值对象的 `try_new`，把数据库脏数据拒之门外。
- 事件流（ADR-0005）：事件载荷复用值对象。

## 退出 / 演进

- 未来如需 freezed/equatable，可在不破坏值对象 API 前提下叠加。
- `Intensity` 未来如需扩展到 0..=10，仅改一处约束 + 加一条迁移。
- 值对象之上可以再加 *refinement*（例如 `Intensity::High = Intensity(4|5)`），按需引入。

## 验收

- Rust: `cargo clippy -D warnings` / `cargo test` 全绿。
- Dart: `flutter analyze` / `flutter test` 全绿。
- 不再有 `Fragment { id: "".into(), ... }` 这种字面量构造可以编译通过的代码路径。
