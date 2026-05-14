//! `Note` 聚合根。

use archforge_contract_notes::{
    Body, NoteArchived, NoteCreated, NoteDto, NoteEdited, NoteEvent, NoteId, NoteRestored,
    NoteStatus, Tag, Title, Version,
};
use archforge_kernel::{AppError, Result, Timestamp};

/// Notes 聚合根。
///
/// 不变量:
/// - `title`、`body`、各 `tag` 由 newtype 构造时已经是字段级合法的。
/// - `tags` 在内部存储为**已去重 + 按字典序排序**的 `Vec<Tag>`, 这样
///   "同一组标签的两次 Edit"得到的 DTO 一定逐字节相等 (利于 idempotency
///   与缓存命中)。
/// - `updated_at >= created_at` 任何 `to_dto()` 返回值都满足。
/// - `version` 在每次成功变更后单调递增。
/// - 只能通过返回 [`NoteEvent`] 的方法变更状态。
///
/// `Clone` 是允许的, 这样 use case 可以在变更前后留快照; 故意**不**实现
/// `Serialize` —— 对外要序列化时走 `to_dto()`。
#[derive(Debug, Clone)]
pub struct Note {
    id: NoteId,
    title: Title,
    body: Body,
    tags: Vec<Tag>,
    status: NoteStatus,
    created_at: Timestamp,
    updated_at: Timestamp,
    version: Version,
}

impl Note {
    /// 新建笔记。返回聚合和需要由调用方发布的 `NoteCreated` 事件
    /// (一般通过 outbox)。
    pub fn create(title: Title, body: Body, tags: Vec<Tag>, now: Timestamp) -> (Self, NoteEvent) {
        let id = NoteId::new();
        let tags = normalise_tags(tags);
        let note = Self {
            id,
            title: title.clone(),
            body: body.clone(),
            tags: tags.clone(),
            status: NoteStatus::Active,
            created_at: now,
            updated_at: now,
            version: Version::INITIAL,
        };
        let event = NoteEvent::Created(NoteCreated {
            id,
            title,
            body,
            tags,
            at: now,
        });
        (note, event)
    }

    /// 从持久层 rehydrate。DTO 各字段已由 newtype 校验, 这里只补一组
    /// 跨字段不变量校验 (时间戳顺序、版本号下限、tags 形状)。
    pub fn rehydrate(dto: NoteDto) -> Result<Self> {
        if dto.updated_at < dto.created_at {
            return Err(AppError::Invalid(format!(
                "updated_at({}) < created_at({}) for note {}",
                dto.updated_at.as_ms(),
                dto.created_at.as_ms(),
                dto.id
            )));
        }
        if dto.version.as_u64() == 0 {
            return Err(AppError::Invalid(format!(
                "version must be >= 1 for note {}",
                dto.id
            )));
        }
        // 适配器在落盘时可能没有保证 tags 已规范化; rehydrate 把它收紧, 这样
        // 之后所有的 to_dto/相等性比较都是确定的。
        let tags = normalise_tags(dto.tags);
        Ok(Self {
            id: dto.id,
            title: dto.title,
            body: dto.body,
            tags,
            status: dto.status,
            created_at: dto.created_at,
            updated_at: dto.updated_at,
            version: dto.version,
        })
    }

    /// 投影成传输用的 DTO。
    pub fn to_dto(&self) -> NoteDto {
        NoteDto {
            id: self.id,
            title: self.title.clone(),
            body: self.body.clone(),
            tags: self.tags.clone(),
            status: self.status,
            created_at: self.created_at,
            updated_at: self.updated_at,
            version: self.version,
            schema_version: 1,
        }
    }

    /// 标识符。
    pub fn id(&self) -> NoteId {
        self.id
    }

    /// 当前标题。
    pub fn title(&self) -> &Title {
        &self.title
    }

    /// 当前正文。
    pub fn body(&self) -> &Body {
        &self.body
    }

    /// 当前标签 (已规范化)。
    pub fn tags(&self) -> &[Tag] {
        &self.tags
    }

    /// 当前状态。
    pub fn status(&self) -> NoteStatus {
        self.status
    }

    /// 当前版本号。
    pub fn version(&self) -> Version {
        self.version
    }

    /// 领域操作: 编辑。
    ///
    /// 三个 `Option` 参数: `None` 表示"不动", `Some(v)` 表示"改成 v"。
    ///
    /// - 整个 patch 都是 `None` (无改动) → `AppError::Invalid` 拒绝, 避免
    ///   产生空 Edit 事件污染审计流。
    /// - 所有 `Some(_)` 的值都跟当前值相等 (no-op 编辑) → 同样 `Invalid`。
    /// - `now < updated_at` (时钟回退) → `Invalid`。
    pub fn edit(
        &mut self,
        new_title: Option<Title>,
        new_body: Option<Body>,
        new_tags: Option<Vec<Tag>>,
        now: Timestamp,
    ) -> Result<NoteEvent> {
        if new_title.is_none() && new_body.is_none() && new_tags.is_none() {
            return Err(AppError::Invalid("edit: no fields supplied".into()));
        }
        if now < self.updated_at {
            return Err(AppError::Invalid("clock skew: now < updated_at".into()));
        }

        let normalised_tags = new_tags.map(normalise_tags);

        let title_changed = matches!(&new_title, Some(t) if t != &self.title);
        let body_changed = matches!(&new_body, Some(b) if b != &self.body);
        let tags_changed = matches!(&normalised_tags, Some(t) if t != &self.tags);
        if !title_changed && !body_changed && !tags_changed {
            return Err(AppError::Invalid(
                "edit: no-op (all fields unchanged)".into(),
            ));
        }

        let mut event_title = None;
        let mut event_body = None;
        let mut event_tags = None;

        if let Some(t) = new_title {
            if t != self.title {
                self.title = t.clone();
                event_title = Some(t);
            }
        }
        if let Some(b) = new_body {
            if b != self.body {
                self.body = b.clone();
                event_body = Some(b);
            }
        }
        if let Some(t) = normalised_tags {
            if t != self.tags {
                self.tags = t.clone();
                event_tags = Some(t);
            }
        }

        self.updated_at = now;
        self.version = self.version.next();

        Ok(NoteEvent::Edited(NoteEdited {
            id: self.id,
            title: event_title,
            body: event_body,
            tags: event_tags,
            at: now,
        }))
    }

    /// 领域操作: 归档。
    pub fn archive(&mut self, now: Timestamp) -> Result<NoteEvent> {
        if self.status == NoteStatus::Archived {
            return Err(AppError::Conflict("note already archived".into()));
        }
        if now < self.updated_at {
            return Err(AppError::Invalid("clock skew: now < updated_at".into()));
        }
        self.status = NoteStatus::Archived;
        self.updated_at = now;
        self.version = self.version.next();
        Ok(NoteEvent::Archived(NoteArchived {
            id: self.id,
            at: now,
        }))
    }

    /// 领域操作: 反归档 / 恢复。
    pub fn restore(&mut self, now: Timestamp) -> Result<NoteEvent> {
        if self.status == NoteStatus::Active {
            return Err(AppError::Conflict("note not archived".into()));
        }
        if now < self.updated_at {
            return Err(AppError::Invalid("clock skew: now < updated_at".into()));
        }
        self.status = NoteStatus::Active;
        self.updated_at = now;
        self.version = self.version.next();
        Ok(NoteEvent::Restored(NoteRestored {
            id: self.id,
            at: now,
        }))
    }
}

/// 把标签集合规范化: 去重 + 按字典序排序。
///
/// 用 `Vec` 而不是 `HashSet`/`BTreeSet` 是为了让外部 JSON 序列化保持顺序
/// 稳定 (无序集合的 JSON 输出顺序在 Rust 标准库里没承诺)。
fn normalise_tags(mut tags: Vec<Tag>) -> Vec<Tag> {
    tags.sort_by(|a, b| a.as_str().cmp(b.as_str()));
    tags.dedup_by(|a, b| a.as_str() == b.as_str());
    tags
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t(s: &str) -> Title {
        Title::new(s).unwrap()
    }
    fn b(s: &str) -> Body {
        Body::new(s).unwrap()
    }
    fn tag(s: &str) -> Tag {
        Tag::new(s).unwrap()
    }

    #[test]
    fn create_emits_created_and_seeds_version() {
        let now = Timestamp::from_ms(100);
        let (n, evt) = Note::create(t("hi"), b("body"), vec![tag("b"), tag("a")], now);

        assert_eq!(n.version(), Version::INITIAL);
        assert_eq!(n.status(), NoteStatus::Active);
        // 标签已规范化
        let tags: Vec<&str> = n.tags().iter().map(Tag::as_str).collect();
        assert_eq!(tags, vec!["a", "b"]);

        match evt {
            NoteEvent::Created(c) => {
                assert_eq!(c.id, n.id());
                assert_eq!(c.at, now);
                let t: Vec<&str> = c.tags.iter().map(Tag::as_str).collect();
                assert_eq!(t, vec!["a", "b"]);
            }
            _ => panic!("expected Created"),
        }
    }

    #[test]
    fn create_dedupes_repeated_tags() {
        let now = Timestamp::from_ms(0);
        let (n, _) = Note::create(t("hi"), b(""), vec![tag("x"), tag("x"), tag("y")], now);
        let tags: Vec<&str> = n.tags().iter().map(Tag::as_str).collect();
        assert_eq!(tags, vec!["x", "y"]);
    }

    #[test]
    fn rehydrate_round_trips_and_normalises_tags() {
        let dto = NoteDto {
            id: NoteId::new(),
            title: t("hi"),
            body: b(""),
            tags: vec![tag("z"), tag("a"), tag("a")],
            status: NoteStatus::Active,
            created_at: Timestamp::from_ms(0),
            updated_at: Timestamp::from_ms(0),
            version: Version::INITIAL,
            schema_version: 1,
        };
        let n = Note::rehydrate(dto).unwrap();
        let tags: Vec<&str> = n.tags().iter().map(Tag::as_str).collect();
        assert_eq!(tags, vec!["a", "z"]);
    }

    #[test]
    fn rehydrate_rejects_inverted_timestamps() {
        let dto = NoteDto {
            id: NoteId::new(),
            title: t("hi"),
            body: b(""),
            tags: vec![],
            status: NoteStatus::Active,
            created_at: Timestamp::from_ms(200),
            updated_at: Timestamp::from_ms(100),
            version: Version::INITIAL,
            schema_version: 1,
        };
        assert!(matches!(Note::rehydrate(dto), Err(AppError::Invalid(_))));
    }

    #[test]
    fn rehydrate_rejects_zero_version() {
        let dto = NoteDto {
            id: NoteId::new(),
            title: t("hi"),
            body: b(""),
            tags: vec![],
            status: NoteStatus::Active,
            created_at: Timestamp::from_ms(0),
            updated_at: Timestamp::from_ms(0),
            version: Version::from_u64(0),
            schema_version: 1,
        };
        assert!(matches!(Note::rehydrate(dto), Err(AppError::Invalid(_))));
    }

    #[test]
    fn edit_with_no_fields_is_invalid() {
        let (mut n, _) = Note::create(t("hi"), b(""), vec![], Timestamp::from_ms(0));
        let r = n.edit(None, None, None, Timestamp::from_ms(1));
        assert!(matches!(r, Err(AppError::Invalid(_))));
        assert_eq!(n.version(), Version::INITIAL);
    }

    #[test]
    fn edit_with_only_unchanged_fields_is_invalid() {
        let (mut n, _) = Note::create(t("hi"), b("body"), vec![], Timestamp::from_ms(0));
        let r = n.edit(Some(t("hi")), Some(b("body")), None, Timestamp::from_ms(1));
        assert!(matches!(r, Err(AppError::Invalid(_))));
        assert_eq!(n.version(), Version::INITIAL);
    }

    #[test]
    fn edit_bumps_version_only_for_changed_fields() {
        let (mut n, _) = Note::create(t("hi"), b("body"), vec![tag("a")], Timestamp::from_ms(0));
        let v0 = n.version();
        let evt = n
            .edit(Some(t("bye")), None, None, Timestamp::from_ms(10))
            .unwrap();
        assert_eq!(n.title().as_str(), "bye");
        assert!(n.version() > v0);
        match evt {
            NoteEvent::Edited(e) => {
                assert_eq!(e.title.as_ref().unwrap().as_str(), "bye");
                assert!(e.body.is_none());
                assert!(e.tags.is_none());
            }
            _ => panic!("expected Edited"),
        }
    }

    #[test]
    fn edit_rejects_clock_skew() {
        let (mut n, _) = Note::create(t("hi"), b(""), vec![], Timestamp::from_ms(100));
        let r = n.edit(Some(t("x")), None, None, Timestamp::from_ms(50));
        assert!(matches!(r, Err(AppError::Invalid(_))));
    }

    #[test]
    fn archive_then_restore_round_trip() {
        let (mut n, _) = Note::create(t("hi"), b(""), vec![], Timestamp::from_ms(0));
        let v0 = n.version();
        let _ = n.archive(Timestamp::from_ms(1)).unwrap();
        assert_eq!(n.status(), NoteStatus::Archived);
        let v1 = n.version();
        assert!(v1 > v0);
        let _ = n.restore(Timestamp::from_ms(2)).unwrap();
        assert_eq!(n.status(), NoteStatus::Active);
        assert!(n.version() > v1);
    }

    #[test]
    fn double_archive_is_conflict() {
        let (mut n, _) = Note::create(t("hi"), b(""), vec![], Timestamp::from_ms(0));
        let _ = n.archive(Timestamp::from_ms(1)).unwrap();
        assert!(matches!(
            n.archive(Timestamp::from_ms(2)),
            Err(AppError::Conflict(_))
        ));
    }

    #[test]
    fn restore_active_is_conflict() {
        let (mut n, _) = Note::create(t("hi"), b(""), vec![], Timestamp::from_ms(0));
        assert!(matches!(
            n.restore(Timestamp::from_ms(1)),
            Err(AppError::Conflict(_))
        ));
    }
}
