use rusqlite::Connection;

use crate::error::Result;

pub const INITIAL_SCHEMA: &str = include_str!("../migrations/0001_init.sql");

pub fn run_migrations(connection: &Connection) -> Result<()> {
    connection.execute_batch(INITIAL_SCHEMA)?;
    Ok(())
}
