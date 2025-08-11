#![cfg(feature = "extended-tests")]

use anyhow::Result;
use std::collections::HashMap;
use std::fs;
use tempfile::TempDir;
use tools::web_ops::{WebFetch, WebSearch};
use tools::{Tool, ToolInput};

#[tokio::test]
async fn web_fetch_file_scheme_reads_content() -> Result<()> {
    let tmp = TempDir::new().unwrap();
    let file_path = tmp.path().join("sample.txt");
    fs::write(&file_path, b"hello world")?;
    let url = format!("file://{}", file_path.display());

    let fetch = WebFetch::new();
    let input = ToolInput {
        command: "web_fetch".into(),
        args: HashMap::from([("url".into(), url)]),
        context: None,
        dry_run: false,
        timeout_ms: Some(5_000),
    };
    let out = fetch.execute(input).await?;
    assert!(out.success);
    assert!(out
        .formatted_output
        .as_ref()
        .unwrap()
        .contains("hello world"));
    assert_eq!(out.metadata.get("source").map(|s| s.as_str()), Some("file"));
    Ok(())
}

#[tokio::test]
async fn web_fetch_data_url_decodes() -> Result<()> {
    // data:text/plain;base64,aGVsbG8=
    let url = "data:text/plain;base64,aGVsbG8=".to_string();
    let fetch = WebFetch::new();
    let input = ToolInput {
        command: "web_fetch".into(),
        args: HashMap::from([("url".into(), url)]),
        context: None,
        dry_run: false,
        timeout_ms: Some(5_000),
    };
    let out = fetch.execute(input).await?;
    assert!(out.success);
    assert!(out.formatted_output.as_ref().unwrap().contains("hello"));
    assert_eq!(out.metadata.get("source").map(|s| s.as_str()), Some("data"));
    Ok(())
}

#[tokio::test]
async fn web_search_dry_run_reports_provider() -> Result<()> {
    // Default provider is mock
    std::env::remove_var("MAGRAY_SEARCH_PROVIDER");
    let search = WebSearch::new();
    let input = ToolInput {
        command: "web_search".into(),
        args: HashMap::from([("query".into(), "rust".into())]),
        context: None,
        dry_run: true,
        timeout_ms: None,
    };
    let out = search.execute(input).await?;
    assert!(out.success);
    assert_eq!(
        out.metadata.get("dry_run").map(|s| s.as_str()),
        Some("true")
    );
    assert!(out.metadata.get("provider").is_some());

    // Switch provider via env (still dry-run, no network)
    std::env::set_var("MAGRAY_SEARCH_PROVIDER", "duckduckgo");
    let input2 = ToolInput {
        command: "web_search".into(),
        args: HashMap::from([("query".into(), "rust".into())]),
        context: None,
        dry_run: true,
        timeout_ms: None,
    };
    let out2 = search.execute(input2).await?;
    assert!(out2
        .metadata
        .get("provider")
        .unwrap()
        .contains("DuckDuckGo"));
    Ok(())
}
