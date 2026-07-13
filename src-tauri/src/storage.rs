use crate::models::ScriptModel;
use rusqlite::{Connection, OptionalExtension};
use serde_json;
use std::sync::Mutex;

/// 基于 SQLite 的脚本与 GM 存储仓库。
///
/// 规则要求：涉及核心资产数据必须在 Rust 端使用 rusqlite 落地为本地文件。
pub struct ScriptStorage {
    conn: Mutex<Connection>,
}

impl ScriptStorage {
    pub fn new() -> Result<Self, String> {
        let data_dir = dirs::data_dir()
            .ok_or("无法获取数据目录")?
            .join("WebWrapper");

        std::fs::create_dir_all(&data_dir).map_err(|e| e.to_string())?;

        let db_path = data_dir.join("scripts.db");
        let conn = Connection::open(&db_path).map_err(|e| e.to_string())?;

        let storage = Self {
            conn: Mutex::new(conn),
        };

        storage.init_tables()?;
        Ok(storage)
    }

    fn init_tables(&self) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.execute(
            "CREATE TABLE IF NOT EXISTS scripts (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                namespace TEXT NOT NULL DEFAULT '',
                version TEXT NOT NULL,
                homepage TEXT NOT NULL DEFAULT '',
                icon TEXT NOT NULL DEFAULT '',
                update_url TEXT NOT NULL DEFAULT '',
                matches TEXT NOT NULL,
                includes TEXT NOT NULL,
                excludes TEXT NOT NULL,
                run_at TEXT NOT NULL,
                requires TEXT NOT NULL,
                grants TEXT NOT NULL,
                code TEXT NOT NULL,
                enabled INTEGER NOT NULL DEFAULT 1
            )",
            [],
        )
        .map_err(|e| e.to_string())?;

        // 兼容旧版数据库：补充新增字段
        for col in ["namespace", "homepage", "icon", "update_url"] {
            let sql = format!("ALTER TABLE scripts ADD COLUMN {} TEXT NOT NULL DEFAULT ''", col);
            let _ = conn.execute(&sql, []);
        }

        conn.execute(
            "CREATE TABLE IF NOT EXISTS gm_storage (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            )",
            [],
        )
        .map_err(|e| e.to_string())?;

        Ok(())
    }

    pub fn add(&self, script: ScriptModel) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.execute(
            "INSERT INTO scripts (id, name, namespace, version, homepage, icon, update_url, matches, includes, excludes, run_at, requires, grants, code, enabled)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
            [
                &script.id,
                &script.name,
                &script.namespace,
                &script.version,
                &script.homepage,
                &script.icon,
                &script.update_url,
                &serde_json::to_string(&script.matches).map_err(|e| e.to_string())?,
                &serde_json::to_string(&script.includes).map_err(|e| e.to_string())?,
                &serde_json::to_string(&script.excludes).map_err(|e| e.to_string())?,
                &script.run_at,
                &serde_json::to_string(&script.requires).map_err(|e| e.to_string())?,
                &serde_json::to_string(&script.grants).map_err(|e| e.to_string())?,
                &script.code,
                &(if script.enabled { 1 } else { 0 }).to_string(),
            ],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn list(&self) -> Result<Vec<ScriptModel>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let mut stmt = conn
            .prepare("SELECT id, name, namespace, version, homepage, icon, update_url, matches, includes, excludes, run_at, requires, grants, code, enabled FROM scripts")
            .map_err(|e| e.to_string())?;

        let rows = stmt
            .query_map([], |row| {
                Ok(ScriptModel {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    namespace: row.get(2)?,
                    version: row.get(3)?,
                    homepage: row.get(4)?,
                    icon: row.get(5)?,
                    update_url: row.get(6)?,
                    matches: serde_json::from_str(&row.get::<_, String>(7)?).unwrap_or_default(),
                    includes: serde_json::from_str(&row.get::<_, String>(8)?).unwrap_or_default(),
                    excludes: serde_json::from_str(&row.get::<_, String>(9)?).unwrap_or_default(),
                    run_at: row.get(10)?,
                    requires: serde_json::from_str(&row.get::<_, String>(11)?).unwrap_or_default(),
                    grants: serde_json::from_str(&row.get::<_, String>(12)?).unwrap_or_default(),
                    code: row.get(13)?,
                    enabled: row.get::<_, i32>(14)? != 0,
                })
            })
            .map_err(|e| e.to_string())?;

        let mut scripts = Vec::new();
        for row in rows {
            scripts.push(row.map_err(|e| e.to_string())?);
        }
        Ok(scripts)
    }

    pub fn update(&self, id: &str, enabled: bool) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.execute(
            "UPDATE scripts SET enabled = ?1 WHERE id = ?2",
            rusqlite::params![enabled as i32, id],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn delete(&self, id: &str) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.execute("DELETE FROM scripts WHERE id = ?1", [id])
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    /// 根据名称与命名空间查找已有脚本，用于重复检测与更新。
    pub fn find_by_identity(&self, name: &str, namespace: &str) -> Result<Option<ScriptModel>, String> {
        let scripts = self.list()?;
        Ok(scripts.into_iter().find(|s| {
            if !namespace.is_empty() && !s.namespace.is_empty() {
                s.name == name && s.namespace == namespace
            } else {
                s.name == name
            }
        }))
    }

    pub fn find_by_url(&self, url: &str) -> Result<Vec<ScriptModel>, String> {
        use crate::matcher::is_url_matched;
        let scripts = self.list()?;

        Ok(scripts
            .into_iter()
            .filter(|s| {
                s.enabled
                    && !is_url_matched(url, &s.excludes)
                    && (is_url_matched(url, &s.matches) || is_url_matched(url, &s.includes))
            })
            .collect())
    }

    pub fn gm_storage_set(&self, key: &str, value: &serde_json::Value) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let value_json = serde_json::to_string(value).map_err(|e| e.to_string())?;
        conn.execute(
            "INSERT INTO gm_storage (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            [key, &value_json],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn gm_storage_get(
        &self,
        key: &str,
        default_value: Option<serde_json::Value>,
    ) -> Result<serde_json::Value, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let value: Option<String> = conn
            .query_row(
                "SELECT value FROM gm_storage WHERE key = ?1",
                [key],
                |row| row.get(0),
            )
            .optional()
            .map_err(|e| e.to_string())?;

        match value {
            Some(v) => serde_json::from_str(&v).map_err(|e| e.to_string()),
            None => Ok(default_value.unwrap_or(serde_json::Value::Null)),
        }
    }

    pub fn gm_storage_delete(&self, key: &str) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.execute("DELETE FROM gm_storage WHERE key = ?1", [key])
            .map_err(|e| e.to_string())?;
        Ok(())
    }
}
