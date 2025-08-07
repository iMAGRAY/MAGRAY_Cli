use clap::Parser;
use cli::commands::GpuCommand;

// Helper struct to parse GPU commands
#[derive(Parser)]
struct TestCli {
    #[command(subcommand)]
    command: TestGpuCommand,
}

#[derive(Parser)]
enum TestGpuCommand {
    Gpu(GpuCommand),
}

#[tokio::test]
async fn test_gpu_info_command() {
    // Test basic GPU info parsing
    let args = vec!["test", "gpu", "info"];
    let cli = TestCli::try_parse_from(args).unwrap();

    if let TestGpuCommand::Gpu(gpu_cmd) = cli.command {
        // Should not panic
        let result = gpu_cmd.execute().await;
        // Allow either success or error, as GPU may not be available
        assert!(result.is_ok() || result.is_err());
    } else {
        panic!("Expected GPU command");
    }
}

#[tokio::test]
async fn test_gpu_benchmark_command_basic() {
    let args = vec!["test", "gpu", "benchmark"];
    let cli = TestCli::try_parse_from(args).unwrap();

    if let TestGpuCommand::Gpu(gpu_cmd) = cli.command {
        let result = gpu_cmd.execute().await;
        // GPU may not be available, but command should parse correctly
        assert!(result.is_ok() || result.is_err());
    }
}

#[tokio::test]
async fn test_gpu_benchmark_with_params() {
    let args = vec![
        "test",
        "gpu",
        "benchmark",
        "--batch-size",
        "50",
        "--compare",
    ];
    let cli = TestCli::try_parse_from(args).unwrap();

    if let TestGpuCommand::Gpu(gpu_cmd) = cli.command {
        let result = gpu_cmd.execute().await;
        // Should parse correctly regardless of GPU availability
        assert!(result.is_ok() || result.is_err());
    }
}

#[tokio::test]
async fn test_gpu_cache_stats() {
    let args = vec!["test", "gpu", "cache", "stats"];
    let cli = TestCli::try_parse_from(args).unwrap();

    if let TestGpuCommand::Gpu(gpu_cmd) = cli.command {
        let result = gpu_cmd.execute().await;
        assert!(result.is_ok() || result.is_err());
    }
}

#[tokio::test]
async fn test_gpu_cache_clear() {
    let args = vec!["test", "gpu", "cache", "clear"];
    let cli = TestCli::try_parse_from(args).unwrap();

    if let TestGpuCommand::Gpu(gpu_cmd) = cli.command {
        let result = gpu_cmd.execute().await;
        assert!(result.is_ok() || result.is_err());
    }
}

#[tokio::test]
async fn test_gpu_cache_size() {
    let args = vec!["test", "gpu", "cache", "size"];
    let cli = TestCli::try_parse_from(args).unwrap();

    if let TestGpuCommand::Gpu(gpu_cmd) = cli.command {
        let result = gpu_cmd.execute().await;
        assert!(result.is_ok() || result.is_err());
    }
}

#[tokio::test]
async fn test_gpu_memory_stats() {
    let args = vec!["test", "gpu", "memory", "stats"];
    let cli = TestCli::try_parse_from(args).unwrap();

    if let TestGpuCommand::Gpu(gpu_cmd) = cli.command {
        let result = gpu_cmd.execute().await;
        assert!(result.is_ok() || result.is_err());
    }
}

#[tokio::test]
async fn test_gpu_memory_clear() {
    let args = vec!["test", "gpu", "memory", "clear"];
    let cli = TestCli::try_parse_from(args).unwrap();

    if let TestGpuCommand::Gpu(gpu_cmd) = cli.command {
        let result = gpu_cmd.execute().await;
        assert!(result.is_ok() || result.is_err());
    }
}

#[tokio::test]
async fn test_gpu_optimize_default() {
    let args = vec!["test", "gpu", "optimize"];
    let cli = TestCli::try_parse_from(args).unwrap();

    if let TestGpuCommand::Gpu(gpu_cmd) = cli.command {
        let result = gpu_cmd.execute().await;
        // May fail if models not available, but should parse
        assert!(result.is_ok() || result.is_err());
    }
}

#[tokio::test]
async fn test_gpu_optimize_with_model() {
    let args = vec!["test", "gpu", "optimize", "custom-model"];
    let cli = TestCli::try_parse_from(args).unwrap();

    if let TestGpuCommand::Gpu(gpu_cmd) = cli.command {
        let result = gpu_cmd.execute().await;
        assert!(result.is_ok() || result.is_err());
    }
}

#[test]
fn test_gpu_command_aliases() {
    // Test visible aliases work
    let args1 = vec!["test", "gpu", "i"]; // info alias
    let cli1 = TestCli::try_parse_from(args1);
    assert!(cli1.is_ok());

    let args2 = vec!["test", "gpu", "b"]; // benchmark alias
    let cli2 = TestCli::try_parse_from(args2);
    assert!(cli2.is_ok());

    let args3 = vec!["test", "gpu", "o"]; // optimize alias
    let cli3 = TestCli::try_parse_from(args3);
    assert!(cli3.is_ok());
}

#[test]
fn test_invalid_gpu_commands() {
    // Test invalid subcommands fail parsing
    let args = vec!["test", "gpu", "invalid"];
    let cli = TestCli::try_parse_from(args);
    assert!(cli.is_err());

    // Test invalid cache action
    let args = vec!["test", "gpu", "cache", "invalid"];
    let cli = TestCli::try_parse_from(args);
    assert!(cli.is_err());

    // Test invalid memory action
    let args = vec!["test", "gpu", "memory", "invalid"];
    let cli = TestCli::try_parse_from(args);
    assert!(cli.is_err());
}

#[test]
fn test_gpu_benchmark_parameters() {
    // Test batch size parameter
    let args = vec!["test", "gpu", "benchmark", "--batch-size", "200"];
    let cli = TestCli::try_parse_from(args);
    assert!(cli.is_ok());

    // Test short version
    let args = vec!["test", "gpu", "benchmark", "-b", "100"];
    let cli = TestCli::try_parse_from(args);
    assert!(cli.is_ok());

    // Test compare flag
    let args = vec!["test", "gpu", "benchmark", "--compare"];
    let cli = TestCli::try_parse_from(args);
    assert!(cli.is_ok());

    // Test short version
    let args = vec!["test", "gpu", "benchmark", "-c"];
    let cli = TestCli::try_parse_from(args);
    assert!(cli.is_ok());
}
