mod gm_api;
mod matcher;
mod models;
mod parser;
mod storage;

use gm_api::{
    delete_script, fetch_and_install, find_scripts_for_url, gm_storage_delete, gm_storage_get,
    gm_storage_set, gm_xhr_proxy, install_script, install_script_from_url, list_scripts,
    navigate_to_url, toggle_script, GmState,
};
use std::sync::Arc;
use storage::ScriptStorage;
use tauri::webview::PageLoadEvent;
use tauri::{Manager, WebviewWindowBuilder};

/// 应用配置：根据构建模式区分前端入口地址。
struct AppConfig {
    home_url: &'static str,
}

impl AppConfig {
    fn new() -> Self {
        Self {
            home_url: if cfg!(debug_assertions) {
                "http://localhost:5173"
            } else {
                "tauri://localhost"
            },
        }
    }
}

/// 将资源文件中的桥接脚本模板渲染为可注入 WebView 的 JS。
///
/// 使用会话级 token 作为执行入口的凭证，防止外部页面越权触发脚本执行。
/// 同时注入导航桥接函数，使外部页面可以通过 Rust 端执行导航。
fn build_bridge_script(token: &str) -> String {
    let bridge = include_str!("../resources/gm_bridge.js").replace("{{WEBWRAPPER_TOKEN}}", token);
    // 注入导航桥接函数：外部页面调用 window.__webwrapper_navigate__(url) 时，
    // 通过 Rust 端 WebviewWindow::navigate 执行真实跳转，避免跨协议限制
    let nav_bridge = r#"
        window.__webwrapper_navigate__ = function(url) {
            if (window.__TAURI_INTERNALS__ && typeof window.__TAURI_INTERNALS__.invoke === 'function') {
                window.__TAURI_INTERNALS__.invoke('navigate_to_url', { url: url });
            }
        };
    "#;
    format!("{}\n{}", nav_bridge, bridge)
}

/// 将资源文件中的浮动导航栏脚本模板渲染为可注入 WebView 的 JS。
fn build_navbar_script(home_url: &str) -> String {
    include_str!("../resources/navbar.js").replace("{{WEBWRAPPER_HOME_URL}}", home_url)
}

/// 构造批量执行油猴脚本的 JS 代码，减少 IPC 往返。
fn build_batch_execute_js(token: &str, scripts: &[models::ScriptModel]) -> String {
    let configs: Vec<_> = scripts
        .iter()
        .map(|s| {
            serde_json::json!({
                "id": s.id,
                "name": s.name,
                "version": s.version,
                "code": s.code,
                "runAt": s.run_at,
                "requires": s.requires,
                "grants": s.grants,
            })
        })
        .collect();
    format!(
        "if(window.__webwrapper_bridge__) window.__webwrapper_bridge__.executeScripts('{}', {});",
        token,
        serde_json::to_string(&configs).unwrap_or_else(|_| "[]".to_string())
    )
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let storage = Arc::new(ScriptStorage::new().expect("无法创建脚本存储"));
    let config = AppConfig::new();

    // 会话级 token，用于防止外部网页越权调用脚本执行入口
    let bridge_token = uuid::Uuid::new_v4().to_string();
    let navbar = build_navbar_script(config.home_url);
    let bridge_for_setup = bridge_token.clone();

    tauri::Builder::default()
        .manage(storage.clone() as GmState)
        .invoke_handler(tauri::generate_handler![
            navigate_to_url,
            install_script,
            install_script_from_url,
            list_scripts,
            toggle_script,
            delete_script,
            find_scripts_for_url,
            gm_storage_get,
            gm_storage_set,
            gm_storage_delete,
            gm_xhr_proxy
        ])
        .setup(move |app| {
            let app_handle = app.handle().clone();
            let bridge = build_bridge_script(&bridge_for_setup);
            let storage_for_nav = storage.clone();
            let storage_for_load = storage.clone();
            let token_for_nav = bridge_token.clone();
            let token_for_load = bridge_token.clone();
            let navbar_for_load = navbar.clone();

            let _window = WebviewWindowBuilder::new(app, "main", tauri::WebviewUrl::default())
                .title("WebWrapper")
                .inner_size(1200.0, 800.0)
                .resizable(true)
                .decorations(true)
                .on_navigation(move |url| {
                    let url_string = url.to_string();
                    let storage = storage_for_nav.clone();
                    let app_handle = app_handle.clone();
                    let token = token_for_nav.clone();

                    // 自动捕获 .user.js 网址并安装脚本，避免在 WebView 中直接显示脚本源码
                    if let Ok(parsed) = url_string.parse::<url::Url>() {
                        if parsed.path().to_lowercase().ends_with(".user.js") {
                            let install_url = url_string.clone();
                            let storage_for_install = storage.clone();
                            let app_handle_for_install = app_handle.clone();

                            tauri::async_runtime::spawn(async move {
                                // 先导航回主页，然后在主页显示安装反馈
                                let home_url = AppConfig::new().home_url.parse::<url::Url>().unwrap();
                                if let Some(window) = app_handle_for_install.get_webview_window("main") {
                                    let _ = window.navigate(home_url);
                                }

                                // 等待主页加载完成
                                tokio::time::sleep(std::time::Duration::from_millis(500)).await;

                                // 显示安装中提示
                                if let Some(window) = app_handle_for_install.get_webview_window("main") {
                                    let overlay_js = r#"
                                        (function(){
                                            var o = document.getElementById('ww-install-overlay');
                                            if(!o){
                                                o = document.createElement('div');
                                                o.id = 'ww-install-overlay';
                                                o.style.cssText = 'position:fixed;top:50%;left:50%;transform:translate(-50%,-50%);background:rgba(0,0,0,0.85);color:#fff;padding:24px 40px;border-radius:12px;font-size:16px;z-index:2147483647;font-family:sans-serif;text-align:center;min-width:200px;';
                                                o.innerHTML = '<div style="margin-bottom:8px;">⏳ 正在安装脚本...</div><div style="font-size:12px;color:#aaa;">请稍候</div>';
                                                document.body.appendChild(o);
                                            }
                                        })();
                                    "#;
                                    let _ = window.eval(overlay_js);
                                }

                                match fetch_and_install(&install_url, &storage_for_install).await {
                                    Ok(script) => {
                                        let name = script.name.replace('\'', "\\'");
                                        let version = script.version.replace('\'', "\\'");
                                        let success_js = format!(
                                            r#"
                                                (function(){{
                                                    var o = document.getElementById('ww-install-overlay');
                                                    if(o){{
                                                        o.innerHTML = '<div style="margin-bottom:8px;">✅ 安装成功</div><div style="font-size:13px;">{} v{}</div>';
                                                        o.style.background = 'rgba(0,128,0,0.85)';
                                                    }}
                                                }})();
                                            "#,
                                            name, version
                                        );
                                        if let Some(window) = app_handle_for_install.get_webview_window("main") {
                                            let _ = window.eval(&success_js);
                                        }
                                        // 1.5秒后移除覆盖层
                                        tauri::async_runtime::spawn(async move {
                                            tokio::time::sleep(std::time::Duration::from_millis(1500)).await;
                                            if let Some(window) = app_handle_for_install.get_webview_window("main") {
                                                let _ = window.eval("var o = document.getElementById('ww-install-overlay'); if(o) o.remove();");
                                            }
                                        });
                                    }
                                    Err(e) => {
                                        let err_msg = e.replace('\'', "\\'").replace('\n', " ");
                                        let error_js = format!(
                                            r#"
                                                (function(){{
                                                    var o = document.getElementById('ww-install-overlay');
                                                    if(o){{
                                                        o.innerHTML = '<div style="margin-bottom:8px;">❌ 安装失败</div><div style="font-size:12px;color:#ff9999;">{}</div>';
                                                        o.style.background = 'rgba(200,0,0,0.85)';
                                                    }}
                                                }})();
                                            "#,
                                            err_msg
                                        );
                                        if let Some(window) = app_handle_for_install.get_webview_window("main") {
                                            let _ = window.eval(&error_js);
                                        }
                                        tauri::async_runtime::spawn(async move {
                                            tokio::time::sleep(std::time::Duration::from_millis(2500)).await;
                                            if let Some(window) = app_handle_for_install.get_webview_window("main") {
                                                let _ = window.eval("var o = document.getElementById('ww-install-overlay'); if(o) o.remove();");
                                            }
                                        });
                                    }
                                }
                            });
                            return false;
                        }
                    }

                    // 在导航发生时尽快注入 document-start 脚本；
                    // on_page_load 的 Started 事件会作为兜底，并通过桥接层去重。
                    tauri::async_runtime::spawn(async move {
                        match storage.find_by_url(&url_string) {
                            Ok(scripts) => {
                                let start_scripts: Vec<_> = scripts
                                    .into_iter()
                                    .filter(|s| s.run_at == "document-start")
                                    .collect();
                                if !start_scripts.is_empty() {
                                    let js = build_batch_execute_js(&token, &start_scripts);
                                    if let Some(window) = app_handle.get_webview_window("main") {
                                        let _ = window.eval(&js);
                                    }
                                }
                            }
                            Err(e) => eprintln!("匹配脚本失败: {}", e),
                        }
                    });

                    true
                })
                .on_page_load({
                    let storage = storage_for_load.clone();
                    move |webview, payload| {
                        let url_string = payload.url().to_string();

                        // 页面加载完成后注入浮动导航栏，避免 initialization_script 时机不稳定导致的问题
                        if payload.event() == PageLoadEvent::Finished {
                            let _ = webview.eval(&navbar_for_load);
                        }

                        let scripts = match storage.find_by_url(&url_string) {
                            Ok(s) => s,
                            Err(_) => return,
                        };

                        match payload.event() {
                            PageLoadEvent::Started => {
                                let start_scripts: Vec<_> = scripts
                                    .into_iter()
                                    .filter(|s| s.run_at == "document-start")
                                    .collect();
                                if !start_scripts.is_empty() {
                                    let js = build_batch_execute_js(&token_for_load, &start_scripts);
                                    let _ = webview.eval(&js);
                                }
                            }
                            PageLoadEvent::Finished => {
                                let end_scripts: Vec<_> = scripts
                                    .into_iter()
                                    .filter(|s| {
                                        s.run_at == "document-end" || s.run_at == "document-idle"
                                    })
                                    .collect();
                                if !end_scripts.is_empty() {
                                    let js = build_batch_execute_js(&token_for_load, &end_scripts);
                                    let _ = webview.eval(&js);
                                }
                            }
                        }
                    }
                })
                .initialization_script(&bridge)
                .build()?;

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
