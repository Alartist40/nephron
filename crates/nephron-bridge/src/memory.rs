//! Dendrite-compatible SQLite + FTS5 local memory store for Nephron.

use anyhow::Result;
use chrono::Utc;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryRecord {
    pub id: i64,
    pub kind: String,
    pub content: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CadPart {
    pub id: i64,
    pub name: String,
    pub material: String,
    pub quantity: i32,
    pub scad_path: String,
}

pub struct MemoryStore {
    conn: Connection,
}

impl MemoryStore {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        if let Some(parent) = path.as_ref().parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(path)?;
        Self::init(conn)
    }

    pub fn open_in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        Self::init(conn)
    }

    fn init(conn: Connection) -> Result<Self> {
        conn.execute_batch(
            "PRAGMA journal_mode=WAL;
             CREATE TABLE IF NOT EXISTS memories (
                 id INTEGER PRIMARY KEY AUTOINCREMENT,
                 kind TEXT NOT NULL,
                 content TEXT NOT NULL,
                 created_at TEXT NOT NULL
             );
             CREATE VIRTUAL TABLE IF NOT EXISTS memories_fts USING fts5(
                 content, content='memories', content_rowid='id'
             );
             CREATE TRIGGER IF NOT EXISTS memories_ai AFTER INSERT ON memories BEGIN
                 INSERT INTO memories_fts(rowid, content) VALUES (new.id, new.content);
             END;
             CREATE TABLE IF NOT EXISTS cad_parts (
                 id INTEGER PRIMARY KEY AUTOINCREMENT,
                 name TEXT NOT NULL,
                 material TEXT NOT NULL,
                 quantity INTEGER NOT NULL,
                 scad_path TEXT NOT NULL
             );",
        )?;
        Ok(Self { conn })
    }

    pub fn save_memory(&self, kind: &str, content: &str) -> Result<i64> {
        let now = Utc::now().to_rfc3339();
        self.conn.execute(
            "INSERT INTO memories (kind, content, created_at) VALUES (?1, ?2, ?3)",
            params![kind, content, now],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn recent_memories(&self, limit: usize) -> Result<Vec<MemoryRecord>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, kind, content, created_at FROM memories ORDER BY id DESC LIMIT ?1",
        )?;
        let rows = stmt.query_map(params![limit as i64], |r| {
            Ok(MemoryRecord {
                id: r.get(0)?,
                kind: r.get(1)?,
                content: r.get(2)?,
                created_at: r.get(3)?,
            })
        })?;
        Ok(rows.filter_map(Result::ok).collect())
    }

    pub fn search_fts(&self, query: &str, limit: usize) -> Result<Vec<MemoryRecord>> {
        let safe = format!("\"{}\"", query.replace('"', "\"\""));
        let mut stmt = self.conn.prepare(
            "SELECT m.id, m.kind, m.content, m.created_at
             FROM memories_fts f JOIN memories m ON m.id = f.rowid
             WHERE memories_fts MATCH ?1 ORDER BY bm25(memories_fts) LIMIT ?2",
        )?;
        let rows = stmt.query_map(params![safe, limit as i64], |r| {
            Ok(MemoryRecord {
                id: r.get(0)?,
                kind: r.get(1)?,
                content: r.get(2)?,
                created_at: r.get(3)?,
            })
        })?;
        Ok(rows.filter_map(Result::ok).collect())
    }

    pub fn save_part(&self, name: &str, material: &str, quantity: i32, scad_path: &str) -> Result<i64> {
        self.conn.execute(
            "INSERT INTO cad_parts (name, material, quantity, scad_path) VALUES (?1, ?2, ?3, ?4)",
            params![name, material, quantity, scad_path],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn list_parts(&self) -> Result<Vec<CadPart>> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, name, material, quantity, scad_path FROM cad_parts ORDER BY id DESC")?;
        let rows = stmt.query_map([], |r| {
            Ok(CadPart {
                id: r.get(0)?,
                name: r.get(1)?,
                material: r.get(2)?,
                quantity: r.get(3)?,
                scad_path: r.get(4)?,
            })
        })?;
        Ok(rows.filter_map(Result::ok).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn saves_and_searches_memories_and_parts() {
        let store = MemoryStore::open_in_memory().unwrap();
        store.save_memory("chat", "Orange Pi 6 Plus enclosure design").unwrap();
        let hits = store.search_fts("enclosure", 5).unwrap();
        assert_eq!(hits.len(), 1);
        assert!(hits[0].content.contains("Orange Pi"));

        store.save_part("Top Lid", "PETG", 1, "parts/lid.scad").unwrap();
        let parts = store.list_parts().unwrap();
        assert_eq!(parts.len(), 1);
        assert_eq!(parts[0].name, "Top Lid");
    }
}
