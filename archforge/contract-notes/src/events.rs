//! Notes 聚合发出的领域事件。
//!
//! 每个 variant 都有稳定带版本的鉴别符 (`v` tag), 这是 wire 契约的一部分。
//! 重命名或破坏式变更要走"新增 variant", 而不是改动已有 variant —— 那是
//! 让老消费者继续可用的唯一方式。

use crate::types::{Body, NoteId, Tag, Title};
use archforge_kernel::{DomainEvent, Timestamp};
use serde::{Deserialize, Serialize};

/// Notes 聚合发出的所有领域事件。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
#[serde(tag = "v")]
pub enum NoteEvent {
    /// `notes.note.created.v1`.
    #[serde(rename = "notes.note.created.v1")]
    Created(NoteCreated),
    /// `notes.note.edited.v1`.
    #[serde(rename = "notes.note.edited.v1")]
    Edited(NoteEdited),
    /// `notes.note.archived.v1`.
    #[serde(rename = "notes.note.archived.v1")]
    Archived(NoteArchived),
    /// `notes.note.restored.v1`.
    #[serde(rename = "notes.note.restored.v1")]
    Restored(NoteRestored),
}

/// `notes.note.created.v1` 的载荷。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NoteCreated {
    /// 新笔记 id。
    pub id: NoteId,
    /// 创建时的标题。
    pub title: Title,
    /// 创建时的正文。
    pub body: Body,
    /// 创建时的标签 (已去重、按字典序排好)。
    pub tags: Vec<Tag>,
    /// 何时发生。
    pub at: Timestamp,
}

/// `notes.note.edited.v1` 的载荷。
///
/// 只携带"真正发生变化"的字段; 没改的字段保持 `None`。这样审计日志能直观地
/// 看到一次编辑到底动了什么。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NoteEdited {
    /// 被编辑的笔记 id。
    pub id: NoteId,
    /// 编辑后的标题; `None` 表示标题未变。
    pub title: Option<Title>,
    /// 编辑后的正文; `None` 表示正文未变。
    pub body: Option<Body>,
    /// 编辑后的标签; `None` 表示标签未变。
    pub tags: Option<Vec<Tag>>,
    /// 何时发生。
    pub at: Timestamp,
}

/// `notes.note.archived.v1` 的载荷。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NoteArchived {
    /// 被归档的笔记 id。
    pub id: NoteId,
    /// 何时发生。
    pub at: Timestamp,
}

/// `notes.note.restored.v1` 的载荷。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NoteRestored {
    /// 被恢复的笔记 id。
    pub id: NoteId,
    /// 何时发生。
    pub at: Timestamp,
}

impl DomainEvent for NoteEvent {
    fn event_type(&self) -> &'static str {
        match self {
            Self::Created(_) => "notes.note.created.v1",
            Self::Edited(_) => "notes.note.edited.v1",
            Self::Archived(_) => "notes.note.archived.v1",
            Self::Restored(_) => "notes.note.restored.v1",
        }
    }

    fn aggregate_id(&self) -> String {
        match self {
            Self::Created(e) => e.id.to_string(),
            Self::Edited(e) => e.id.to_string(),
            Self::Archived(e) => e.id.to_string(),
            Self::Restored(e) => e.id.to_string(),
        }
    }

    fn occurred_at(&self) -> Timestamp {
        match self {
            Self::Created(e) => e.at,
            Self::Edited(e) => e.at,
            Self::Archived(e) => e.at,
            Self::Restored(e) => e.at,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn created_event_round_trips() {
        let e = NoteEvent::Created(NoteCreated {
            id: NoteId::new(),
            title: Title::new("hi").unwrap(),
            body: Body::new("there").unwrap(),
            tags: vec![Tag::new("rust").unwrap()],
            at: Timestamp::from_ms(1),
        });
        let s = serde_json::to_string(&e).unwrap();
        assert!(s.contains("notes.note.created.v1"));
        let back: NoteEvent = serde_json::from_str(&s).unwrap();
        assert_eq!(e, back);
    }

    #[test]
    fn edited_only_captures_changed_fields() {
        let e = NoteEvent::Edited(NoteEdited {
            id: NoteId::new(),
            title: Some(Title::new("new").unwrap()),
            body: None,
            tags: None,
            at: Timestamp::from_ms(5),
        });
        assert_eq!(e.event_type(), "notes.note.edited.v1");
        let s = serde_json::to_string(&e).unwrap();
        let back: NoteEvent = serde_json::from_str(&s).unwrap();
        assert_eq!(e, back);
    }

    #[test]
    fn archived_and_restored_round_trip() {
        let id = NoteId::new();
        let a = NoteEvent::Archived(NoteArchived {
            id,
            at: Timestamp::from_ms(10),
        });
        let r = NoteEvent::Restored(NoteRestored {
            id,
            at: Timestamp::from_ms(20),
        });
        assert_eq!(a.event_type(), "notes.note.archived.v1");
        assert_eq!(r.event_type(), "notes.note.restored.v1");
        let _: NoteEvent = serde_json::from_str(&serde_json::to_string(&a).unwrap()).unwrap();
        let _: NoteEvent = serde_json::from_str(&serde_json::to_string(&r).unwrap()).unwrap();
    }
}
