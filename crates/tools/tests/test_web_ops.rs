use tools::web_ops::WebSearch;
use tools::{Tool, ToolInput};
use std::collections::HashMap;
use anyhow::Result;

#[tokio::test]
async fn test_web_search_spec() {
    let web_search = WebSearch::new();
    let spec = web_search.spec();
    
    assert_eq!(spec.name, "web_search");
    assert!(spec.description.contains("Поиск"));
    assert!(spec.usage.contains("web_search"));
    assert!(!spec.examples.is_empty());
    assert!(spec.input_schema.contains("query"));
}

#[tokio::test]
async fn test_web_search_natural_language_parsing() -> Result<()> {
    let web_search = WebSearch::new();
    let input = web_search.parse_natural_language("найди информацию о Rust").await?;
    
    assert_eq!(input.command, "web_search");
    assert!(input.args.contains_key("query"));
    assert_eq!(input.args.get("query"), Some(&"найди информацию о Rust".to_string()));
    assert!(input.context.is_some());
    
    Ok(())
}

#[tokio::test]
async fn test_web_search_empty_query() -> Result<()> {
    let web_search = WebSearch::new();
    let input = ToolInput {
        command: "web_search".to_string(),
        args: HashMap::new(), // No query provided
        context: None,
    };
    
    let result = web_search.execute(input).await?;
    
    assert!(!result.success);
    assert!(result.result.contains("Пустой запрос"));
    assert!(result.formatted_output.is_none());
    
    Ok(())
}

#[tokio::test]
async fn test_web_search_empty_query_string() -> Result<()> {
    let web_search = WebSearch::new();
    let mut args = HashMap::new();
    args.insert("query".to_string(), "   ".to_string()); // Empty/whitespace query
    
    let input = ToolInput {
        command: "web_search".to_string(),
        args,
        context: None,
    };
    
    let result = web_search.execute(input).await?;
    
    assert!(!result.success);
    assert!(result.result.contains("Пустой запрос"));
    
    Ok(())
}

#[tokio::test]
async fn test_web_search_supports_natural_language() {
    let web_search = WebSearch::new();
    assert!(web_search.supports_natural_language());
}

#[tokio::test]
async fn test_web_search_with_valid_query() -> Result<()> {
    let web_search = WebSearch::new();
    let mut args = HashMap::new();
    args.insert("query".to_string(), "test".to_string());
    
    let input = ToolInput {
        command: "web_search".to_string(),
        args,
        context: None,
    };
    
    // This test may fail due to network issues, but shouldn't panic
    let result = web_search.execute(input).await;
    assert!(result.is_ok());
    
    let output = result.unwrap();
    // Should have some result content
    assert!(!output.result.is_empty());
    
    // If successful, should have formatted output
    if output.success {
        assert!(output.formatted_output.is_some());
    }
    
    Ok(())
}

#[tokio::test] 
async fn test_web_search_natural_language_contains_query() -> Result<()> {
    let web_search = WebSearch::new();
    let query = "search for Rust programming";
    let input = web_search.parse_natural_language(query).await?;
    
    assert_eq!(input.args.get("query"), Some(&query.to_string()));
    assert_eq!(input.context, Some(query.to_string()));
    
    Ok(())
}

#[tokio::test]
async fn test_web_search_spec_schema() {
    let web_search = WebSearch::new();
    let spec = web_search.spec();
    
    assert!(spec.input_schema.contains("query"));
    assert!(spec.input_schema.contains("string"));
}

#[tokio::test]
async fn test_web_search_handles_encoding() -> Result<()> {
    let web_search = WebSearch::new();
    let mut args = HashMap::new();
    args.insert("query".to_string(), "special chars: & + % #".to_string());
    
    let input = ToolInput {
        command: "web_search".to_string(),
        args,
        context: None,
    };
    
    // Should handle URL encoding without panicking
    let result = web_search.execute(input).await;
    assert!(result.is_ok());
    
    Ok(())
}

#[tokio::test]
async fn test_web_search_example_format() {
    let web_search = WebSearch::new();
    let spec = web_search.spec();
    
    // Check that examples follow expected format
    assert!(!spec.examples.is_empty());
    let example = &spec.examples[0];
    assert!(example.contains("web_search"));
    assert!(example.contains("'"));
}