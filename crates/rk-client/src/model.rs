use std::fmt;

use chrono::NaiveDate;
use chrono::NaiveDateTime;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Priority {
    Low,
    Medium,
    High,
}

impl Priority {
    pub fn as_str(&self) -> &'static str {
        match self {
            Priority::Low => "Low",
            Priority::Medium => "Medium",
            Priority::High => "High",
        }
    }

    pub fn indicator(&self) -> &'static str {
        match self {
            Priority::Low => "L",
            Priority::Medium => "M",
            Priority::High => "H",
        }
    }
}

impl fmt::Display for Priority {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl std::str::FromStr for Priority {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Low" => Ok(Priority::Low),
            "Medium" => Ok(Priority::Medium),
            "High" => Ok(Priority::High),
            _ => Err(format!("Invalid priority: {}", s)),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Column {
    Todo,
    InProgress,
    Done,
}

impl Column {
    pub fn as_str(&self) -> &'static str {
        match self {
            Column::Todo => "todo",
            Column::InProgress => "in_progress",
            Column::Done => "done",
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            Column::Todo => "Todo",
            Column::InProgress => "In Progress",
            Column::Done => "Done",
        }
    }

    pub fn all() -> [Column; 3] {
        [Column::Todo, Column::InProgress, Column::Done]
    }

    pub fn index(&self) -> usize {
        match self {
            Column::Todo => 0,
            Column::InProgress => 1,
            Column::Done => 2,
        }
    }

    pub fn from_index(i: usize) -> Option<Column> {
        match i {
            0 => Some(Column::Todo),
            1 => Some(Column::InProgress),
            2 => Some(Column::Done),
            _ => None,
        }
    }
}

impl fmt::Display for Column {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

impl std::str::FromStr for Column {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "todo" => Ok(Column::Todo),
            "in_progress" => Ok(Column::InProgress),
            "done" => Ok(Column::Done),
            _ => Err(format!("Invalid column: {}", s)),
        }
    }
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct Task {
    pub id: i64,
    pub uuid: String,
    pub title: String,
    pub description: String,
    pub priority: Priority,
    pub column: Column,
    pub due_date: Option<NaiveDate>,
    pub tags: Vec<String>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub deleted: bool,
    pub deleted_at: Option<NaiveDateTime>,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct Tag {
    pub id: i64,
    pub uuid: String,
    pub name: String,
    pub updated_at: NaiveDateTime,
    pub deleted: bool,
    pub deleted_at: Option<NaiveDateTime>,
}
