#![cfg(feature = "extended-tests")]
use anyhow::Result;
use std::collections::HashMap;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tools::web_ops::WebFetch;
use tools::{Tool, ToolInput};

#[tokio::test]
async fn web_fetch_http_truncates_and_sets_metadata() -> Result<()> {
    std::env::set_var("MAGRAY_NET_ALLOW", "localhost,127.0.0.1");
    // Start a local HTTP server
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let addr = listener.local_addr()?;

    // Prepare a large plain-text body (> 1MB)
    let body_len: usize = 1_200_000;
    let body = "B".repeat(body_len);
    let header = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body_len
    );

    // Serve one connection
    tokio::spawn(async move {
        if let Ok((mut stream, _)) = listener.accept().await {
            // Read request headers to '\r\n\r\n'
            let mut buf = vec![0u8; 8192];
            let mut total = 0usize;
            loop {
                match stream.read(&mut buf[total..]).await {
                    Ok(0) => break,
                    Ok(n) => {
                        total += n;
                        if total >= 4 {
                            if let Some(window) =
                                buf[..total].windows(4).find(|w| *w == b"\r\n\r\n")
                            {
                                break;
                            }
                            if total >= buf.len() {
                                break;
                            }
                        }
                    }
                    Err(_) => break,
                }
            }
            let _ = stream.write_all(header.as_bytes()).await;
            let _ = stream.write_all(body.as_bytes()).await;
            let _ = stream.flush().await;
            let _ = stream.shutdown().await;
        }
    });

    // Invoke WebFetch against local server
    let tool = WebFetch::new();
    let url = format!("http://{}:{}/", addr.ip(), addr.port());
    let mut args = HashMap::new();
    args.insert("url".to_string(), url.clone());
    let out = tool
        .execute(ToolInput {
            command: "web_fetch".into(),
            args,
            context: None,
            dry_run: false,
            timeout_ms: Some(5000),
        })
        .await?;

    assert!(out.success, "request should succeed");
    assert_eq!(out.metadata.get("status").map(String::as_str), Some("200"));
    assert!(out
        .metadata
        .get("content_type")
        .unwrap_or(&"".to_string())
        .contains("text/plain"));
    assert_eq!(out.metadata.get("source").map(String::as_str), Some("http"));
    // bytes should be capped at 1_048_576 (1MB)
    assert_eq!(
        out.metadata
            .get("bytes")
            .and_then(|s| s.parse::<usize>().ok()),
        Some(1_048_576)
    );
    let formatted = out
        .formatted_output
        .as_ref()
        .expect("formatted_output exists");
    assert!(formatted.len() >= 100_000);
    assert!(formatted.ends_with("[truncated]"));

    Ok(())
}
