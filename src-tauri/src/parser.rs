use crate::models::ScriptModel;
use regex::Regex;
use std::collections::HashMap;

/// 解析油猴脚本元数据块与代码体。
///
/// 支持标准 `// ==UserScript== ... // ==/UserScript==` 格式，并兼容 Windows CRLF。
/// 对于无元数据块的脚本，将整段代码保留并回退到默认配置。
/// 同时兼容 `@include` / `@exclude` / `@require` / `@grant` / `@run-at` 等常用指令。
pub fn parse_script(raw_code: &str) -> Result<ScriptModel, String> {
    let meta_regex = Regex::new(r"(?s)//\s*==UserScript==\s*(.*?)\s*//\s*==/UserScript==")
        .map_err(|e| format!("元数据正则编译失败: {}", e))?;

    let (meta_map, code) = if let Some(caps) = meta_regex.captures(raw_code) {
        let meta_block = caps.get(1).map(|m| m.as_str()).unwrap_or("");
        let code = raw_code[caps.get(0).unwrap().end()..].trim().to_string();
        (parse_meta_block(meta_block), code)
    } else {
        (HashMap::new(), raw_code.trim().to_string())
    };

    if code.is_empty() {
        return Err("脚本代码体为空".to_string());
    }

    let mut script = ScriptModel {
        code,
        ..Default::default()
    };

    if let Some(name) = meta_map.get("name").and_then(|v| v.first()) {
        script.name = name.clone();
    }

    if let Some(namespace) = meta_map.get("namespace").and_then(|v| v.first()) {
        script.namespace = namespace.clone();
    }

    if let Some(version) = meta_map.get("version").and_then(|v| v.first()) {
        script.version = version.clone();
    }

    if let Some(homepage) = meta_map
        .get("homepage")
        .or_else(|| meta_map.get("homepageURL"))
        .and_then(|v| v.first())
    {
        script.homepage = homepage.clone();
    }

    if let Some(icon) = meta_map.get("icon").and_then(|v| v.first()) {
        script.icon = icon.clone();
    }

    if let Some(update_url) = meta_map.get("updateURL").and_then(|v| v.first()) {
        script.update_url = update_url.clone();
    }

    if let Some(matches) = meta_map.get("match") {
        script.matches = matches.clone();
    }

    if let Some(includes) = meta_map.get("include") {
        script.includes = includes.clone();
    }

    if let Some(excludes) = meta_map.get("exclude") {
        script.excludes = excludes.clone();
    }

    if let Some(run_at) = meta_map.get("run-at").and_then(|v| v.first()) {
        script.run_at = normalize_run_at(run_at);
    }

    if let Some(requires) = meta_map.get("require") {
        script.requires = requires.clone();
    }

    if let Some(grants) = meta_map.get("grant") {
        script.grants = grants.iter().map(|g| g.trim().to_string()).collect();
    }

    Ok(script)
}

fn parse_meta_block(block: &str) -> HashMap<String, Vec<String>> {
    let mut map: HashMap<String, Vec<String>> = HashMap::new();
    let entry_regex = Regex::new(r"//\s*@([\w-]+)\s+(.+)").unwrap();

    for line in block.lines() {
        if let Some(caps) = entry_regex.captures(line) {
            let key = caps[1].to_string();
            let value = caps[2].trim().to_string();
            map.entry(key).or_default().push(value);
        }
    }

    map
}

fn normalize_run_at(run_at: &str) -> String {
    match run_at.trim() {
        "document-start" => "document-start".to_string(),
        "document-idle" => "document-idle".to_string(),
        "document-end" | "document-body" => "document-end".to_string(),
        _ => "document-end".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_basic_metadata() {
        let code = r#"// ==UserScript==
// @name         Test Script
// @version      1.2.3
// @match        https://example.com/*
// @grant        GM_getValue
// @run-at       document-start
// ==/UserScript==
console.log("hello");
"#;
        let script = parse_script(code).unwrap();
        assert_eq!(script.name, "Test Script");
        assert_eq!(script.version, "1.2.3");
        assert_eq!(script.matches, vec!["https://example.com/*"]);
        assert_eq!(script.grants, vec!["GM_getValue"]);
        assert_eq!(script.run_at, "document-start");
        assert!(script.code.contains("console.log"));
    }

    #[test]
    fn test_parse_multiple_grants() {
        let code = r#"// ==UserScript==
// @grant GM_getValue
// @grant GM_setValue
// ==/UserScript==
console.log('ok');
"#;
        let script = parse_script(code).unwrap();
        assert_eq!(script.grants, vec!["GM_getValue", "GM_setValue"]);
    }

    #[test]
    fn test_no_metadata_defaults() {
        let code = "console.log('no metadata');";
        let script = parse_script(code).unwrap();
        assert_eq!(script.name, "未命名脚本");
        assert_eq!(script.run_at, "document-end");
        assert!(script.matches.is_empty());
    }

    #[test]
    fn test_empty_code_error() {
        let code = "// ==UserScript==\n// @name Empty\n// ==/UserScript==";
        assert!(parse_script(code).is_err());
    }
}
