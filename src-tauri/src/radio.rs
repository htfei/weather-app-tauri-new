use rusqlite::{Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::State;

/// 电台流信息。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RadioStream {
    pub url: String,
    pub format: String,
    #[serde(default)]
    pub quality: Option<String>,
}

/// 电台源信息。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RadioSource {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub logo: Option<String>,
    #[serde(default)]
    pub category: Option<String>,
    #[serde(default)]
    pub region: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    pub streams: Vec<RadioStream>,
}

/// 电台目录。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RadioCatalog {
    pub version: String,
    pub platform: String,
    #[serde(rename = "updatedAt")]
    pub updated_at: String,
    pub sources: Vec<RadioSource>,
}

/// 电台数据持久化仓库。
pub struct RadioStorage {
    data_dir: PathBuf,
    conn: Mutex<Connection>,
}

impl RadioStorage {
    pub fn new() -> Result<Self, String> {
        let data_dir = dirs::data_dir()
            .ok_or("无法获取数据目录")?
            .join("TingFM Radio");

        std::fs::create_dir_all(&data_dir).map_err(|e| e.to_string())?;

        let db_path = data_dir.join("radio.db");
        let conn = Connection::open(&db_path).map_err(|e| e.to_string())?;

        let storage = Self {
            data_dir,
            conn: Mutex::new(conn),
        };
        storage.init_tables()?;
        Ok(storage)
    }

    fn init_tables(&self) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.execute(
            "CREATE TABLE IF NOT EXISTS favorites (
                source_id TEXT PRIMARY KEY,
                created_at INTEGER NOT NULL
            )",
            [],
        )
        .map_err(|e| e.to_string())?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS play_history (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                source_id TEXT NOT NULL,
                played_at INTEGER NOT NULL
            )",
            [],
        )
        .map_err(|e| e.to_string())?;

        Ok(())
    }

    /// 目录文件路径。
    pub fn catalog_path(&self) -> PathBuf {
        self.data_dir.join("radio-catalog.json")
    }

    /// 读取本地目录，不存在时返回 None。
    pub fn read_catalog(&self) -> Result<Option<RadioCatalog>, String> {
        let path = self.catalog_path();
        if !path.exists() {
            return Ok(None);
        }
        let content =
            std::fs::read_to_string(&path).map_err(|e| format!("读取目录失败: {}", e))?;
        let catalog: RadioCatalog =
            serde_json::from_str(&content).map_err(|e| format!("解析目录失败: {}", e))?;
        Ok(Some(catalog))
    }

    /// 保存目录到本地文件。
    pub fn save_catalog(&self, catalog: &RadioCatalog) -> Result<(), String> {
        let path = self.catalog_path();
        let content = serde_json::to_string_pretty(catalog).map_err(|e| e.to_string())?;
        std::fs::write(&path, content).map_err(|e| format!("保存目录失败: {}", e))?;
        Ok(())
    }

    /// 切换收藏状态，返回当前是否收藏。
    pub fn toggle_favorite(&self, source_id: &str) -> Result<bool, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let exists: Option<i64> = conn
            .query_row(
                "SELECT 1 FROM favorites WHERE source_id = ?1",
                [source_id],
                |row| row.get(0),
            )
            .optional()
            .map_err(|e| e.to_string())?;

        if exists.is_some() {
            conn.execute(
                "DELETE FROM favorites WHERE source_id = ?1",
                [source_id],
            )
            .map_err(|e| e.to_string())?;
            Ok(false)
        } else {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_err(|e| e.to_string())?
                .as_secs() as i64;
            conn.execute(
                "INSERT INTO favorites (source_id, created_at) VALUES (?1, ?2)",
                [source_id, &now.to_string()],
            )
            .map_err(|e| e.to_string())?;
            Ok(true)
        }
    }

    pub fn get_favorites(&self) -> Result<Vec<String>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let mut stmt = conn
            .prepare("SELECT source_id FROM favorites ORDER BY created_at DESC")
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |row| row.get::<_, String>(0))
            .map_err(|e| e.to_string())?;

        let mut list = Vec::new();
        for row in rows {
            list.push(row.map_err(|e| e.to_string())?);
        }
        Ok(list)
    }

    pub fn add_play_history(&self, source_id: &str) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| e.to_string())?
            .as_secs() as i64;
        conn.execute(
            "INSERT INTO play_history (source_id, played_at) VALUES (?1, ?2)",
            [source_id, &now.to_string()],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }
}

/// 获取本地电台目录。
#[tauri::command]
pub fn get_radio_catalog(state: State<'_, RadioStorage>) -> Result<RadioCatalog, String> {
    match state.read_catalog()? {
        Some(catalog) => Ok(catalog),
        None => Err("未找到电台目录".to_string()),
    }
}

/// 从远程 URL 下载并更新目录。
#[tauri::command]
pub async fn update_radio_catalog_from_url(
    url: String,
    state: State<'_, RadioStorage>,
) -> Result<(), String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| e.to_string())?;

    let text = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("下载目录失败: {}", e))?
        .text()
        .await
        .map_err(|e| format!("读取目录失败: {}", e))?;

    let catalog: RadioCatalog =
        serde_json::from_str(&text).map_err(|e| format!("解析目录失败: {}", e))?;

    state.save_catalog(&catalog)
}

/// 直接保存目录（用于本地文件导入）。
#[tauri::command]
pub fn save_radio_catalog(
    catalog: RadioCatalog,
    state: State<'_, RadioStorage>,
) -> Result<(), String> {
    state.save_catalog(&catalog)
}

/// 切换收藏状态。
#[tauri::command]
pub fn toggle_favorite(id: String, state: State<'_, RadioStorage>) -> Result<bool, String> {
    state.toggle_favorite(&id)
}

/// 获取收藏列表。
#[tauri::command]
pub fn get_favorites(state: State<'_, RadioStorage>) -> Result<Vec<String>, String> {
    state.get_favorites()
}

/// 添加播放历史。
#[tauri::command]
pub fn add_play_history(id: String, state: State<'_, RadioStorage>) -> Result<(), String> {
    state.add_play_history(&id)
}
