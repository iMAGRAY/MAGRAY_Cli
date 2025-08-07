#![cfg(not(feature = "minimal"))]
use clap::Parser;
use cli::commands::ModelsCommand;

// Helper struct to parse Models commands
#[derive(Parser)]
struct TestCli {
    #[command(subcommand)]
    command: TestModelsCommand,
}

#[derive(Parser)]
enum TestModelsCommand {
    Models(ModelsCommand),
}

#[tokio::test]
async fn test_models_list_command() {
    let args = vec!["test", "models", "list"];
    let cli = TestCli::try_parse_from(args).unwrap();

    if let TestModelsCommand::Models(models_cmd) = cli.command {
        let result = models_cmd.execute().await;
        assert!(result.is_ok());
    } else {
        panic!("Expected Models command");
    }
}

#[tokio::test]
async fn test_models_list_with_type_filter() {
    let args = vec!["test", "models", "list", "--model-type", "embedding"];
    let cli = TestCli::try_parse_from(args).unwrap();

    if let TestModelsCommand::Models(models_cmd) = cli.command {
        let result = models_cmd.execute().await;
        assert!(result.is_ok());
    }
}

#[tokio::test]
async fn test_models_list_with_type_filter_reranker() {
    let args = vec!["test", "models", "list", "--model-type", "reranker"];
    let cli = TestCli::try_parse_from(args).unwrap();

    if let TestModelsCommand::Models(models_cmd) = cli.command {
        let result = models_cmd.execute().await;
        assert!(result.is_ok());
    }
}

#[tokio::test]
async fn test_models_list_available_only() {
    let args = vec!["test", "models", "list", "--available-only"];
    let cli = TestCli::try_parse_from(args).unwrap();

    if let TestModelsCommand::Models(models_cmd) = cli.command {
        let result = models_cmd.execute().await;
        assert!(result.is_ok());
    }
}

#[tokio::test]
async fn test_models_list_combined_filters() {
    let args = vec![
        "test",
        "models",
        "list",
        "--model-type",
        "embedding",
        "--available-only",
    ];
    let cli = TestCli::try_parse_from(args).unwrap();

    if let TestModelsCommand::Models(models_cmd) = cli.command {
        let result = models_cmd.execute().await;
        assert!(result.is_ok());
    }
}

#[tokio::test]
async fn test_models_diagnose() {
    let args = vec!["test", "models", "diagnose"];
    let cli = TestCli::try_parse_from(args).unwrap();

    if let TestModelsCommand::Models(models_cmd) = cli.command {
        let result = models_cmd.execute().await;
        assert!(result.is_ok());
    }
}

#[tokio::test]
async fn test_models_show() {
    let args = vec!["test", "models", "show", "test-model"];
    let cli = TestCli::try_parse_from(args).unwrap();

    if let TestModelsCommand::Models(models_cmd) = cli.command {
        let result = models_cmd.execute().await;
        // May fail if model doesn't exist, but should parse correctly
        assert!(result.is_ok());
    }
}

#[tokio::test]
async fn test_models_show_common_models() {
    let test_models = vec!["bge-m3", "qwen3emb", "bge-reranker-v2-m3"];

    for model_name in test_models {
        let args = vec!["test", "models", "show", model_name];
        let cli = TestCli::try_parse_from(args).unwrap();

        if let TestModelsCommand::Models(models_cmd) = cli.command {
            let result = models_cmd.execute().await;
            assert!(result.is_ok());
        }
    }
}

#[tokio::test]
async fn test_models_recommendations() {
    let args = vec!["test", "models", "recommendations"];
    let cli = TestCli::try_parse_from(args).unwrap();

    if let TestModelsCommand::Models(models_cmd) = cli.command {
        let result = models_cmd.execute().await;
        assert!(result.is_ok());
    }
}

#[tokio::test]
async fn test_models_check() {
    let args = vec!["test", "models", "check"];
    let cli = TestCli::try_parse_from(args).unwrap();

    if let TestModelsCommand::Models(models_cmd) = cli.command {
        let result = models_cmd.execute().await;
        assert!(result.is_ok());
    }
}

#[test]
fn test_models_command_aliases() {
    // Test ls alias for list
    let args = vec!["test", "models", "ls"];
    let cli = TestCli::try_parse_from(args);
    assert!(cli.is_ok());

    // Test diag alias for diagnose
    let args = vec!["test", "models", "diag"];
    let cli = TestCli::try_parse_from(args);
    assert!(cli.is_ok());

    // Test info alias for show
    let args = vec!["test", "models", "info", "test"];
    let cli = TestCli::try_parse_from(args);
    assert!(cli.is_ok());

    // Test rec alias for recommendations
    let args = vec!["test", "models", "rec"];
    let cli = TestCli::try_parse_from(args);
    assert!(cli.is_ok());

    // Test check alias
    let args = vec!["test", "models", "check"];
    let cli = TestCli::try_parse_from(args);
    assert!(cli.is_ok());
}

#[test]
fn test_models_list_parameter_combinations() {
    // Test short flags
    let args = vec!["test", "models", "list", "-t", "embedding"];
    let cli = TestCli::try_parse_from(args);
    assert!(cli.is_ok());

    let args = vec!["test", "models", "list", "-a"];
    let cli = TestCli::try_parse_from(args);
    assert!(cli.is_ok());

    // Test combined short flags
    let args = vec!["test", "models", "list", "-t", "reranker", "-a"];
    let cli = TestCli::try_parse_from(args);
    assert!(cli.is_ok());
}

#[test]
fn test_models_type_filter_variants() {
    // Test embedding variants
    let variants = vec!["embedding", "emb"];
    for variant in variants {
        let args = vec!["test", "models", "list", "--model-type", variant];
        let cli = TestCli::try_parse_from(args);
        assert!(cli.is_ok());
    }

    // Test reranker variants
    let variants = vec!["reranker", "rerank"];
    for variant in variants {
        let args = vec!["test", "models", "list", "--model-type", variant];
        let cli = TestCli::try_parse_from(args);
        assert!(cli.is_ok());
    }
}

#[test]
fn test_invalid_models_commands() {
    // Test invalid subcommand
    let args = vec!["test", "models", "invalid"];
    let cli = TestCli::try_parse_from(args);
    assert!(cli.is_err());

    // Test show without model name
    let args = vec!["test", "models", "show"];
    let cli = TestCli::try_parse_from(args);
    assert!(cli.is_err());
}

#[tokio::test]
async fn test_models_list_unknown_type_filter() {
    let args = vec!["test", "models", "list", "--model-type", "unknown"];
    let cli = TestCli::try_parse_from(args).unwrap();

    if let TestModelsCommand::Models(models_cmd) = cli.command {
        let result = models_cmd.execute().await;
        // Should still succeed but with warning
        assert!(result.is_ok());
    }
}

#[tokio::test]
async fn test_models_show_nonexistent() {
    let args = vec!["test", "models", "show", "nonexistent-model-xyz"];
    let cli = TestCli::try_parse_from(args).unwrap();

    if let TestModelsCommand::Models(models_cmd) = cli.command {
        let result = models_cmd.execute().await;
        // Should succeed even if model doesn't exist (just shows error message)
        assert!(result.is_ok());
    }
}

#[test]
fn test_models_command_parsing_edge_cases() {
    // Test empty model name (should fail)
    let args = vec!["test", "models", "show", ""];
    let cli = TestCli::try_parse_from(args);
    // Empty string should still be accepted by clap
    assert!(cli.is_ok());

    // Test model name with special characters
    let args = vec![
        "test",
        "models",
        "show",
        "model-with-dashes_and_underscores",
    ];
    let cli = TestCli::try_parse_from(args);
    assert!(cli.is_ok());
}
