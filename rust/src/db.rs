use rusqlite::{Connection, params};
use parking_lot::Mutex;
use std::sync::Arc;

/// SQLite database wrapper (mirrors db/index.ts + db/schema.ts)
pub struct Database {
    conn: Arc<Mutex<Connection>>,
}

impl Database {
    pub fn new() -> Self {
        // Ensure data directory exists
        std::fs::create_dir_all("data").ok();

        let conn = Connection::open("data/caro.db")
            .expect("Failed to open SQLite database");

        conn.execute_batch("PRAGMA journal_mode = WAL;").unwrap();
        conn.execute_batch("PRAGMA synchronous = NORMAL;").unwrap();

        Database {
            conn: Arc::new(Mutex::new(conn)),
        }
    }

    /// Create table if not exists (mirrors initDB)
    pub fn init(&self) {
        let conn = self.conn.lock();
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS histories (
                pair_key TEXT PRIMARY KEY,
                player_a TEXT NOT NULL,
                player_b TEXT NOT NULL,
                wins_a INTEGER DEFAULT 0 NOT NULL,
                wins_b INTEGER DEFAULT 0 NOT NULL,
                draws INTEGER DEFAULT 0 NOT NULL,
                updated_at INTEGER
            );"
        ).expect("Failed to create histories table");
        conn.execute_batch("PRAGMA optimize;").unwrap();
    }

    /// Upsert history record (mirrors incHistory in store.ts)
    pub fn inc_history(&self, winner_id: Option<&str>, id1: &str, id2: &str) {
        let mut ids = [id1, id2];
        ids.sort();
        let (a, b) = (ids[0], ids[1]);

        if a == b {
            return; // không ghi lịch sử tự đấu với chính mình
        }

        let pair_key = format!("{}|{}", a, b);

        let conn = self.conn.lock();

        // Try to insert first
        let result = conn.execute(
            "INSERT OR IGNORE INTO histories (pair_key, player_a, player_b, wins_a, wins_b, draws, updated_at) 
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, strftime('%s','now'))",
            params![
                pair_key,
                a,
                b,
                if winner_id == Some(a) { 1 } else { 0 },
                if winner_id == Some(b) { 1 } else { 0 },
                if winner_id.is_none() { 1 } else { 0 },
            ],
        );

        match result {
            Ok(0) => {
                // Row already exists, update
                let update_sql = if winner_id.is_none() {
                    "UPDATE histories SET draws = draws + 1, updated_at = strftime('%s','now') WHERE pair_key = ?1"
                } else if winner_id == Some(a) {
                    "UPDATE histories SET wins_a = wins_a + 1, updated_at = strftime('%s','now') WHERE pair_key = ?1"
                } else {
                    "UPDATE histories SET wins_b = wins_b + 1, updated_at = strftime('%s','now') WHERE pair_key = ?1"
                };
                conn.execute(update_sql, params![pair_key])
                    .unwrap_or_else(|e| { tracing::error!("Save DB failed: {}", e); 0 });
            }
            Ok(_) => {} // Inserted successfully
            Err(e) => tracing::error!("Save DB failed: {}", e),
        }
    }

    /// Get perspective history (mirrors perspectiveHistory in store.ts)
    pub fn perspective_history(&self, me: &str, other: &str) -> serde_json::Value {
        let mut ids = [me, other];
        ids.sort();
        let pair_key = format!("{}|{}", ids[0], ids[1]);

        let conn = self.conn.lock();

        let result = conn.query_row(
            "SELECT player_a, player_b, wins_a, wins_b, draws FROM histories WHERE pair_key = ?1",
            params![pair_key],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, i64>(3)?,
                    row.get::<_, i64>(4)?,
                ))
            },
        );

        let (player_a, _player_b, wins_a, wins_b, draws) = result.unwrap_or_else(|_| {
            let (a, b) = if me < other {
                (me.to_string(), other.to_string())
            } else {
                (other.to_string(), me.to_string())
            };
            (a, b, 0, 0, 0)
        });

        let me_is_a = player_a == me;
        let wins = if me_is_a { wins_a } else { wins_b };
        let losses = if me_is_a { wins_b } else { wins_a };

        serde_json::json!({
            "me": me,
            "opponent": other,
            "wins": wins,
            "losses": losses,
            "draws": draws,
            "total": wins_a + wins_b + draws,
        })
    }
}

impl Clone for Database {
    fn clone(&self) -> Self {
        Database {
            conn: self.conn.clone(),
        }
    }
}
