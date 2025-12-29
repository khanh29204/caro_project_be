import { Database } from "bun:sqlite";
import { drizzle } from "drizzle-orm/bun-sqlite";
import { mkdirSync } from "fs";
import * as schema from "./schema";

// Đảm bảo thư mục data tồn tại
try {
  mkdirSync("data");
} catch (e) {}

const sqlite = new Database("data/caro.db");
sqlite.exec("PRAGMA journal_mode = WAL;");
sqlite.exec("PRAGMA synchronous = NORMAL;");
export const db = drizzle(sqlite, { schema });
export function initDB() {
  sqlite.run(`
        CREATE TABLE IF NOT EXISTS histories (
            pair_key TEXT PRIMARY KEY,
            player_a TEXT NOT NULL,
            player_b TEXT NOT NULL,
            wins_a INTEGER DEFAULT 0 NOT NULL,
            wins_b INTEGER DEFAULT 0 NOT NULL,
            draws INTEGER DEFAULT 0 NOT NULL,
            updated_at INTEGER
        );
    `);
  sqlite.run("PRAGMA optimize;");
  console.log("Database initialized");
}
