use crate::{Tool, ToolInput, ToolOutput, ToolSpec};
use anyhow::{anyhow, Result};
use std::collections::HashMap;

pub struct WebSearch;

impl WebSearch {
    pub fn new() -> Self {
        WebSearch
    }
}

impl Default for WebSearch {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy)]
enum WebSearchProviderKind {
    Mock,
    DuckDuckGo,
}

fn select_search_provider_from_env() -> WebSearchProviderKind {
    match std::env::var("MAGRAY_SEARCH_PROVIDER").unwrap_or_default().to_lowercase().as_str() {
        "duckduckgo" | "ddg" => WebSearchProviderKind::DuckDuckGo,
        _ => WebSearchProviderKind::Mock,
    }
}

#[async_trait::async_trait]
impl Tool for WebSearch {
    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: "web_search".to_string(),
            description: "Поиск информации в интернете".to_string(),
            usage: "web_search <запрос>".to_string(),
            examples: vec![
                "web_search \"Rust программирование\"".to_string(),
                "найди информацию о машинном обучении".to_string(),
            ],
            input_schema: r#"{"query": "string"}"#.to_string(),
            usage_guide: None,
        }
    }

    async fn execute(&self, input: ToolInput) -> Result<ToolOutput> {
        let query = input
            .args
            .get("query")
            .ok_or_else(|| anyhow!("Отсутствует параметр 'query'"))?
            .to_string();

        if input.dry_run {
            let mut meta = HashMap::new();
            meta.insert("dry_run".into(), "true".into());
            meta.insert("provider".into(), format!("{:?}", select_search_provider_from_env()));
            return Ok(ToolOutput {
                success: true,
                result: format!("[dry-run] search: {}", query),
                formatted_output: None,
                metadata: meta,
            });
        }

        let provider = select_search_provider_from_env();
        match provider {
            WebSearchProviderKind::Mock => {
                let result = format!(
                    "🔍 Поиск: '{}'\n\n[Результаты]\n1. {} — Результат 1\n2. {} — Результат 2\n3. {} — Результат 3",
                    query, query, query, query
                );
                Ok(ToolOutput { success: true, result, formatted_output: None, metadata: HashMap::new() })
            }
            WebSearchProviderKind::DuckDuckGo => {
                // Best-effort: HTML endpoint; in offline/CI this may fail, but policy/tests use mock provider by default
                let url = format!("https://duckduckgo.com/html/?q={}", urlencoding::encode(&query));
                let client = reqwest::Client::builder()
                    .user_agent("MagrayBot/0.1 (+https://example.local)")
                    .timeout(std::time::Duration::from_secs(input.timeout_ms.unwrap_or(10_000) as u64 / 1000 + 10))
                    .build()?;
                let resp = client.get(&url).send().await?;
                let status = resp.status();
                let body = resp.text().await.unwrap_or_default();
                // very simple link extraction
                let mut results: Vec<String> = Vec::new();
                for line in body.lines() {
                    if let Some(href_idx) = line.find("href=\"") {
                        let rest = &line[href_idx + 6..];
                        if let Some(end_idx) = rest.find('\"') {
                            let href = &rest[..end_idx];
                            if href.starts_with("http") && !href.contains("duckduckgo.com") {
                                results.push(href.to_string());
                            }
                        }
                    }
                    if results.len() >= 5 { break; }
                }
                if results.is_empty() {
                    return Ok(ToolOutput { success: status.is_success(), result: format!("🔍 Поиск: '{}'\n\n(нет результатов)", query), formatted_output: None, metadata: HashMap::new() });
                }
                let mut out = format!("🔍 Поиск: '{}'\n\n", query);
                for (i, r) in results.iter().enumerate() { out.push_str(&format!("{}. {}\n", i+1, r)); }
                Ok(ToolOutput { success: true, result: out, formatted_output: None, metadata: HashMap::new() })
            }
        }
    }

    async fn parse_natural_language(&self, query: &str) -> Result<ToolInput> {
        let mut args = HashMap::new();
        let query_clean = query
            .replace("найди информацию о", "")
            .replace("найти ", "")
            .replace("поиск ", "")
            .trim()
            .to_string();
        args.insert("query".to_string(), query_clean);
        Ok(ToolInput { command: "web_search".to_string(), args, context: Some(query.to_string()), dry_run: false, timeout_ms: None })
    }
}

pub struct WebFetch;

impl WebFetch {
    pub fn new() -> Self { WebFetch }
}

impl Default for WebFetch { fn default() -> Self { Self::new() } }

fn is_textual_content(content_type: Option<&str>) -> bool {
    if let Some(ct) = content_type {
        let l = ct.to_lowercase();
        return l.starts_with("text/") || l.contains("application/json") || l.contains("application/xml") || l.contains("application/javascript");
    }
    true
}

async fn fetch_http_with_limit(url: &str, timeout_ms: Option<u64>, max_bytes: usize) -> Result<(u16, String, String, usize)> {
    let client = reqwest::Client::builder()
        .user_agent("MagrayBot/0.1 (+https://example.local)")
        .timeout(std::time::Duration::from_millis(timeout_ms.unwrap_or(10_000)))
        .build()?;
    let resp = client.get(url).send().await?;
    let status = resp.status().as_u16();
    let content_type = resp.headers().get(reqwest::header::CONTENT_TYPE).and_then(|v| v.to_str().ok()).unwrap_or("").to_string();
    let mut collected: Vec<u8> = Vec::with_capacity(4096);
    let mut resp = resp;
    loop {
        match resp.chunk().await? {
            Some(bytes) => {
                if collected.len() + bytes.len() > max_bytes { collected.extend_from_slice(&bytes[..max_bytes.saturating_sub(collected.len())]); break; }
                collected.extend_from_slice(&bytes);
            }
            None => break,
        }
    }
    let bytes_len = collected.len();
    let body = if is_textual_content(Some(&content_type)) { String::from_utf8_lossy(&collected).to_string() } else { format!("[binary content: {} bytes]", bytes_len) };
    Ok((status, content_type, body, bytes_len))
}

fn decode_data_url(url: &str) -> Result<(String, usize)> {
    // data:[<mediatype>][;base64],<data>
    let rest = &url[5..];
    let mut parts = rest.splitn(2, ',');
    let meta = parts.next().ok_or_else(|| anyhow!("bad data url"))?;
    let data = parts.next().ok_or_else(|| anyhow!("bad data url"))?;
    let is_base64 = meta.ends_with(";base64");
    let bytes = if is_base64 { base64::decode(data)? } else { urlencoding::decode(data)?.into_owned().into_bytes() };
    let text = String::from_utf8_lossy(&bytes).to_string();
    Ok((text, bytes.len()))
}

#[async_trait::async_trait]
impl Tool for WebFetch {
    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: "web_fetch".to_string(),
            description: "Загружает содержимое веб-страницы".to_string(),
            usage: "web_fetch <url>".to_string(),
            examples: vec![
                "web_fetch https://example.com".to_string(),
                "загрузи страницу rust-lang.org".to_string(),
            ],
            input_schema: r#"{"url": "string"}"#.to_string(),
            usage_guide: None,
        }
    }

    async fn execute(&self, input: ToolInput) -> Result<ToolOutput> {
        let url = input
            .args
            .get("url")
            .ok_or_else(|| anyhow!("Отсутствует параметр 'url'"))?
            .to_string();

        if input.dry_run {
            let mut meta = HashMap::new();
            meta.insert("dry_run".into(), "true".into());
            return Ok(ToolOutput { success: true, result: format!("[dry-run] GET {}", url), formatted_output: None, metadata: meta });
        }

        // Support schemes: http(s), file, data
        let max_bytes: usize = 1_048_576; // 1 MB cap
        let timeout_ms = input.timeout_ms;
        let (success, result, formatted_output, mut metadata): (bool, String, Option<String>, HashMap<String, String>);
        if url.starts_with("http://") || url.starts_with("https://") {
            match fetch_http_with_limit(&url, timeout_ms, max_bytes).await {
                Ok((status, content_type, body, bytes)) => {
                    let truncated = if body.len() > 100_000 { format!("{}\n... [truncated]", &body[..100_000]) } else { body };
                    let mut meta = HashMap::new();
                    meta.insert("status".into(), status.to_string());
                    meta.insert("content_type".into(), content_type);
                    meta.insert("bytes".into(), bytes.to_string());
                    meta.insert("source".into(), "http".into());
                    (success, result, formatted_output, metadata) = (status < 400, format!("📄 GET {} -> {} ({} bytes)", url, status, bytes), Some(truncated), meta);
                }
                Err(e) => {
                    let mut meta = HashMap::new();
                    meta.insert("error".into(), e.to_string());
                    meta.insert("source".into(), "http".into());
                    (success, result, formatted_output, metadata) = (false, format!("✗ HTTP error: {}", e), None, meta);
                }
            }
        } else if url.starts_with("file://") {
            let path = &url[7..];
            match tokio::fs::read(path).await {
                Ok(bytes) => {
                    let bytes_len = bytes.len();
                    let body = String::from_utf8_lossy(&bytes).to_string();
                    let mut meta = HashMap::new();
                    meta.insert("bytes".into(), bytes_len.to_string());
                    meta.insert("source".into(), "file".into());
                    (success, result, formatted_output, metadata) = (true, format!("📄 FILE {} ({} bytes)", path, bytes_len), Some(body), meta);
                }
                Err(e) => {
                    let mut meta = HashMap::new();
                    meta.insert("error".into(), e.to_string());
                    meta.insert("source".into(), "file".into());
                    (success, result, formatted_output, metadata) = (false, format!("✗ FILE error: {}", e), None, meta);
                }
            }
        } else if url.starts_with("data:") {
            match decode_data_url(&url) {
                Ok((text, bytes)) => {
                    let mut meta = HashMap::new();
                    meta.insert("bytes".into(), bytes.to_string());
                    meta.insert("source".into(), "data".into());
                    (success, result, formatted_output, metadata) = (true, format!("📄 DATA ({} bytes)", bytes), Some(text), meta);
                }
                Err(e) => {
                    let mut meta = HashMap::new();
                    meta.insert("error".into(), e.to_string());
                    meta.insert("source".into(), "data".into());
                    (success, result, formatted_output, metadata) = (false, format!("✗ DATA error: {}", e), None, meta);
                }
            }
        } else {
            return Err(anyhow!("Неподдерживаемая схема URL. Используйте http(s)://, file:// или data:"));
        }

        Ok(ToolOutput { success, result, formatted_output, metadata })
    }

    async fn parse_natural_language(&self, query: &str) -> Result<ToolInput> {
        let mut args = HashMap::new();
        // Извлекаем URL из запроса
        let words: Vec<&str> = query.split_whitespace().collect();
        for word in words {
            if word.starts_with("http://")
                || word.starts_with("https://")
                || word.starts_with("file://")
                || word.starts_with("data:")
                || word.contains('.')
            {
                args.insert("url".to_string(), word.to_string());
                break;
            }
        }

        // Если URL не найден, используем весь запрос
        if !args.contains_key("url") {
            args.insert("url".to_string(), query.to_string());
        }

        Ok(ToolInput { command: "web_fetch".to_string(), args, context: Some(query.to_string()), dry_run: false, timeout_ms: None })
    }
}
