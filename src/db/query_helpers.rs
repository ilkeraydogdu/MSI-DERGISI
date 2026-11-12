//! Generic query helpers for the MSI Dergisi DAL layer.
//! Reduces boilerplate in repo.rs by abstracting the common
//! open_conn → prepare → query_map → filter_map(Result::ok) → collect pattern.
//!
//! Usage:
//!   let items = query_all(&db, sql, params![], |r| Ok(MyStruct { ... }));
//!   let item  = query_one(&db, sql, params![], |r| Ok(MyStruct { ... }));
//!   let count = query_scalar::<i64>(&db, "SELECT COUNT(*) FROM t", params![]);

use rusqlite::{Params, Row};
use crate::db::Database;

/// Execute a SELECT that returns multiple rows, mapping each via `mapper`.
/// Returns an empty Vec on any error (connection, prepare, or row mapping).
pub fn query_all<T, F>(db: &Database, sql: &str, params: impl Params, mapper: F) -> Vec<T>
where
    F: FnMut(&Row<'_>) -> rusqlite::Result<T>,
{
    let conn = match db.open_conn() {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };
    let mut stmt = match conn.prepare(sql) {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };
    let result = match stmt.query_map(params, mapper) {
        Ok(rows) => rows.filter_map(|r| r.ok()).collect(),
        Err(_) => Vec::new(),
    };
    result
}

/// Execute a SELECT that returns a single optional row.
/// Returns None on any error or when no row matches.
pub fn query_one<T, F>(db: &Database, sql: &str, params: impl Params, mapper: F) -> Option<T>
where
    F: FnMut(&Row<'_>) -> rusqlite::Result<T>,
{
    let conn = match db.open_conn() {
        Ok(c) => c,
        Err(_) => return None,
    };
    let mut stmt = match conn.prepare(sql) {
        Ok(s) => s,
        Err(_) => return None,
    };
    match stmt.query_row(params, mapper) {
        Ok(val) => Some(val),
        Err(_) => None,
    }
}

/// Execute a scalar SELECT (COUNT, SUM, MAX, etc.) returning a default value on error.
pub fn query_scalar<T>(db: &Database, sql: &str, params: impl Params) -> T
where
    T: rusqlite::types::FromSql + Default,
{
    let conn = match db.open_conn() {
        Ok(c) => c,
        Err(_) => return T::default(),
    };
    conn.query_row(sql, params, |r| r.get(0)).unwrap_or_default()
}

/// Execute a non-SELECT statement (INSERT, UPDATE, DELETE).
/// Returns the number of affected rows, or 0 on error.
pub fn execute(db: &Database, sql: &str, params: impl Params) -> usize {
    let conn = match db.open_conn() {
        Ok(c) => c,
        Err(_) => return 0,
    };
    conn.execute(sql, params).unwrap_or(0)
}

/// Execute a batch of SQL statements (no parameters).
/// Returns Ok(()) on success, Err on failure.
pub fn execute_batch(db: &Database, sql: &str) -> Result<(), String> {
    let conn = db.open_conn().map_err(|e| format!("Pool error: {}", e))?;
    conn.execute_batch(sql).map_err(|e| format!("Batch error: {}", e))
}

/// Build a dynamic WHERE clause from optional filter conditions.
/// Returns ("WHERE col1 = ?1 AND col2 = ?2", vec![val1, val2]) or ("", vec![]) if no filters.
/// Each entry is (column_name, optional_value). Only non-None values are included.
pub fn build_where_clause(filters: &[(&str, Option<&str>)]) -> (String, Vec<String>) {
    let mut clauses = Vec::new();
    let mut values = Vec::new();

    for (col, val) in filters.iter() {
        if let Some(v) = val {
            values.push(v.to_string());
            clauses.push(format!("{} = ?{}", col, values.len()));
        }
    }

    if clauses.is_empty() {
        (String::new(), Vec::new())
    } else {
        (format!(" WHERE {}", clauses.join(" AND ")), values)
    }
}

/// Build a dynamic ORDER BY + LIMIT clause.
pub fn build_order_limit(order_by: Option<&str>, limit: Option<usize>, offset: Option<usize>) -> String {
    let mut clause = String::new();
    if let Some(order) = order_by {
        clause.push_str(&format!(" ORDER BY {}", order));
    }
    if let Some(lim) = limit {
        clause.push_str(&format!(" LIMIT {}", lim));
    }
    if let Some(off) = offset {
        clause.push_str(&format!(" OFFSET {}", off));
    }
    clause
}