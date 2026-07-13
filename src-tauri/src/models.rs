use serde::{Deserialize, Serialize};

/// 油猴脚本持久化模型。
///
/// 所有字段均通过 Rust 端解析并序列化后存储在 SQLite 中，
/// 前端仅接收 JSON 表示，避免直接处理原始脚本代码的解析逻辑。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScriptModel {
    pub id: String,
    pub name: String,
    pub namespace: String,
    pub version: String,
    pub homepage: String,
    pub icon: String,
    pub update_url: String,
    pub matches: Vec<String>,
    pub includes: Vec<String>,
    pub excludes: Vec<String>,
    pub run_at: String,
    pub requires: Vec<String>,
    pub grants: Vec<String>,
    pub code: String,
    pub enabled: bool,
}

impl Default for ScriptModel {
    fn default() -> Self {
        Self {
            id: String::new(),
            name: String::from("未命名脚本"),
            namespace: String::new(),
            version: String::from("1.0"),
            homepage: String::new(),
            icon: String::new(),
            update_url: String::new(),
            matches: Vec::new(),
            includes: Vec::new(),
            excludes: Vec::new(),
            run_at: String::from("document-end"),
            requires: Vec::new(),
            grants: Vec::new(),
            code: String::new(),
            enabled: true,
        }
    }
}
