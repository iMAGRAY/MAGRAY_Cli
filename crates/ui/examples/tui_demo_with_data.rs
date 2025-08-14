use ui::{
    components::{
        plan_viewer::{ActionPlan, ActionStep, StepStatus},
        diff_viewer::{DiffData, DiffLine, DiffLineType, FileDiff},
    },
    tui::TUIApp,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🖥  Starting MAGRAY TUI Demo with Test Data...");
    
    // Создаем TUI приложение
    let mut app = TUIApp::new()?;
    
    // Загружаем тестовые данные
    load_test_plan(&mut app);
    load_test_diff(&mut app);
    
    println!("🚀 TUI initialized with test data. Use Tab to switch focus, Arrow keys to navigate, 'q' to quit.");
    
    // Запускаем TUI
    if let Err(e) = app.run() {
        eprintln!("TUI error: {}", e);
        return Err(e);
    }
    
    println!("👋 TUI demo session ended.");
    Ok(())
}

fn load_test_plan(app: &mut TUIApp) {
    let test_plan = ActionPlan {
        id: "test-plan-001".to_string(),
        title: "Implement User Authentication System".to_string(),
        description: "Complete implementation of JWT-based authentication with role management".to_string(),
        steps: vec![
            ActionStep {
                id: "auth-001".to_string(),
                description: "Create User model and database schema".to_string(),
                details: "Define User struct with fields: id, username, email, password_hash, role, created_at, updated_at. Create migration scripts for PostgreSQL.".to_string(),
                status: StepStatus::Completed,
                dependencies: vec![],
                tools: vec!["diesel-cli".to_string(), "postgresql".to_string()],
                estimated_duration: Some(1200), // 20 minutes
            },
            ActionStep {
                id: "auth-002".to_string(),
                description: "Implement password hashing service".to_string(),
                details: "Use bcrypt library for secure password hashing with configurable cost factor. Add validation for password strength.".to_string(),
                status: StepStatus::InProgress,
                dependencies: vec!["auth-001".to_string()],
                tools: vec!["bcrypt".to_string(), "validator".to_string()],
                estimated_duration: Some(900), // 15 minutes
            },
            ActionStep {
                id: "auth-003".to_string(),
                description: "Create JWT token service".to_string(),
                details: "Implement JWT creation, validation, and refresh mechanisms. Include role-based claims for authorization.".to_string(),
                status: StepStatus::Pending,
                dependencies: vec!["auth-001".to_string()],
                tools: vec!["jsonwebtoken".to_string(), "chrono".to_string()],
                estimated_duration: Some(1800), // 30 minutes
            },
            ActionStep {
                id: "auth-004".to_string(),
                description: "Build authentication middleware".to_string(),
                details: "Create middleware for token validation, role checking, and request authentication. Handle token refresh automatically.".to_string(),
                status: StepStatus::Pending,
                dependencies: vec!["auth-003".to_string()],
                tools: vec!["axum".to_string(), "tower".to_string()],
                estimated_duration: Some(2400), // 40 minutes
            },
            ActionStep {
                id: "auth-005".to_string(),
                description: "Implement login/logout endpoints".to_string(),
                details: "Create REST endpoints for user registration, login, logout, and token refresh. Include input validation and error handling.".to_string(),
                status: StepStatus::Failed,
                dependencies: vec!["auth-002".to_string(), "auth-003".to_string()],
                tools: vec!["axum".to_string(), "serde".to_string(), "validator".to_string()],
                estimated_duration: Some(3600), // 60 minutes
            },
        ],
        created_at: "2025-08-15T20:00:00Z".to_string(),
    };
    
    let plan_json = serde_json::to_string(&test_plan).unwrap();
    app.load_plan(plan_json);
}

fn load_test_diff(app: &mut TUIApp) {
    let test_diff = DiffData {
        title: "Authentication System Changes".to_string(),
        files: vec![
            FileDiff {
                old_path: "src/models/mod.rs".to_string(),
                new_path: "src/models/mod.rs".to_string(),
                lines: vec![
                    DiffLine {
                        line_type: DiffLineType::Header,
                        old_line_number: None,
                        new_line_number: None,
                        content: "@@ -1,5 +1,8 @@".to_string(),
                    },
                    DiffLine {
                        line_type: DiffLineType::Context,
                        old_line_number: Some(1),
                        new_line_number: Some(1),
                        content: "pub mod user;".to_string(),
                    },
                    DiffLine {
                        line_type: DiffLineType::Context,
                        old_line_number: Some(2),
                        new_line_number: Some(2),
                        content: "pub mod project;".to_string(),
                    },
                    DiffLine {
                        line_type: DiffLineType::Add,
                        old_line_number: None,
                        new_line_number: Some(3),
                        content: "pub mod auth;".to_string(),
                    },
                    DiffLine {
                        line_type: DiffLineType::Add,
                        old_line_number: None,
                        new_line_number: Some(4),
                        content: "pub mod token;".to_string(),
                    },
                    DiffLine {
                        line_type: DiffLineType::Context,
                        old_line_number: Some(3),
                        new_line_number: Some(5),
                        content: "".to_string(),
                    },
                ],
                is_binary: false,
            },
            FileDiff {
                old_path: "src/models/auth.rs".to_string(),
                new_path: "src/models/auth.rs".to_string(),
                lines: vec![
                    DiffLine {
                        line_type: DiffLineType::Header,
                        old_line_number: None,
                        new_line_number: None,
                        content: "@@ -0,0 +1,42 @@".to_string(),
                    },
                    DiffLine {
                        line_type: DiffLineType::Add,
                        old_line_number: None,
                        new_line_number: Some(1),
                        content: "use serde::{Deserialize, Serialize};".to_string(),
                    },
                    DiffLine {
                        line_type: DiffLineType::Add,
                        old_line_number: None,
                        new_line_number: Some(2),
                        content: "use diesel::{Queryable, Insertable};".to_string(),
                    },
                    DiffLine {
                        line_type: DiffLineType::Add,
                        old_line_number: None,
                        new_line_number: Some(3),
                        content: "use chrono::{DateTime, Utc};".to_string(),
                    },
                    DiffLine {
                        line_type: DiffLineType::Add,
                        old_line_number: None,
                        new_line_number: Some(4),
                        content: "".to_string(),
                    },
                    DiffLine {
                        line_type: DiffLineType::Add,
                        old_line_number: None,
                        new_line_number: Some(5),
                        content: "#[derive(Debug, Clone, Serialize, Deserialize, Queryable)]".to_string(),
                    },
                    DiffLine {
                        line_type: DiffLineType::Add,
                        old_line_number: None,
                        new_line_number: Some(6),
                        content: "pub struct User {".to_string(),
                    },
                    DiffLine {
                        line_type: DiffLineType::Add,
                        old_line_number: None,
                        new_line_number: Some(7),
                        content: "    pub id: i32,".to_string(),
                    },
                    DiffLine {
                        line_type: DiffLineType::Add,
                        old_line_number: None,
                        new_line_number: Some(8),
                        content: "    pub username: String,".to_string(),
                    },
                    DiffLine {
                        line_type: DiffLineType::Add,
                        old_line_number: None,
                        new_line_number: Some(9),
                        content: "    pub email: String,".to_string(),
                    },
                    DiffLine {
                        line_type: DiffLineType::Add,
                        old_line_number: None,
                        new_line_number: Some(10),
                        content: "    pub password_hash: String,".to_string(),
                    },
                    DiffLine {
                        line_type: DiffLineType::Add,
                        old_line_number: None,
                        new_line_number: Some(11),
                        content: "    pub role: UserRole,".to_string(),
                    },
                    DiffLine {
                        line_type: DiffLineType::Add,
                        old_line_number: None,
                        new_line_number: Some(12),
                        content: "    pub created_at: DateTime<Utc>,".to_string(),
                    },
                    DiffLine {
                        line_type: DiffLineType::Add,
                        old_line_number: None,
                        new_line_number: Some(13),
                        content: "    pub updated_at: DateTime<Utc>,".to_string(),
                    },
                    DiffLine {
                        line_type: DiffLineType::Add,
                        old_line_number: None,
                        new_line_number: Some(14),
                        content: "}".to_string(),
                    },
                ],
                is_binary: false,
            },
        ],
        created_at: "2025-08-15T20:05:00Z".to_string(),
    };
    
    let diff_json = serde_json::to_string(&test_diff).unwrap();
    app.load_diff(diff_json);
}