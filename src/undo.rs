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
    DuplicateTask {
        new_id: i64,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Column;

    #[test]
    fn test_push_pop() {
        let mut stack = UndoStack::new();
        stack.push(UndoAction::MoveTask {
            task_id: 1,
            from_column: Column::Todo,
        });
        assert!(stack.pop().is_some());
        assert!(stack.pop().is_none());
    }

    #[test]
    fn test_max_capacity() {
        let mut stack = UndoStack::new(); // max 20
        for i in 0..25 {
            stack.push(UndoAction::MoveTask {
                task_id: i,
                from_column: Column::Todo,
            });
        }
        let mut count = 0;
        while stack.pop().is_some() {
            count += 1;
        }
        assert_eq!(count, 20);
    }

    #[test]
    fn test_lifo_order() {
        let mut stack = UndoStack::new();
        stack.push(UndoAction::MoveTask {
            task_id: 1,
            from_column: Column::Todo,
        });
        stack.push(UndoAction::MoveTask {
            task_id: 2,
            from_column: Column::Done,
        });
        if let Some(UndoAction::MoveTask { task_id, .. }) = stack.pop() {
            assert_eq!(task_id, 2);
        } else {
            panic!("Expected MoveTask");
        }
    }
}
