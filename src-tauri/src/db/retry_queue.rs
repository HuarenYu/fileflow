use anyhow::Result;
use rusqlite::{params, Connection};

pub struct RetryQueue {
    conn: Connection,
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
        Ok(Self { conn })
    }

    pub fn push(&self, path: &str, error: &str) -> Result<()> {
        self.conn.execute(
            "INSERT INTO retry_queue (path, error, created_at) VALUES (?1, ?2, ?3)",
            params![path, error, chrono::Utc::now().timestamp()],
        )?;
        Ok(())
    }

    pub fn drain(&self) -> Result<Vec<String>> {
        let mut stmt = self
            .conn
            .prepare("SELECT path FROM retry_queue ORDER BY created_at")?;
        let paths: Vec<String> = stmt
            .query_map([], |row| row.get(0))?
            .filter_map(|r| r.ok())
            .collect();
        self.conn.execute("DELETE FROM retry_queue", [])?;
        Ok(paths)
    }
}
