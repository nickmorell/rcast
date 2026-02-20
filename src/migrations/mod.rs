use rusqlite::{Connection, Result, Transaction};
use std::collections::HashSet;

pub mod versions;
use versions::Migration;

pub fn run_migrations(conn: &mut Connection) -> Result<()> {
    let tx = conn.transaction()?;

    tx.execute(
        "CREATE TABLE IF NOT EXISTS __migrations (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT UNIQUE NOT NULL,
            applied_at INTEGER NOT NULL DEFAULT (unixepoch())
        )",
        [],
    )?;

    let migrations: Vec<&dyn Migration> =
        vec![&versions::initial_migration_02082026::InitialMigration];

    let mut names = HashSet::new();
    for m in &migrations {
        if !names.insert(m.name()) {
            panic!("Duplicate migration name: {}", m.name());
        }
    }

    let applied = get_applied_names(&tx)?;

    for migration in migrations.iter().filter(|m| !applied.contains(m.name())) {
        migration.up(&tx).unwrap();

        tx.execute(
            "INSERT INTO __migrations (name) VALUES (?1)",
            [migration.name()],
        )?;

        eprintln!("[migration] Applied: {}", migration.name());
    }

    tx.commit()
}

pub fn rollback_to(conn: &mut Connection, target_name: &str) -> Result<()> {
    let tx = conn.transaction()?;

    let migrations: Vec<&dyn Migration> =
        vec![&versions::initial_migration_02082026::InitialMigration];

    let target_pos = migrations
        .iter()
        .position(|m| m.name() == target_name)
        .expect("Target migration not found");

    let applied = get_applied_names(&tx)?;

    for migration in migrations
        .iter()
        .rev()
        .skip(migrations.len() - 1 - target_pos)
    {
        if applied.contains(migration.name()) {
            migration.down(&tx).unwrap();

            tx.execute(
                "DELETE FROM __migrations WHERE name = ?1",
                [migration.name()],
            )?;

            eprintln!("[migration] Rolled back: {}", migration.name());
        }
    }

    tx.commit()
}

pub fn rollback_n(conn: &mut Connection, count: usize) -> Result<()> {
    let tx = conn.transaction()?;

    let migrations: Vec<&dyn Migration> =
        vec![&versions::initial_migration_02082026::InitialMigration];

    let limit: i64 = count as i64;

    let mut stmt = tx.prepare("SELECT name FROM __migrations ORDER BY id DESC LIMIT ?1")?;

    let recent_names: Vec<String> = stmt
        .query_map([limit], |row| row.get(0))?
        .collect::<Result<Vec<_>>>()?;

    drop(stmt);

    for name in &recent_names {
        let migration = migrations
            .iter()
            .find(|m| m.name() == name.as_str())
            .expect(&format!("Migration '{}' not found", name));

        migration.down(&tx).unwrap();

        tx.execute("DELETE FROM __migrations WHERE name = ?1", [&name])?;

        eprintln!("[migration] Rolled back: {}", migration.name());
    }

    tx.commit()
}

fn get_applied_names(tx: &Transaction) -> Result<HashSet<String>> {
    let mut stmt = tx.prepare("SELECT name FROM __migrations")?;
    let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;

    let mut names = HashSet::new();
    for name in rows {
        names.insert(name?);
    }
    Ok(names)
}
