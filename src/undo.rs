use std::collections::VecDeque;

use crate::model::{Column, Priority};

#[derive(Debug, Clone)]
pub enum UndoAction {
    MoveTask {
        task_id: i64,
        from_column: Column,
    },
    PriorityChange {
        task_id: i64,
        previous: Priority,
    },
    DeleteTask {
        title: String,
        description: String,
        priority: Priority,
        column: Column,
        due_date: Option<chrono::NaiveDate>,
    },
    EditTask {
        task_id: i64,
        prev_title: String,
        prev_description: String,
        prev_priority: Priority,
        prev_due_date: Option<chrono::NaiveDate>,
    },
}

pub struct UndoStack {
    stack: VecDeque<UndoAction>,
    max_size: usize,
}

impl UndoStack {
    pub fn new() -> Self {
        UndoStack {
            stack: VecDeque::new(),
            max_size: 20,
        }
    }

    pub fn push(&mut self, action: UndoAction) {
        if self.stack.len() >= self.max_size {
            self.stack.pop_front();
        }
        self.stack.push_back(action);
    }

    pub fn pop(&mut self) -> Option<UndoAction> {
        self.stack.pop_back()
    }
}
