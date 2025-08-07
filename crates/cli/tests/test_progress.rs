use cli::progress::{AdaptiveSpinner, MultiStageProgress, ProgressBuilder, ProgressType};
use std::time::Duration;
use tokio::time::sleep;

#[test]
fn test_progress_type_configs() {
    let fast_config = ProgressType::Fast.config();
    assert_eq!(fast_config.tick_interval, Duration::from_millis(80));
    assert_eq!(fast_config.color, "cyan");

    let medium_config = ProgressType::Medium.config();
    assert_eq!(medium_config.tick_interval, Duration::from_millis(120));
    assert_eq!(medium_config.color, "blue");

    let slow_config = ProgressType::Slow.config();
    assert_eq!(slow_config.tick_interval, Duration::from_millis(150));
    assert_eq!(slow_config.color, "yellow");
}

#[test]
fn test_adaptive_spinner_creation() {
    let spinner = ProgressType::Fast.create_spinner("Testing...");

    // Spinner should be created successfully
    // Can't test internal state easily, but ensure no panics
    spinner.set_message("Updated message");
    spinner.finish_success(Some("Test completed"));
}

#[test]
fn test_progress_builder_fast() {
    let spinner = ProgressBuilder::fast("Fast operation");

    // Test that spinner can be used
    spinner.set_message("Processing quickly...");
    spinner.finish_success(Some("Fast operation completed"));
}

#[test]
fn test_progress_builder_backup() {
    let spinner = ProgressBuilder::backup("Backup operation");

    spinner.set_message("Backing up data...");
    spinner.finish_success(None); // Should use default success message
}

#[test]
fn test_progress_builder_memory() {
    let spinner = ProgressBuilder::memory("Memory operation");

    spinner.set_message("Processing memory...");
    spinner.finish_success(Some("Memory operation done"));
}

#[test]
fn test_adaptive_spinner_error_finish() {
    let spinner = ProgressType::Medium.create_spinner("Test operation");

    spinner.set_message("Something went wrong...");
    spinner.finish_error("Operation failed");
}

#[test]
fn test_adaptive_spinner_progress() {
    let spinner = ProgressType::Slow.create_spinner("Long operation");

    // Test progress updates
    spinner.set_progress(0, 100);
    spinner.set_progress(25, 100);
    spinner.set_progress(50, 100);
    spinner.set_progress(100, 100);

    spinner.finish_success(Some("Progress completed"));
}

#[test]
fn test_adaptive_delay() {
    let fast_spinner = ProgressType::Fast.create_spinner("Fast");
    let slow_spinner = ProgressType::Slow.create_spinner("Slow");

    let fast_delay = fast_spinner.adaptive_delay();
    let slow_delay = slow_spinner.adaptive_delay();

    assert!(fast_delay < slow_delay);
    assert_eq!(fast_delay, Duration::from_millis(50));
    assert_eq!(slow_delay, Duration::from_millis(200));

    fast_spinner.finish_and_clear();
    slow_spinner.finish_and_clear();
}

#[test]
fn test_multi_stage_progress() {
    let stages = vec![
        ("Initializing", ProgressType::Fast),
        ("Processing", ProgressType::Medium),
        ("Finalizing", ProgressType::Slow),
    ];

    let mut multi_progress = MultiStageProgress::new(stages);

    // Test stage progression
    assert!(multi_progress.next_stage()); // Stage 1
    assert!(multi_progress.current_spinner().is_some());

    assert!(multi_progress.next_stage()); // Stage 2
    assert!(multi_progress.current_spinner().is_some());

    assert!(multi_progress.next_stage()); // Stage 3
    assert!(multi_progress.current_spinner().is_some());

    assert!(!multi_progress.next_stage()); // No more stages

    multi_progress.finish();
}

#[tokio::test]
async fn test_async_spinner_simulation() {
    let spinner = ProgressType::Memory.create_spinner("Memory processing");

    spinner.set_message("Starting memory operation...");
    sleep(Duration::from_millis(10)).await;

    spinner.set_message("Processing data in memory...");
    sleep(Duration::from_millis(10)).await;

    spinner.set_message("Finalizing memory operation...");
    sleep(Duration::from_millis(10)).await;

    spinner.finish_success(Some("✅ Memory operation completed!"));
}

#[tokio::test]
async fn test_async_multi_stage() {
    let stages = vec![
        ("Setup", ProgressType::Fast),
        ("Processing", ProgressType::Medium),
    ];

    let mut multi_progress = MultiStageProgress::new(stages);

    // First stage
    multi_progress.next_stage();
    if let Some(spinner) = multi_progress.current_spinner() {
        spinner.set_message("Setting up environment...");
        sleep(Duration::from_millis(10)).await;
    }

    // Second stage
    multi_progress.next_stage();
    if let Some(spinner) = multi_progress.current_spinner() {
        spinner.set_message("Processing data...");
        sleep(Duration::from_millis(10)).await;
    }

    multi_progress.finish();
}

#[test]
fn test_backup_progress_type() {
    let backup_spinner = ProgressType::Backup.create_spinner("Backup operation");
    let config = ProgressType::Backup.config();

    assert_eq!(config.color, "green");
    assert!(config.success_message.is_some());
    assert_eq!(config.tick_interval, Duration::from_millis(200));

    backup_spinner.finish_success(None);
}

#[test]
fn test_search_progress_type() {
    let search_spinner = ProgressType::Search.create_spinner("Search operation");
    let config = ProgressType::Search.config();

    assert_eq!(config.color, "magenta");
    assert!(config.success_message.is_some());
    assert_eq!(config.tick_interval, Duration::from_millis(300));

    search_spinner.finish_success(None);
}

#[test]
fn test_memory_progress_type() {
    let memory_spinner = ProgressType::Memory.create_spinner("Memory operation");
    let config = ProgressType::Memory.config();

    assert_eq!(config.color, "purple");
    assert!(config.success_message.is_some());
    assert_eq!(config.tick_interval, Duration::from_millis(250));

    memory_spinner.finish_success(None);
}

#[test]
fn test_progress_with_zero_total() {
    let spinner = ProgressType::Fast.create_spinner("Quick task");

    // Test progress with zero total (should handle gracefully)
    spinner.set_progress(0, 0);
    spinner.finish_success(Some("Zero total handled"));
}

#[test]
fn test_spinner_message_updates() {
    let spinner = ProgressType::Medium.create_spinner("Initial message");

    spinner.set_message("Updated message 1");
    spinner.set_message("Updated message 2");
    spinner.set_message("Final message");

    spinner.finish_success(Some("Message updates completed"));
}
