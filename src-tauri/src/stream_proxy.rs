use serde_json::Value;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tiny_http::{Header, Response, Server};

/// 本地流代理服务器状态。
pub struct StreamProxyServer {
    port: u16,
    _handle: std::thread::JoinHandle<()>,
}

impl StreamProxyServer {
    pub fn start() -> Result<Self, String> {
        let server = Server::http("127.0.0.1:0").map_err(|e| format!("启动代理服务器失败: {}", e))?;
        let port = server
            .server_addr()
            .to_ip()
            .map(|addr| addr.port())
            .ok_or("无法获取代理端口")?;

        let handle = std::thread::spawn(move || {
            let client = reqwest::blocking::Client::builder()
                .timeout(Duration::from_secs(30))
                .build()
                .expect("创建 HTTP 客户端失败");

            for request in server.incoming_requests() {
                let response = handle_proxy_request(&client, &request);
                let _ = request.respond(response);
            }
        });

        Ok(StreamProxyServer {
            port,
            _handle: handle,
        })
    }

    pub fn base_url(&self) -> String {
        format!("http://127.0.0.1:{}", self.port)
    }
}

fn handle_proxy_request(
    client: &reqwest::blocking::Client,
    request: &tiny_http::Request,
) -> Response<std::io::Cursor<Vec<u8>>> {
    let url = match request.url().split('?').nth(1).and_then(|q| {
        q.split('&')
            .find(|pair| pair.starts_with("url="))
            .map(|pair| &pair[4..])
    }) {
        Some(encoded) => match urlencoding::decode(encoded) {
            Ok(decoded) => decoded.to_string(),
            Err(_) => return error_response("无法解码 URL 参数", 400),
        },
        None => return error_response("缺少 url 参数", 400),
    };

    let headers_json = request.url().split('?').nth(1).and_then(|q| {
        q.split('&')
            .find(|pair| pair.starts_with("headers="))
            .map(|pair| &pair[10..])
    });

    let mut headers = reqwest::header::HeaderMap::new();

    if let Some(encoded) = headers_json {
        match urlencoding::decode(encoded) {
            Ok(decoded) => {
                if let Ok(Value::Object(map)) = serde_json::from_str::<Value>(&decoded) {
                    for (key, value) in map {
                        if let Some(val) = value.as_str() {
                            if let Ok(name) = reqwest::header::HeaderName::from_bytes(key.as_bytes())
                            {
                                if let Ok(v) = reqwest::header::HeaderValue::from_str(val) {
                                    headers.insert(name, v);
                                }
                            }
                        }
                    }
                }
            }
            Err(_) => return error_response("无法解码 headers 参数", 400),
        }
    }

    let response = match client.get(&url).headers(headers).send() {
        Ok(res) => res,
        Err(e) => return error_response(&format!("代理请求失败: {}", e), 502),
    };

    let status = response.status();
    let mut tiny_response = Response::new(
        status.as_u16().into(),
        response
            .headers()
            .iter()
            .filter_map(|(k, v)| {
                let name = k.as_str();
                let value = v.to_str().ok()?;
                Some(Header::from_bytes(name.as_bytes(), value.as_bytes()).ok()?)
            })
            .collect(),
        std::io::Cursor::new(response.bytes().unwrap_or_default().to_vec()),
        None,
        None,
    );

    // 注入 CORS 头，允许前端页面访问。
    tiny_response.add_header(
        Header::from_bytes(
            "Access-Control-Allow-Origin",
            "*".as_bytes(),
        )
        .unwrap(),
    );

    tiny_response
}

fn error_response(message: &str, status: u16) -> Response<std::io::Cursor<Vec<u8>>> {
    Response::new(
        status.into(),
        vec![],
        std::io::Cursor::new(message.as_bytes().to_vec()),
        None,
        None,
    )
}

/// 全局代理服务器句柄，按需启动。
static PROXY_SERVER: Mutex<Option<Arc<StreamProxyServer>>> = Mutex::new(None);

/// 获取或启动代理服务器，返回基础 URL。
pub fn ensure_proxy_server() -> Result<String, String> {
    let mut server = PROXY_SERVER.lock().map_err(|e| e.to_string())?;
    if server.is_none() {
        *server = Some(Arc::new(StreamProxyServer::start()?));
    }
    Ok(server.as_ref().unwrap().base_url())
}

/// 生成代理播放 URL。
pub fn build_proxy_url(base: &str, original_url: &str, headers: &HashMap<String, String>) -> String {
    let encoded_url = urlencoding::encode(original_url);
    let headers_json = serde_json::to_string(headers).unwrap_or_default();
    let encoded_headers = urlencoding::encode(&headers_json);
    format!("{}/proxy?url={}&headers={}", base, encoded_url, encoded_headers)
}

#[tauri::command]
pub fn proxy_stream_url(
    url: String,
    headers: Option<HashMap<String, String>>,
) -> Result<String, String> {
    let base = ensure_proxy_server()?;
    Ok(build_proxy_url(&base, &url, &headers.unwrap_or_default()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_proxy_url() {
        let base = "http://127.0.0.1:1234";
        let url = "http://example.com/stream.m3u8";
        let mut headers = HashMap::new();
        headers.insert("Referer".to_string(), "https://tingfm.net/".to_string());
        let proxy_url = build_proxy_url(base, url, &headers);
        assert!(proxy_url.starts_with("http://127.0.0.1:1234/proxy?url="));
        assert!(proxy_url.contains("headers="));
    }

    #[test]
    fn test_ensure_proxy_server_starts() {
        let base = ensure_proxy_server().expect("应能启动代理服务器");
        assert!(base.starts_with("http://127.0.0.1:"));
    }
}
