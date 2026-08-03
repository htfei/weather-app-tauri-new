mod radio;
mod stream_proxy;

use radio::{
    add_play_history, get_favorites, get_radio_catalog, save_radio_catalog, toggle_favorite,
    update_radio_catalog_from_url, RadioStorage,
};
use std::sync::Arc;
use tauri::WebviewWindowBuilder;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let radio_storage = Arc::new(RadioStorage::new().expect("无法创建电台存储"));

    tauri::Builder::default()
        .manage(radio_storage)
        .invoke_handler(tauri::generate_handler![
            get_radio_catalog,
            update_radio_catalog_from_url,
            save_radio_catalog,
            toggle_favorite,
            get_favorites,
            add_play_history,
            stream_proxy::proxy_stream_url,
        ])
        .setup(move |app| {
            let _window = WebviewWindowBuilder::new(app, "main", tauri::WebviewUrl::default())
                .title("TingFM Radio")
                .inner_size(1100.0, 760.0)
                .min_inner_size(640.0, 480.0)
                .resizable(true)
                .decorations(true)
                .build()?;

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
