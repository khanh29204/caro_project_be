use rusqlite::{Connection, params};
use parking_lot::Mutex;
use std::sync::Arc;

/// SQLite database wrapper (mirrors db/index.ts + db/schema.ts)
pub struct Database {
    conn: Arc<Mutex<Option<Connection>>>,
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
            conn: Arc::new(Mutex::new(Some(conn))),
        }
    }

    /// Create table if not exists (mirrors initDB)
    pub fn init(&self) {
        let conn_guard = self.conn.lock();
        let conn = conn_guard.as_ref().unwrap();
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS histories (
                pair_key TEXT PRIMARY KEY,
                player_a TEXT NOT NULL,
                player_b TEXT NOT NULL,
                wins_a INTEGER DEFAULT 0 NOT NULL,
                wins_b INTEGER DEFAULT 0 NOT NULL,
                draws INTEGER DEFAULT 0 NOT NULL,
                updated_at INTEGER
            );
            CREATE TABLE IF NOT EXISTS user_links (
                old_id TEXT PRIMARY KEY,
                new_id TEXT NOT NULL
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

        let conn_guard = self.conn.lock();
        let conn = match conn_guard.as_ref() {
            Some(c) => c,
            None => return,
        };

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

    fn get_all_aliases(conn: &Connection, id: &str) -> Vec<String> {
        let mut root_id = id.to_string();
        if let Ok(new_id) = conn.query_row("SELECT new_id FROM user_links WHERE old_id = ?1", params![id], |row| row.get::<_, String>(0)) {
            root_id = new_id;
        }
        
        let mut aliases = vec![root_id.clone()];
        if let Ok(mut stmt) = conn.prepare("SELECT old_id FROM user_links WHERE new_id = ?1") {
            if let Ok(iter) = stmt.query_map(params![root_id], |row| row.get::<_, String>(0)) {
                for old_id in iter.flatten() {
                    if old_id != root_id && !aliases.contains(&old_id) {
                        aliases.push(old_id);
                    }
                }
            }
        }
        aliases
    }

    /// Get perspective history (mirrors perspectiveHistory in store.ts)
    pub fn perspective_history(&self, me: &str, other: &str) -> serde_json::Value {
        let conn_guard = self.conn.lock();
        let conn = match conn_guard.as_ref() {
            Some(c) => c,
            None => return serde_json::json!({ "error": "DB closed" }),
        };

        let my_aliases = Self::get_all_aliases(conn, me);
        let opp_aliases = Self::get_all_aliases(conn, other);

        let mut total_wins = 0;
        let mut total_losses = 0;
        let mut total_draws = 0;

        let in_my = my_aliases.iter().map(|s| format!("'{}'", s)).collect::<Vec<_>>().join(",");
        let in_opp = opp_aliases.iter().map(|s| format!("'{}'", s)).collect::<Vec<_>>().join(",");

        let sql = format!(
            "SELECT player_a, player_b, wins_a, wins_b, draws FROM histories 
             WHERE (player_a IN ({}) AND player_b IN ({}))
                OR (player_a IN ({}) AND player_b IN ({}))",
            in_my, in_opp, in_opp, in_my
        );

        if let Ok(mut stmt) = conn.prepare(&sql) {
            if let Ok(iter) = stmt.query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, i64>(3)?,
                    row.get::<_, i64>(4)?,
                ))
            }) {
                for row in iter.flatten() {
                    let (player_a, _, wins_a, wins_b, draws) = row;
                    if my_aliases.contains(&player_a) {
                        total_wins += wins_a;
                        total_losses += wins_b;
                    } else {
                        total_wins += wins_b;
                        total_losses += wins_a;
                    }
                    total_draws += draws;
                }
            }
        }

        serde_json::json!({
            "me": me,
            "opponent": other,
            "wins": total_wins,
            "losses": total_losses,
            "draws": total_draws,
            "total": total_wins + total_losses + total_draws,
        })
    }

    pub fn check_history(&self, id: &str) -> bool {
        let conn_guard = self.conn.lock();
        let conn = match conn_guard.as_ref() {
            Some(c) => c,
            None => return false,
        };
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM histories WHERE player_a = ?1 OR player_b = ?1",
            params![id],
            |row| row.get(0)
        ).unwrap_or(0);
        count > 0
    }

    pub fn merge_history(&self, old_id: &str, new_id: &str, resolution: &str) {
        let conn_guard = self.conn.lock();
        let conn = match conn_guard.as_ref() {
            Some(c) => c,
            None => return,
        };

        if resolution == "keep_new" {
            let _ = conn.execute("DELETE FROM histories WHERE player_a = ?1 OR player_b = ?1", params![old_id]);
        } else if resolution == "keep_old" {
            let _ = conn.execute("DELETE FROM histories WHERE player_a = ?1 OR player_b = ?1", params![new_id]);
        }
        
        // Always link them so future queries combine them
        let _ = conn.execute("INSERT OR REPLACE INTO user_links (old_id, new_id) VALUES (?1, ?2)", params![old_id, new_id]);
    }

    pub fn get_profile(&self, id: &str) -> serde_json::Value {
        let conn_guard = self.conn.lock();
        let conn = match conn_guard.as_ref() {
            Some(c) => c,
            None => return serde_json::json!({ "error": "DB closed" }),
        };
        
        let my_aliases = Self::get_all_aliases(conn, id);
        let in_my = my_aliases.iter().map(|s| format!("'{}'", s)).collect::<Vec<_>>().join(",");

        let mut total_wins = 0;
        let mut total_losses = 0;
        let mut total_draws = 0;

        let sql = format!("SELECT player_a, player_b, wins_a, wins_b, draws FROM histories WHERE player_a IN ({}) OR player_b IN ({})", in_my, in_my);

        if let Ok(mut stmt) = conn.prepare(&sql) {
            if let Ok(iter) = stmt.query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, i64>(3)?,
                    row.get::<_, i64>(4)?,
                ))
            }) {
                for row in iter.flatten() {
                    let (player_a, _, wins_a, wins_b, draws) = row;
                    if my_aliases.contains(&player_a) {
                        total_wins += wins_a;
                        total_losses += wins_b;
                    } else {
                        total_wins += wins_b;
                        total_losses += wins_a;
                    }
                    total_draws += draws;
                }
            }
        }

        serde_json::json!({
            "id": id,
            "wins": total_wins,
            "losses": total_losses,
            "draws": total_draws,
            "total": total_wins + total_losses + total_draws
        })
    }

    pub fn checkpoint(&self) {
        let mut conn_guard = self.conn.lock();
        if let Some(conn) = conn_guard.as_ref() {
            if let Err(e) = conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);") {
                tracing::error!("Failed to checkpoint database: {}", e);
            } else {
                tracing::info!("Database checkpointed successfully");
            }
        }
    }

    pub fn close(&self) {
        let mut conn_guard = self.conn.lock();
        if let Some(conn) = conn_guard.take() {
            // Explicitly drop the connection to force SQLite to clean up WAL/SHM
            drop(conn);
            tracing::info!("Database connection closed explicitly, WAL files should be cleaned up.");
        }
    }
}
impl Clone for Database {
    fn clone(&self) -> Self {
        Database {
            conn: self.conn.clone(),
        }
    }
}
