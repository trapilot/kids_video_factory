use rusqlite::{params, Connection, Result};
use chrono::Local;
use std::sync::Arc;
use tokio::sync::Mutex;
use crate::models::*;
use crate::helper::*;

#[derive(Debug, Clone)]
pub struct DbManager {
    conn: Arc<Mutex<Connection>>
}

pub const APP_VERSION: u8 = 1;

impl DbManager {
    pub fn new(db_path: &str) -> Self {
        let conn = Connection::open(db_path).expect("open db failed");
        conn.execute(
        "CREATE TABLE IF NOT EXISTS video_history (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                age INTEGER NOT NULL,
                topic TEXT NOT NULL,
                created_at TEXT NOT NULL
            )",
            [],
        ).unwrap();

        conn.execute(
        "CREATE TABLE IF NOT EXISTS workflow_state (
                session_id TEXT PRIMARY KEY,
                state_json TEXT NOT NULL,
                version INTEGER NOT NULL DEFAULT 1,
                status TEXT NOT NULL,
                retry_count INTEGER NOT NULL DEFAULT 0,
                max_retry INTEGER NOT NULL DEFAULT 3,
                backoff_ms INTEGER NOT NULL DEFAULT 1000,
                last_error TEXT,
                updated_at TEXT NOT NULL
            )",
            [],
        ).unwrap();

        // conn.execute("CREATE INDEX idx_scenes_version_status ON workflow_state(version, status)", []).unwrap();
        
        Self {
            conn: Arc::new(Mutex::new(conn)),
        }
    }

    async fn conn(&self) -> tokio::sync::MutexGuard<'_, Connection> {
        self.conn.lock().await
    }

    pub async fn save_topic(&self, age: u8, topic: &str) -> Result<()> {
        let now = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
        let conn = self.conn().await;

        conn.execute(
            "INSERT INTO video_history (age, topic, created_at) VALUES (?1, ?2, ?3)",
            params![age, topic, now],
        )?;
        Ok(())
    }

    pub async fn save_state(
        &self,
        state: &VideoState,
    ) -> Result<()> {
        let conn = self.conn().await;
        let state_json = serde_json::to_string(state).unwrap();
        
        conn.execute(
            "INSERT INTO workflow_state (
                session_id,
                state_json,
                version,
                status,
                retry_count,
                max_retry,
                backoff_ms,
                last_error,
                updated_at
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
            ON CONFLICT(session_id) DO UPDATE SET
                state_json=excluded.state_json,
                status=excluded.status,
                retry_count=excluded.retry_count,
                max_retry=excluded.max_retry,
                backoff_ms=excluded.backoff_ms,
                last_error=excluded.last_error,
                updated_at=excluded.updated_at",
            rusqlite::params![
                state.session_id,
                state_json,
                APP_VERSION,
                state.meta.status,
                state.meta.retry_count,
                state.meta.max_retry,
                state.meta.backoff_ms,
                state.meta.last_error,
                now_rfc()
            ],
        )
        .map_err(|e| e.to_string()).unwrap();

        Ok(())
    }

    pub async fn delete_state(
        &self,
        session_id: &str,
    ) -> Result<()> {
        let conn = self.conn().await;

        conn.execute(
            "DELETE FROM workflow_state WHERE session_id = ?1",
            params![session_id],
        )?;

        Ok(())
    }

    pub async fn get_recent_topics(&self, age: u8, limit: u8) -> Result<Vec<String>> {
        let conn = self.conn().await;
        
        let mut stmt = conn.prepare(
            "SELECT topic FROM video_history WHERE age = ?1 ORDER BY id DESC LIMIT ?2"
        )?;
        
        let topic_iter = stmt.query_map(
            params![age, limit], |row| row.get::<_, String>(0)
        )?;

        let mut topics = Vec::new();

        for topic in topic_iter {
            topics.push(topic?);
        }

        Ok(topics)
    }
    
    pub async fn get_recent_state(&self) -> Result<Option<VideoState>> {
        let conn = self.conn().await;

        let mut stmt = conn.prepare(
            "
            SELECT state_json
            FROM workflow_state
            WHERE version = ?1
            ORDER BY updated_at DESC
            LIMIT 1
            ",
        )?;

        let result = stmt.query_row(
            [APP_VERSION], |row| row.get::<_, String>(0)
        );

        match result {
            Ok(state_json) => {
                match serde_json::from_str::<VideoState>(&state_json) {
                    Ok(state) if !state.is_finished() => Ok(Some(state)),
                    Ok(_) => Ok(None),
                    Err(_) => Ok(None),
                }
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }
}