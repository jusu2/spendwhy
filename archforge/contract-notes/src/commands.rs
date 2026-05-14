//! 从调用方 (Application 层或表现层) 流向 Port 的命令。
//!
//! 查询当前 phase 暂不开 —— 等需要展示侧 (列表、搜索、按 tag 聚合) 时再
//! 单独引入 `NoteQuery`。

use crate::types::{Body, NoteId, Tag, Title};

/// 新建笔记。
///
/// 服务端会分配 `NoteId`, 调用方不传; 时间戳由 use case 通过 `ctx.clock`
/// 注入, 避免领域层直接读墙钟。
#[derive(Debug, Clone)]
pub struct CreateNoteCmd {
    /// 标题。
    pub title: Title,
    /// 正文。允许传 `Body::new("")` 表示"还没正文"。
    pub body: Body,
    /// 标签。允许为空; 重复的标签会在领域层被去重。
    pub tags: Vec<Tag>,
}

/// 编辑已存在的笔记。
///
/// 每个字段都是 `Option`: 缺省表示"本次不动这一项"。所有传了的字段会一次性
/// 原子写入, 而不是逐个 Port 调用。这样调用方拿到的版本号能稳定地往前推
/// 一格 (而不是每个字段各自推一格)。
#[derive(Debug, Clone)]
pub struct EditNoteCmd {
    /// 要编辑的笔记 id。
    pub id: NoteId,
    /// 新标题; `None` 表示不改。
    pub title: Option<Title>,
    /// 新正文; `None` 表示不改。
    pub body: Option<Body>,
    /// 新标签集合; `None` 表示不动, `Some(vec![])` 表示**清空所有标签**。
    pub tags: Option<Vec<Tag>>,
}

/// 归档一条笔记。
///
/// 归档已经归档的笔记是冲突 (`AppError::Conflict`), 而不是 no-op ——
/// 这样调用方能区分"我以为我归档了, 实际还活着"和"已经归档过了"。
#[derive(Debug, Clone, Copy)]
pub struct ArchiveNoteCmd {
    /// 要归档的笔记 id。
    pub id: NoteId,
}

/// 恢复 (反归档) 一条笔记。
///
/// 同理: 恢复一条本来就 Active 的笔记是 `Conflict`, 不是 no-op。
#[derive(Debug, Clone, Copy)]
pub struct RestoreNoteCmd {
    /// 要恢复的笔记 id。
    pub id: NoteId,
}
