use regex::Regex;

/// 判断给定 URL 是否匹配任一模式。
///
/// 模式规则：
/// - 以 `regex:` 开头的模式被视为正则表达式。
/// - 其他模式被视为 Tampermonkey 风格的 glob（`*` 匹配任意字符，`?` 匹配单个字符）。
/// - 空模式列表返回 `false`，表示不匹配。
pub fn is_url_matched(url: &str, patterns: &[String]) -> bool {
    if patterns.is_empty() {
        return false;
    }

    for pattern in patterns {
        if pattern.starts_with("regex:") {
            let regex_str = &pattern[6..];
            if Regex::new(regex_str).map(|r| r.is_match(url)).unwrap_or(false) {
                return true;
            }
        } else if pattern_to_regex(pattern).is_match(url) {
            return true;
        }
    }

    false
}

fn pattern_to_regex(pattern: &str) -> Regex {
    let mut regex_str = String::from("^");

    for ch in pattern.chars() {
        match ch {
            '*' => regex_str.push_str(".*"),
            '?' => regex_str.push('.'),
            '.' => regex_str.push_str("\\."),
            '+' | '(' | ')' | '[' | ']' | '{' | '}' | '^' | '$' | '\\' | '|' => {
                regex_str.push('\\');
                regex_str.push(ch);
            }
            _ => regex_str.push(ch),
        }
    }

    regex_str.push('$');
    Regex::new(&regex_str).unwrap_or_else(|_| Regex::new(".*").unwrap())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_glob_match() {
        assert!(is_url_matched("https://example.com/path", &["https://example.com/*".to_string()]));
        assert!(!is_url_matched("https://other.com/", &["https://example.com/*".to_string()]));
    }

    #[test]
    fn test_regex_match() {
        assert!(is_url_matched("https://example.com/abc", &["regex:https://example\\.com/.*".to_string()]));
        assert!(!is_url_matched("https://other.com/", &["regex:https://example\\.com/.*".to_string()]));
    }

    #[test]
    fn test_empty_patterns() {
        assert!(!is_url_matched("https://example.com/", &[]));
    }
}
