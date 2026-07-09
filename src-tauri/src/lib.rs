use tauri::WebviewWindowBuilder;
use serde_json;

const TARGET_URL: &str = "https://newsnow.busiyi.world/";

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let script_content = include_str!("../resources/chat.user.js");
    
    let script_injector = format!(
        r#"
        (function() {{
            window.GM_getValue = function(key, defaultValue) {{
                try {{
                    var val = localStorage.getItem('gm_' + key);
                    return val ? JSON.parse(val) : defaultValue;
                }} catch(e) {{ return defaultValue; }}
            }};
            
            window.GM_setValue = function(key, value) {{
                try {{
                    localStorage.setItem('gm_' + key, JSON.stringify(value));
                }} catch(e) {{}}
            }};
            
            window.GM_xmlhttpRequest = function(options) {{
                return fetch(options.url, {{
                    method: options.method || 'GET',
                    headers: options.headers || {{}},
                    body: options.data || null
                }}).then(function(response) {{
                    return response.text().then(function(text) {{
                        if (options.onload) {{
                            options.onload({{
                                responseText: text,
                                status: response.status
                            }});
                        }}
                        return {{ text: text, status: response.status }};
                    }});
                }});
            }};
            
            window.GM_info = {{
                script: {{
                    name: 'Chat',
                    version: '1.0'
                }}
            }};
            
            var loadScript = function(url, callback) {{
                var s = document.createElement('script');
                s.src = url;
                s.onload = callback;
                s.onerror = callback;
                (document.head || document.documentElement).appendChild(s);
            }};
            
            document.addEventListener('click', function(e) {{
                var target = e.target;
                while (target && target.nodeType === 1 && target.tagName !== 'A') {{
                    target = target.parentNode;
                }}
                if (target && target.tagName === 'A') {{
                    var href = target.getAttribute('href');
                    var targetAttr = target.getAttribute('target');
                    if (href && href !== '#' && !href.startsWith('javascript:')) {{
                        if (targetAttr === '_blank' || targetAttr === '_new') {{
                            e.preventDefault();
                            if (!href.startsWith('http')) {{
                                var base = window.location.origin + window.location.pathname;
                                href = new URL(href, base).href;
                            }}
                            window.location.href = href;
                        }}
                    }}
                }}
            }}, true);
            
            var createNavBar = function() {{
                var navBar = document.createElement('div');
                navBar.style.cssText = 'position:fixed;top:0;left:0;right:0;height:44px;background:rgba(255,255,255,0.95);border-bottom:1px solid #eee;display:flex;align-items:center;padding:0 8px;z-index:9999;box-shadow:0 2px 4px rgba(0,0,0,0.1);backdrop-filter:blur(10px);';
                
                var backBtn = document.createElement('button');
                backBtn.innerHTML = '←';
                backBtn.style.cssText = 'width:32px;height:32px;border:none;background:#f0f0f0;border-radius:6px;cursor:pointer;font-size:16px;margin-right:4px;';
                backBtn.onclick = function() {{ window.history.back(); }};
                
                var forwardBtn = document.createElement('button');
                forwardBtn.innerHTML = '→';
                forwardBtn.style.cssText = 'width:32px;height:32px;border:none;background:#f0f0f0;border-radius:6px;cursor:pointer;font-size:16px;margin-right:4px;';
                forwardBtn.onclick = function() {{ window.history.forward(); }};
                
                var homeBtn = document.createElement('button');
                homeBtn.innerHTML = '🏠';
                homeBtn.style.cssText = 'width:32px;height:32px;border:none;background:#f0f0f0;border-radius:6px;cursor:pointer;font-size:16px;margin-right:8px;';
                homeBtn.onclick = function() {{ window.location.href = 'https://newsnow.busiyi.world/'; }};
                
                var urlInput = document.createElement('input');
                urlInput.type = 'text';
                urlInput.value = window.location.href;
                urlInput.style.cssText = 'flex:1;height:32px;border:1px solid #ddd;border-radius:6px;padding:0 8px;font-size:14px;';
                urlInput.onkeydown = function(e) {{
                    if (e.key === 'Enter') {{
                        var url = this.value.trim();
                        if (!url.startsWith('http')) {{
                            url = 'https://' + url;
                        }}
                        window.location.href = url;
                    }}
                }};
                
                navBar.appendChild(backBtn);
                navBar.appendChild(forwardBtn);
                navBar.appendChild(homeBtn);
                navBar.appendChild(urlInput);
                
                document.body.style.paddingTop = '44px';
                document.body.appendChild(navBar);
                
                window.addEventListener('popstate', function() {{
                    urlInput.value = window.location.href;
                }});
            }};
            
            var injectScript = function() {{
                createNavBar();
                loadScript('https://unpkg.com/@supabase/supabase-js@2.49.3/dist/umd/supabase.js', function() {{
                    var script = document.createElement('script');
                    script.textContent = {};
                    (document.head || document.documentElement || document.body).appendChild(script);
                }});
            }};
            
            if (document.readyState === 'loading') {{
                document.addEventListener('DOMContentLoaded', injectScript);
            }} else {{
                injectScript();
            }}
        }})();
        "#,
        serde_json::to_string(script_content).unwrap()
    );

    tauri::Builder::default()
        .setup(move |app| {
            let _window = WebviewWindowBuilder::new(app, "main", tauri::WebviewUrl::External(TARGET_URL.parse().unwrap()))
                .title("WebWrapper")
                .inner_size(1200.0, 800.0)
                .resizable(true)
                .decorations(true)
                .on_navigation(|url| {
                    println!("Navigation request: {}", url);
                    true
                })

                .initialization_script(script_injector)
                .build()?;
            
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
