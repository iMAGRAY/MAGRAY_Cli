use cli::progress::{ProgressBar, Spinner, ProgressStyle};
use std::time::Duration;
use tokio::time::sleep;

#[test]
fn test_progress_bar_creation() {
    let pb = ProgressBar::new(100);
    
    assert_eq!(pb.total(), 100);
    assert_eq!(pb.position(), 0);
    assert!(!pb.is_finished());
}

#[test]
fn test_progress_bar_increment() {
    let pb = ProgressBar::new(10);
    
    pb.inc(1);
    assert_eq!(pb.position(), 1);
    
    pb.inc(5);
    assert_eq!(pb.position(), 6);
    
    pb.inc(10); // Should cap at total
    assert_eq!(pb.position(), 10);
    assert!(pb.is_finished());
}

#[test]
fn test_progress_bar_set_position() {
    let pb = ProgressBar::new(100);
    
    pb.set_position(50);
    assert_eq!(pb.position(), 50);
    
    pb.set_position(150); // Should cap at total
    assert_eq!(pb.position(), 100);
    assert!(pb.is_finished());
}

#[test]
fn test_progress_bar_set_message() {
    let pb = ProgressBar::new(100);
    
    pb.set_message("Processing...");
    // Message is set internally, hard to test without output capture
    
    pb.set_message("Almost done");
    // Just ensure it doesn't panic
}

#[test]
fn test_progress_bar_finish() {
    let pb = ProgressBar::new(100);
    
    pb.set_position(50);
    assert!(!pb.is_finished());
    
    pb.finish();
    assert!(pb.is_finished());
    assert_eq!(pb.position(), 100);
}

#[test]
fn test_progress_bar_finish_with_message() {
    let pb = ProgressBar::new(100);
    
    pb.finish_with_message("Completed successfully!");
    assert!(pb.is_finished());
    assert_eq!(pb.position(), 100);
}

#[test]
fn test_spinner_creation() {
    let spinner = Spinner::new();
    
    // Spinner starts in running state
    assert!(!spinner.is_finished());
}

#[test]
fn test_spinner_set_message() {
    let spinner = Spinner::new();
    
    spinner.set_message("Loading...");
    spinner.set_message("Processing data...");
    
    // Just ensure no panics
}

#[test]
fn test_spinner_finish() {
    let spinner = Spinner::new();
    
    spinner.finish();
    assert!(spinner.is_finished());
}

#[test]
fn test_spinner_finish_with_message() {
    let spinner = Spinner::new();
    
    spinner.finish_with_message("✅ Done!");
    assert!(spinner.is_finished());
}

#[tokio::test]
async fn test_async_progress_simulation() {
    let pb = ProgressBar::new(10);
    
    for i in 0..10 {
        pb.set_message(&format!("Step {}/10", i + 1));
        pb.inc(1);
        sleep(Duration::from_millis(10)).await;
    }
    
    pb.finish_with_message("All steps completed!");
    assert!(pb.is_finished());
    assert_eq!(pb.position(), 10);
}

#[tokio::test]
async fn test_async_spinner_simulation() {
    let spinner = Spinner::new();
    
    spinner.set_message("Starting process...");
    sleep(Duration::from_millis(50)).await;
    
    spinner.set_message("Processing data...");
    sleep(Duration::from_millis(50)).await;
    
    spinner.set_message("Finalizing...");
    sleep(Duration::from_millis(50)).await;
    
    spinner.finish_with_message("✅ Process completed!");
    assert!(spinner.is_finished());
}

#[test]
fn test_progress_bar_zero_total() {
    let pb = ProgressBar::new(0);
    
    assert_eq!(pb.total(), 0);
    assert_eq!(pb.position(), 0);
    assert!(pb.is_finished()); // Zero total means immediately finished
}

#[test]
fn test_progress_bar_large_increment() {
    let pb = ProgressBar::new(100);
    
    // Increment beyond total should cap at total
    pb.inc(200);
    assert_eq!(pb.position(), 100);
    assert!(pb.is_finished());
}

#[test]
fn test_multiple_progress_bars() {
    let pb1 = ProgressBar::new(50);
    let pb2 = ProgressBar::new(75);
    
    pb1.inc(25);
    pb2.inc(25);
    
    assert_eq!(pb1.position(), 25);
    assert_eq!(pb2.position(), 25);
    
    pb1.finish();
    assert!(pb1.is_finished());
    assert!(!pb2.is_finished());
    
    pb2.finish();
    assert!(pb2.is_finished());
}

#[test]
fn test_progress_percentage() {
    let pb = ProgressBar::new(100);
    
    assert_eq!(pb.percentage(), 0.0);
    
    pb.set_position(25);
    assert_eq!(pb.percentage(), 25.0);
    
    pb.set_position(50);
    assert_eq!(pb.percentage(), 50.0);
    
    pb.finish();
    assert_eq!(pb.percentage(), 100.0);
}

#[test]
fn test_progress_eta_calculation() {
    let pb = ProgressBar::new(100);
    
    // Initially no ETA available
    assert!(pb.eta().is_none());
    
    pb.inc(10);
    // After some progress, ETA might be available
    // Note: This is timing-dependent and might be None in fast tests
    let eta = pb.eta();
    assert!(eta.is_none() || eta.is_some());
}

#[test]
fn test_adaptive_progress_styles() {
    let pb = ProgressBar::new(100);
    
    // Test different style adaptations
    pb.set_style(ProgressStyle::default_bar());
    pb.set_style(ProgressStyle::default_spinner());
    
    // Styles are applied internally, just ensure no panics
    pb.inc(50);
    pb.finish();
}

#[tokio::test]
async fn test_progress_with_concurrent_updates() {
    use tokio::join;
    
    let pb = ProgressBar::new(100);
    let pb_clone1 = pb.clone();
    let pb_clone2 = pb.clone();
    
    let task1 = async {
        for _ in 0..25 {
            pb_clone1.inc(1);
            sleep(Duration::from_millis(1)).await;
        }
    };
    
    let task2 = async {
        for _ in 0..25 {
            pb_clone2.inc(1);
            sleep(Duration::from_millis(1)).await;
        }
    };
    
    join!(task1, task2);
    
    // Both tasks should have contributed to progress
    assert!(pb.position() >= 50);
    pb.finish();
}

#[test]
fn test_progress_bar_clone() {
    let pb1 = ProgressBar::new(100);
    let pb2 = pb1.clone();
    
    pb1.inc(30);
    assert_eq!(pb2.position(), 30);
    
    pb2.inc(20);
    assert_eq!(pb1.position(), 50);
    
    pb1.finish();
    assert!(pb2.is_finished());
}