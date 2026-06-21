use rusqlite::Connection;

use crate::error::Result;

pub const INITIAL_SCHEMA: &str = include_str!("../migrations/0001_init.sql");

pub fn run_migrations(connection: &Connection) -> Result<()> {
    connection.execute_batch(
        "
        PRAGMA foreign_keys = ON;
        PRAGMA journal_mode = WAL;
        ",
    )?;

    if schema_version_exists(connection, 1)? {
        return Ok(());
    }

    connection.execute_batch(INITIAL_SCHEMA)?;
    Ok(())
}

fn schema_version_exists(connection: &Connection, version: i64) -> Result<bool> {
    let table_exists: i64 = connection.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'schema_migrations'",
        [],
        |row| row.get(0),
    )?;

    if table_exists == 0 {
        return Ok(false);
    }

    let version_count: i64 = connection.query_row(
        "SELECT COUNT(*) FROM schema_migrations WHERE version = ?1",
        [version],
        |row| row.get(0),
    )?;

    Ok(version_count > 0)
}
