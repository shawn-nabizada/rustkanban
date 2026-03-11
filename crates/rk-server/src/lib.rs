pub mod auth;
pub mod config;
pub mod error;
pub mod purge;
pub mod routes;
pub mod session;

/// Timestamp format for PostgreSQL `to_char()` — ISO 8601 without timezone.
pub const TS_FMT: &str = "YYYY-MM-DD\"T\"HH24:MI:SS";

/// Valid task column values.
pub const VALID_COLUMNS: &[&str] = &["todo", "in_progress", "done"];

/// Valid task priority values.
pub const VALID_PRIORITIES: &[&str] = &["Low", "Medium", "High"];
