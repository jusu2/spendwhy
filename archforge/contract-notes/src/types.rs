//! 值对象 + [`NoteDto`] 传输形状。

use archforge_kernel::{arch_newtype, Timestamp};
use serde::{Deserialize, Serialize};

arch_newtype! {
    /// 笔记标识符。随机 v4 uuid, 笔记上下文外部不透明。
    pub struct NoteId(Uuid);
}

arch_newtype! {
    /// 笔记标题。去首尾空白后 1..=200 个 Unicode code point。空标题不接受 ——
    /// 它会让"按标题搜索"的可发现性变得极差。
    pub struct Title(String) where |s| {
        let trimmed = s.trim();
        !trimmed.is_empty() && trimmed.chars().count() <= 200
    };
}

arch_newtype! {
    /// 笔记正文。允许为空 (用户可以只先记一个标题), 上限 64 KiB。
    /// 这个上限远超人类正常输入, 但能挡住把 Note 当 blob 容器滥用。
    pub struct Body(String) where |s| s.len() <= 64 * 1024;
}

arch_newtype! {
    /// 标签 (tag)。去首尾空白后 1..=40 个 Unicode code point, 不能含空白 ——
    /// 多 token 用多个 tag 表示, 不要用空格拼。
    pub struct Tag(String) where |s| {
        let trimmed = s.trim();
        !trimmed.is_empty()
            && trimmed.chars().count() <= 40
            && !trimmed.chars().any(char::is_whitespace)
    };
}

/// 笔记状态。
///
/// 状态转换规则 (领域层会强制, 这里只做形状定义):
///
/// - `Active` ↔ `Archived`: 双向可逆。
/// - 没有"软删除"; 想要永久删除的需求, 走另一个 use case (后续 phase)。
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NoteStatus {
    /// 可见、可被列出。
    Active,
    /// 已归档; 默认不出现在主列表里, 但仍可被检索/恢复。
    Archived,
}

impl core::fmt::Display for NoteStatus {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Active => f.write_str("active"),
            Self::Archived => f.write_str("archived"),
        }
    }
}

/// 聚合版本号, 用于乐观并发控制 (CAS)。
///
/// 每次成功写都会单调 +1。`update` 操作必须把读到的版本号回传给 Port,
/// 让适配器用 [`AppError::Conflict`] 拒绝过期写。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Version(u64);

impl Version {
    /// 新创建聚合的初始版本号。
    pub const INITIAL: Self = Self(1);

    /// 从原始 `u64` 构造 (适配器 re-hydrate 时用)。
    pub const fn from_u64(v: u64) -> Self {
        Self(v)
    }

    /// 取出内部值。
    pub const fn as_u64(&self) -> u64 {
        self.0
    }

    /// +1, 在 `u64::MAX` 处饱和。饱和只是保险; 实际上每个聚合 2^64 个版本
    /// 是触不到的。
    pub fn next(self) -> Self {
        Self(self.0.saturating_add(1))
    }
}

impl Default for Version {
    fn default() -> Self {
        Self::INITIAL
    }
}

impl core::fmt::Display for Version {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "v{}", self.0)
    }
}

/// 笔记的笔记侧投影。**这是唯一允许跨 Port 边界的形状** ——
/// `domain-notes::Note` 留在自己的 crate 里。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NoteDto {
    /// 唯一标识。
    pub id: NoteId,
    /// 标题。
    pub title: Title,
    /// 正文。
    pub body: Body,
    /// 标签列表。集合语义 (无序、无重复); 这里用 `Vec<Tag>` 是为了 JSON
    /// 友好, 由领域层负责去重/排序。
    #[serde(default)]
    pub tags: Vec<Tag>,
    /// 状态。
    pub status: NoteStatus,
    /// 创建时间。
    pub created_at: Timestamp,
    /// 最后一次修改时间 (包括状态变更)。
    pub updated_at: Timestamp,
    /// 聚合版本号, 乐观并发控制用。
    #[serde(default)]
    pub version: Version,
    /// schema 版本。适配器**必须**为本布局发出 `1`。将来的破坏性变更引入
    /// 新 DTO 类型, 而不是给字段塞新含义。
    #[serde(default = "default_schema_v1")]
    pub schema_version: u16,
}

const fn default_schema_v1() -> u16 {
    1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn title_validator_rejects_blank_and_overlong() {
        assert!(Title::new("").is_err());
        assert!(Title::new("   ").is_err());
        assert!(Title::new("x").is_ok());
        let too_long: String = "x".repeat(201);
        assert!(Title::new(too_long).is_err());
        let ok_max: String = "x".repeat(200);
        assert!(Title::new(ok_max).is_ok());
    }

    #[test]
    fn body_accepts_empty_and_bounded() {
        assert!(Body::new("").is_ok());
        assert!(Body::new("hello").is_ok());
        let too_big = "x".repeat(64 * 1024 + 1);
        assert!(Body::new(too_big).is_err());
    }

    #[test]
    fn tag_rejects_whitespace_and_overlong() {
        assert!(Tag::new("").is_err());
        assert!(Tag::new("has space").is_err());
        assert!(Tag::new("rust").is_ok());
        let too_long: String = "x".repeat(41);
        assert!(Tag::new(too_long).is_err());
    }

    #[test]
    fn version_monotonic() {
        let a = Version::INITIAL;
        let b = a.next();
        assert!(b > a);
        assert_eq!(b.as_u64(), 2);
    }

    #[test]
    fn status_display_is_snake_case() {
        assert_eq!(NoteStatus::Active.to_string(), "active");
        assert_eq!(NoteStatus::Archived.to_string(), "archived");
        let json = serde_json::to_string(&NoteStatus::Archived).unwrap();
        assert_eq!(json, r#""archived""#);
    }

    #[test]
    fn note_dto_round_trips() {
        let dto = NoteDto {
            id: NoteId::new(),
            title: Title::new("hi").unwrap(),
            body: Body::new("there").unwrap(),
            tags: vec![Tag::new("rust").unwrap()],
            status: NoteStatus::Active,
            created_at: Timestamp::from_ms(100),
            updated_at: Timestamp::from_ms(200),
            version: Version::INITIAL,
            schema_version: 1,
        };
        let json = serde_json::to_string(&dto).unwrap();
        let back: NoteDto = serde_json::from_str(&json).unwrap();
        assert_eq!(dto, back);
    }

    #[test]
    fn note_dto_back_compat_defaults() {
        // 早于 version/tags 引入的 DTO 也得能解析。
        let json = r#"{
            "id": "00000000-0000-0000-0000-000000000001",
            "title": "hi",
            "body": "",
            "status": "active",
            "created_at": 0,
            "updated_at": 0,
            "schema_version": 1
        }"#;
        let dto: NoteDto = serde_json::from_str(json).unwrap();
        assert_eq!(dto.version, Version::INITIAL);
        assert!(dto.tags.is_empty());
    }

    #[test]
    fn note_dto_rejects_invalid_title_on_deserialize() {
        let bad = r#"{
            "id": "00000000-0000-0000-0000-000000000001",
            "title": "",
            "body": "",
            "status": "active",
            "created_at": 0,
            "updated_at": 0,
            "schema_version": 1
        }"#;
        assert!(serde_json::from_str::<NoteDto>(bad).is_err());
    }
}
