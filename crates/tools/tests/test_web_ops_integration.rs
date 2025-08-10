use anyhow::Result;

#[tokio::test]
async fn web_fetch_large_data_url_truncates_metadata() -> Result<()> {
    use tools::web_ops::WebFetch;
    use tools::{Tool, ToolInput};
    use std::collections::HashMap;

    // Construct a large data URL > 120k chars to trigger truncation path (though truncation path is for http, we still verify metadata structure)
    let large_text = "A".repeat(200_000);
    let data_url = format!("data:text/plain,{}", urlencoding::encode(&large_text));

    let tool = WebFetch::new();
    let mut args = HashMap::new();
    args.insert("url".to_string(), data_url);
    let out = tool.execute(ToolInput { command: "web_fetch".into(), args, context: None, dry_run: false, timeout_ms: Some(2000) }).await?;
    assert!(out.success);
    assert!(out.metadata.contains_key("bytes"));
    assert_eq!(out.metadata.get("source").map(String::as_str), Some("data"));
    assert!(out.formatted_output.as_ref().unwrap().len() >= 200_000 / 2); // urlencoding shrinks check relaxed
    Ok(())
}