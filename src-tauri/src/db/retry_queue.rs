use anyhow::Result;
use rusqlite::{params, Connection};
use std::sync::Mutex;

pub struct RetryQueue {
    conn: Mutex<Connection>,
}

impl RetryQueue {
    pub fn new(db_path: &str) -> Result<Self> {
        let conn = Connection::open(db_path)?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS retry_queue (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                path TEXT NOT NULL,
                error TEXT NOT NULL,
                created_at INTEGER NOT NULL
            );",
        )?;
        Ok(Self { conn: Mutex::new(conn) })
    }

    pub fn push(&self, path: &str, error: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO retry_queue (path, error, created_at) VALUES (?1, ?2, ?3)",
            params![path, error, chrono::Utc::now().timestamp()],
        )?;
        Ok(())
    }

    pub fn drain(&self) -> Result<Vec<String>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT path FROM retry_queue ORDER BY created_at, id")?;
        let paths: Vec<String> = stmt
            .query_map([], |row| row.get(0))?
            .collect::<Result<_, _>>()?;
        conn.execute("DELETE FROM retry_queue", [])?;
        Ok(paths)
    }
}
