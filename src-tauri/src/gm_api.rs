use crate::models::ScriptModel;
use crate::parser::parse_script;
use crate::storage::ScriptStorage;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tauri::{Manager, State};

pub type GmState = Arc<ScriptStorage>;

/// GM_setValue：将脚本数据持久化到 SQLite。
#[tauri::command]
pub fn gm_storage_set(key: String, value: Value, state: State<'_, GmState>) -> Result<(), String> {
    state.gm_storage_set(&key, &value)
}

/// GM_getValue：从 SQLite 读取脚本数据。
#[tauri::command]
pub fn gm_storage_get(
    key: String,
    default_value: Option<Value>,
    state: State<'_, GmState>,
) -> Result<Value, String> {
    state.gm_storage_get(&key, default_value)
}

/// GM_deleteValue：从 SQLite 删除脚本数据。
#[tauri::command]
pub fn gm_storage_delete(key: String, state: State<'_, GmState>) -> Result<(), String> {
    state.gm_storage_delete(&key)
}

/// GM_xmlhttpRequest 的 Rust 代理。
///
/// 返回完整响应对象，包含 status、statusText、responseText、headers 和 finalUrl，
/// 以便脚本可以像 Tampermonkey 一样访问响应元数据。
#[tauri::command]
pub async fn gm_xhr_proxy(
    url: String,
    method: Option<String>,
    headers: Option<HashMap<String, String>>,
    data: Option<String>,
    response_type: Option<String>,
) -> Result<Value, String> {
    let client = reqwest::Client::new();
    let method = method.unwrap_or_else(|| "GET".to_string());

    let mut request = match method.to_uppercase().as_str() {
        "GET" => client.get(&url),
        "POST" => client.post(&url),
        "PUT" => client.put(&url),
        "DELETE" => client.delete(&url),
        "PATCH" => client.patch(&url),
        "HEAD" => client.head(&url),
        "OPTIONS" => client.request(reqwest::Method::OPTIONS, &url),
        _ => client.get(&url),
    };

    if let Some(headers) = headers {
        for (key, value) in headers {
            request = request.header(key, value);
        }
    }

    if let Some(data) = data {
        request = request.body(data);
    }

    let response = request.send().await.map_err(|e| e.to_string())?;
    let status = response.status();
    let final_url = response.url().to_string();

    let mut response_headers = HashMap::new();
    for (key, value) in response.headers() {
        if let Ok(v) = value.to_str() {
            response_headers.insert(key.to_string(), v.to_string());
        }
    }

    let response_type = response_type.unwrap_or_else(|| "text".to_string());
    let body = if response_type == "json" {
        match response.json::<Value>().await {
            Ok(v) => serde_json::to_string(&v).unwrap_or_default(),
            Err(e) => return Err(format!("响应 JSON 解析失败: {}", e)),
        }
    } else {
        response.text().await.map_err(|e| e.to_string())?
    };

    Ok(serde_json::json!({
        "status": status.as_u16(),
        "statusText": status.canonical_reason().unwrap_or("Unknown"),
        "responseText": body,
        "responseHeaders": response_headers,
        "finalUrl": final_url,
        "readyState": 4
    }))
}

/// 从代码字符串安装脚本（核心逻辑，供命令和自动安装共用）。
///
/// 同名同命名空间脚本执行更新而非重复创建。
pub fn install_from_code(code: &str, storage: &ScriptStorage) -> Result<ScriptModel, String> {
    let mut script = parse_script(code)?;

    if let Some(existing) = storage.find_by_identity(&script.name, &script.namespace)? {
        script.id = existing.id;
        script.enabled = existing.enabled;
        storage.delete(&script.id)?;
    } else {
        script.id = format!(
            "script_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_err(|e| e.to_string())?
                .as_millis()
        );
    }

    storage.add(script.clone())?;
    Ok(script)
}

/// 从 URL 下载并安装脚本（核心逻辑，供命令和自动安装共用）。
pub async fn fetch_and_install(url: &str, storage: &ScriptStorage) -> Result<ScriptModel, String> {
    let client = reqwest::Client::new();
    let code = client
        .get(url)
        .send()
        .await
        .map_err(|e| format!("下载脚本失败: {}", e))?
        .text()
        .await
        .map_err(|e| format!("读取脚本响应失败: {}", e))?;

    install_from_code(&code, storage)
}

/// 从代码字符串安装脚本（Tauri 命令入口）。
#[tauri::command]
pub fn install_script(code: String, state: State<'_, GmState>) -> Result<ScriptModel, String> {
    install_from_code(&code, &state)
}

/// 从 URL 下载并安装脚本（Tauri 命令入口）。
#[tauri::command]
pub async fn install_script_from_url(url: String, state: State<'_, GmState>) -> Result<ScriptModel, String> {
    fetch_and_install(&url, &state).await
}

/// 列出所有已安装脚本。
#[tauri::command]
pub fn list_scripts(state: State<'_, GmState>) -> Result<Vec<ScriptModel>, String> {
    state.list()
}

/// 切换脚本启用状态。
#[tauri::command]
pub fn toggle_script(id: String, enabled: bool, state: State<'_, GmState>) -> Result<(), String> {
    state.update(&id, enabled)
}

/// 删除脚本。
#[tauri::command]
pub fn delete_script(id: String, state: State<'_, GmState>) -> Result<(), String> {
    state.delete(&id)
}

/// 查找匹配指定 URL 的脚本。
#[tauri::command]
pub fn find_scripts_for_url(url: String, state: State<'_, GmState>) -> Result<Vec<ScriptModel>, String> {
    state.find_by_url(&url)
}

/// 导航到指定 URL。
#[tauri::command]
pub fn navigate_to_url(url: String, app: tauri::AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("main") {
        let parsed = url.parse::<url::Url>().map_err(|e| format!("URL 解析失败: {}", e))?;
        window.navigate(parsed).map_err(|e| e.to_string())?;
    }
    Ok(())
}
