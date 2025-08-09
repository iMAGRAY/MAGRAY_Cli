#![cfg(feature = "extended-tests")]

use anyhow::Result;
use tools::web_ops::WebFetch;
use tools::{Tool, ToolInput};
use std::collections::HashMap;

#[tokio::test]
async fn web_fetch_file_not_found_reports_error() -> Result<()> {
    let url = "file:///definitely/not/exist".to_string();
    let fetch = WebFetch::new();
    let out = fetch.execute(ToolInput { command: "web_fetch".into(), args: HashMap::from([("url".into(), url)]), context: None, dry_run: false, timeout_ms: Some(2000) }).await?;
    assert!(!out.success);
    assert!(out.result.contains("FILE error"));
    assert_eq!(out.metadata.get("source").map(|s| s.as_str()), Some("file"));
    Ok(())
}

#[tokio::test]
async fn web_fetch_malformed_data_url_reports_error() -> Result<()> {
    let url = "data:;base64,%%%".to_string();
    let fetch = WebFetch::new();
    let out = fetch.execute(ToolInput { command: "web_fetch".into(), args: HashMap::from([("url".into(), url)]), context: None, dry_run: false, timeout_ms: Some(2000) }).await?;
    assert!(!out.success);
    assert!(out.result.contains("DATA error"));
    assert_eq!(out.metadata.get("source").map(|s| s.as_str()), Some("data"));
    Ok(())
}