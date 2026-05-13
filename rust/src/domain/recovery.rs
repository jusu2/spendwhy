//! 领域类型：Recovery。

use crate::error::{AppError, AppResult};

#[derive(Debug, Clone)]
pub struct Recovery {
    pub id: String,
    pub created_at_ms: i64,
    pub intensity: u8,
    pub description: String,
    pub related_fragment_ids: Vec<String>,
}

impl Recovery {
    pub fn validate(&self) -> AppResult<()> {
        if self.id.is_empty() {
            return Err(AppError::invalid_input("id", "recovery id is empty"));
        }
        if !(1..=5).contains(&self.intensity) {
            return Err(AppError::invalid_input(
                "intensity",
                "intensity must be in 1..=5",
            ));
        }
        if self.description.trim().is_empty() {
            return Err(AppError::invalid_input(
                "description",
                "recovery description is empty",
            ));
        }
        Ok(())
    }
}
