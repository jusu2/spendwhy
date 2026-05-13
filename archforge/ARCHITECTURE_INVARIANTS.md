# ArchForge Architecture Invariants

**这 5 条是整个体系的「物理常数」。** 任何 PR 违反任一条 = 自动拒。Kernel 1.0 之后这份文档进入冻结期 12 个月。

---

## 1. 依赖方向单向

依赖图必须是 DAG，且方向永远向「契约」收敛：

```text
kernel  ← (任何 crate 都可依赖；它是叶子)
  ↑
contract-*                 ← 只依赖 kernel
  ↑
domain-*    app-*          ← 只依赖 contract（不互相依赖）
  ↑           ↑
infra-*     ← 只依赖 contract（不依赖 domain/app/其他 infra）
  ↑
bridge-*   examples/*      ← 装配层；唯一允许「向下看见所有人」的层
```

**强制手段**：`deny.toml` + CI 卡控。`cargo tree` 中出现反向依赖 = 失败。

## 2. 跨层只能传 DTO

- **Domain Model**（rich，私有字段，Always-Valid）严禁出 `domain-*` crate，**严禁实现 `Serialize`**。
- **Port trait 签名**只能用 `contract-*` 暴露的 DTO / Cmd / Query / Event。
- Adapter 内部完成 `Row ↔ Domain` 映射；Application 内部完成 `Domain ↔ DTO` 映射。

违反这条 = 换 Domain 风格（lite/rich/typestate）时所有 Adapter 都要改。这是解耦失败的最常见根因。

## 3. Port 不依赖具体技术

- 任何 `pub` API 签名出现 `sqlx::*`、`reqwest::*`、`rusqlite::*`、`std::io::Error`、`serde_json::Value` = 失败。
- 底层错误必须在 Adapter 内部 `From` 收敛进 contract 的 narrow `Error` enum。
- CI 用 `cargo public-api` 比对，加 grep 规则。

## 4. 每条 Error 都是枚举 + `#[non_exhaustive]`

- Port 错误：narrow enum，业务语义维度（`NotFound | Conflict | Invalid | Unavailable | Forbidden | Internal`），最多 8 个 variant。
- 全部加 `#[non_exhaustive]`，强制 downstream 写 `_ =>` 分支。新增 variant 是非破坏性变更。
- **禁止 `String`-only 错误** 与 `anyhow::Error` 出现在 pub API 上。

## 5. 每个 Adapter 必通过同一组 Port 一致性测试

- `conformance/` crate 提供与具体 Adapter 无关的 property test。
- 任何 Adapter 必须把自己塞进对应的 conformance 函数通过测试，否则不能发版。
- 这是把 LSP（Liskov 替换）从「口头承诺」升级为「类型系统 + property test 双重承诺」的工程兑现。

---

## 反模式：以下 7 件事 PR 一概拒绝

1. 第一个用例就抽 trait（等到第三个出现再抽，Rule of Three）。
2. `Box<dyn Trait>` 作为「万能积木」（默认用泛型 + 关联类型）。
3. 在 kernel 里加便利方法（kernel 只有「定义」，不许有「行为」）。
4. 用 macro 隐藏 wiring（DI 必须显式、可 grep）。
5. 在一个 mega-crate 里用 feature flag 切层（feature unification 会跨项目串味儿）。
6. `HashMap<String, serde_json::Value>` 当 DTO（类型系统逃生舱，所有保证瞬间失效）。
7. 以「快」为由跳过 conformance 测试套件。

---

## 修改本文档的流程

- 必须走 ADR（`docs/ADR/`）。
- 必须 2 个 kernel OWNER review。
- 必须附「不变量被违反时会发生什么」的真实故障案例。
