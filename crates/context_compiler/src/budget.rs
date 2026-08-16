use crate::intent::Intent;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BudgetCategory {
    Task,
    Specification,
    Code,
    Structure,
    Memory,
    History,
    Tests,
    Documentation,
    Git,
    Conversation,
}

impl BudgetCategory {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Task => "task",
            Self::Specification => "specification",
            Self::Code => "code",
            Self::Structure => "structure",
            Self::Memory => "memory",
            Self::History => "history",
            Self::Tests => "tests",
            Self::Documentation => "documentation",
            Self::Git => "git",
            Self::Conversation => "conversation",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskType {
    Debugging,
    Architecture,
    Implementation,
    Review,
    Documentation,
    General,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BudgetAllocation {
    pub total: usize,
    pub by_category: BTreeMap<BudgetCategory, usize>,
    pub used: usize,
    pub used_by_category: BTreeMap<BudgetCategory, usize>,
}

impl BudgetAllocation {
    pub fn remaining(&self, category: BudgetCategory) -> usize {
        let cap = self.by_category.get(&category).copied().unwrap_or(0);
        let used = self.used_by_category.get(&category).copied().unwrap_or(0);
        cap.saturating_sub(used)
    }
}

/// Allocate a token budget by task type. Percentages sum to 100.
pub fn allocate_budget(task_type: TaskType, total: usize) -> BudgetAllocation {
    let weights: [(BudgetCategory, u32); 10] = match task_type {
        TaskType::Debugging => [
            (BudgetCategory::Tests, 22),
            (BudgetCategory::Code, 22),
            (BudgetCategory::History, 16),
            (BudgetCategory::Memory, 8),
            (BudgetCategory::Git, 8),
            (BudgetCategory::Task, 8),
            (BudgetCategory::Specification, 6),
            (BudgetCategory::Structure, 4),
            (BudgetCategory::Documentation, 4),
            (BudgetCategory::Conversation, 2),
        ],
        TaskType::Architecture => [
            (BudgetCategory::Specification, 20),
            (BudgetCategory::Structure, 16),
            (BudgetCategory::History, 14),
            (BudgetCategory::Memory, 12),
            (BudgetCategory::Code, 10),
            (BudgetCategory::Git, 8),
            (BudgetCategory::Task, 8),
            (BudgetCategory::Documentation, 6),
            (BudgetCategory::Tests, 4),
            (BudgetCategory::Conversation, 2),
        ],
        TaskType::Review => [
            (BudgetCategory::Code, 24),
            (BudgetCategory::Git, 16),
            (BudgetCategory::Tests, 14),
            (BudgetCategory::Specification, 12),
            (BudgetCategory::History, 10),
            (BudgetCategory::Task, 8),
            (BudgetCategory::Structure, 6),
            (BudgetCategory::Memory, 4),
            (BudgetCategory::Documentation, 4),
            (BudgetCategory::Conversation, 2),
        ],
        TaskType::Documentation => [
            (BudgetCategory::Documentation, 28),
            (BudgetCategory::Specification, 18),
            (BudgetCategory::Code, 14),
            (BudgetCategory::Structure, 10),
            (BudgetCategory::Memory, 8),
            (BudgetCategory::Task, 6),
            (BudgetCategory::History, 6),
            (BudgetCategory::Git, 4),
            (BudgetCategory::Tests, 4),
            (BudgetCategory::Conversation, 2),
        ],
        TaskType::Implementation | TaskType::General => [
            (BudgetCategory::Code, 24),
            (BudgetCategory::Task, 14),
            (BudgetCategory::Tests, 12),
            (BudgetCategory::Specification, 12),
            (BudgetCategory::Structure, 10),
            (BudgetCategory::Memory, 8),
            (BudgetCategory::History, 8),
            (BudgetCategory::Git, 6),
            (BudgetCategory::Documentation, 4),
            (BudgetCategory::Conversation, 2),
        ],
    };

    let mut by_category = BTreeMap::new();
    let mut assigned = 0usize;
    let last = weights.len() - 1;
    for (i, (cat, pct)) in weights.iter().enumerate() {
        let tokens = if i == last {
            total.saturating_sub(assigned)
        } else {
            total * (*pct as usize) / 100
        };
        assigned += tokens;
        by_category.insert(*cat, tokens);
    }

    BudgetAllocation {
        total,
        by_category,
        used: 0,
        used_by_category: BTreeMap::new(),
    }
}

pub fn task_type_from_intent(intent: &Intent) -> TaskType {
    intent.task_type
}

impl From<Intent> for TaskType {
    fn from(intent: Intent) -> Self {
        intent.task_type
    }
}
