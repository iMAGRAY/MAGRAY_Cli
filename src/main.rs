mod app;
mod plan_viewer;
mod diff_viewer;
mod action_buttons;

fn main() {
    // Initialize the application
    if let Err(e) = app::run() {
        eprintln!("Application failed to start: {}", e);
        std::process::exit(1);
    }
}
