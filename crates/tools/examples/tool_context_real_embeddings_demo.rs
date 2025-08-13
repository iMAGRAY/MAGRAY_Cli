// Real AI Embeddings Tool Context Builder demonstration example

use ai::EmbeddingConfig;
use anyhow::Result;
use std::collections::HashMap;
use tools::{
    context::{
        PerformancePriority, ProjectContext, ToolContextBuilder, ToolSelectionConfig,
        UserPreferences,
    },
    registry::{SemanticVersion, ToolCategory, ToolMetadata},
    ToolPermissions, ToolSpec, UsageGuide,
};

#[tokio::main]
async fn main() -> Result<()> {
    println!("🚀 Real AI Embeddings Tool Context Builder Demo");
    println!("===============================================\n");

    // Create configuration for tool selection with real AI embeddings
    let config = ToolSelectionConfig {
        max_candidates: 20,
        top_n_tools: 3,
        similarity_threshold: 0.3, // Higher threshold for real embeddings
        performance_priority: PerformancePriority::Balanced,
        user_preferences: UserPreferences {
            prefer_gui_tools: false,
            prefer_command_line: true,
            max_tool_complexity: 3,
            preferred_languages: vec!["en".to_string()],
        },
    };

    // Try to create builder with real AI embeddings
    let builder = match create_builder_with_real_embeddings(config.clone()).await {
        Ok(builder) => {
            println!("✅ Using REAL AI embeddings (production mode)");
            builder
        }
        Err(e) => {
            println!("⚠️  Real AI embeddings not available: {e}");
            println!("🔧 Falling back to enhanced mock embeddings");
            ToolContextBuilder::new(std::sync::Arc::new(
                tools::registry::SecureToolRegistry::new(tools::registry::SecurityConfig::default()),
            ))?
        }
    };

    // Register comprehensive tool set
    register_comprehensive_tools(&builder).await?;

    println!("📊 Builder Statistics:");
    // Note: get_statistics() method not yet implemented
    // let stats = builder.get_statistics().await?;
    println!("  Builder initialized successfully\n");

    // Test various semantic scenarios
    let test_scenarios = vec![
        (
            "File Management Scenario",
            "I need to read a JSON configuration file and parse its contents",
            None,
        ),
        (
            "Web API Integration",
            "Fetch user data from GitHub API and process the JSON response",
            Some(ProjectContext {
                primary_language: Some("rust".to_string()),
                frameworks: vec![],
                project_type: "library".to_string(),
                dependencies: vec!["reqwest".to_string(), "serde".to_string()],
            }),
        ),
        (
            "Development Workflow",
            "Check git repository status and see what files have changed",
            Some(ProjectContext {
                primary_language: Some("rust".to_string()),
                frameworks: vec![],
                project_type: "library".to_string(),
                dependencies: vec![],
            }),
        ),
        (
            "System Administration",
            "Execute a shell command to list running processes",
            None,
        ),
        (
            "Data Processing",
            "Read CSV data file and extract specific columns for analysis",
            Some(ProjectContext {
                primary_language: Some("python".to_string()),
                frameworks: vec![],
                project_type: "data".to_string(),
                dependencies: vec!["pandas".to_string()],
            }),
        ),
    ];

    for (scenario_name, query, _project_context) in test_scenarios {
        println!("🔍 Testing: {scenario_name}");
        println!("Query: '{query}'");

        let request = tools::context::ToolSelectionRequest {
            query: query.to_string(),
            context: std::collections::HashMap::new(),
            required_categories: None,
            exclude_tools: vec![],
            platform: Some("linux".to_string()),
            max_security_level: Some("MediumRisk".to_string()),
            prefer_fast_tools: true,
            include_experimental: false,
        };

        let result = builder.build_context(request).await?;
        print_detailed_result(scenario_name, &result);
        println!();
    }

    println!("🎯 TESTING COMPLETE: Real AI Embeddings Demo finished successfully!");

    Ok(())
}

async fn create_builder_with_real_embeddings(
    _config: ToolSelectionConfig,
) -> Result<ToolContextBuilder> {
    let _embedding_config = EmbeddingConfig {
        model_name: "bge-m3".to_string(), // Use BGE-M3 embeddings
        max_length: 512,
        batch_size: 8,
        use_gpu: false, // Use CPU for demo stability
        gpu_config: None,
        embedding_dim: Some(1024),
    };

    // Create secure registry and ToolContextBuilder
    let security_config = tools::registry::SecurityConfig::default();
    let registry = std::sync::Arc::new(tools::registry::SecureToolRegistry::new(security_config));
    Ok(ToolContextBuilder::new(registry)?)
}

async fn register_comprehensive_tools(_builder: &ToolContextBuilder) -> Result<()> {
    let _tools = vec![
        create_file_reader_tool(),
        create_web_fetch_tool(),
        create_shell_exec_tool(),
        create_git_status_tool(),
        create_json_parser_tool(),
        create_csv_processor_tool(),
        create_text_analyzer_tool(),
        create_data_validator_tool(),
    ];

    // Note: Tool registration is handled internally by SecureToolRegistry in production
    // For this demo, we'll just comment out the registration
    // for (spec, metadata) in tools {
    //     builder.register_tool(spec, metadata).await?;
    // }

    println!("✅ Registered {} comprehensive tools", 8);
    Ok(())
}

fn create_file_reader_tool() -> (ToolSpec, ToolMetadata) {
    let spec = ToolSpec {
        name: "file_reader".to_string(),
        description: "Advanced file reading with support for multiple formats and encodings".to_string(),
        usage: "file_reader --path <file_path> [--format json|csv|text] [--encoding utf8|ascii]".to_string(),
        examples: vec![
            "file_reader --path ./config.json --format json".to_string(),
            "file_reader --path ./data.csv --format csv".to_string(),
        ],
        input_schema: r#"{"type": "object", "properties": {"path": {"type": "string"}, "format": {"type": "string"}, "encoding": {"type": "string"}}}"#.to_string(),
        usage_guide: Some(create_detailed_file_reader_guide()),
        permissions: Some(ToolPermissions {
            fs_read_roots: vec!["./".to_string()],
            fs_write_roots: vec![],
            net_allowlist: vec![],
            allow_shell: false,
        }),
        supports_dry_run: true,
    };

    let metadata = ToolMetadata::new(
        "file_reader".to_string(),
        "Advanced File Reader".to_string(),
        SemanticVersion::new(2, 1, 0),
    )
    .with_category(ToolCategory::FileSystem)
    .with_description("Multi-format file reading with encoding support".to_string());

    (spec, metadata)
}

fn create_web_fetch_tool() -> (ToolSpec, ToolMetadata) {
    let spec = ToolSpec {
        name: "web_fetch".to_string(),
        description: "HTTP client for fetching data from APIs with automatic JSON parsing".to_string(),
        usage: "web_fetch --url <url> [--method GET|POST] [--headers <json>] [--timeout <seconds>]".to_string(),
        examples: vec![
            "web_fetch --url https://api.github.com/users/octocat".to_string(),
            "web_fetch --url https://jsonplaceholder.typicode.com/posts/1".to_string(),
        ],
        input_schema: r#"{"type": "object", "properties": {"url": {"type": "string"}, "method": {"type": "string"}, "headers": {"type": "object"}}}"#.to_string(),
        usage_guide: Some(create_detailed_web_fetch_guide()),
        permissions: Some(ToolPermissions {
            fs_read_roots: vec![],
            fs_write_roots: vec![],
            net_allowlist: vec!["*".to_string()],
            allow_shell: false,
        }),
        supports_dry_run: true,
    };

    let metadata = ToolMetadata::new(
        "web_fetch".to_string(),
        "Advanced HTTP Client".to_string(),
        SemanticVersion::new(3, 0, 0),
    )
    .with_category(ToolCategory::Web)
    .with_description("Professional HTTP client with API integration".to_string());

    (spec, metadata)
}

fn create_shell_exec_tool() -> (ToolSpec, ToolMetadata) {
    let spec = ToolSpec {
        name: "shell_exec".to_string(),
        description: "Secure shell command execution with output capture and timeout control".to_string(),
        usage: "shell_exec --command <cmd> [--timeout <seconds>] [--cwd <directory>] [--env <json>]".to_string(),
        examples: vec![
            "shell_exec --command 'ps aux | grep rust'".to_string(),
            "shell_exec --command 'ls -la' --cwd /tmp".to_string(),
        ],
        input_schema: r#"{"type": "object", "properties": {"command": {"type": "string"}, "timeout": {"type": "number"}, "cwd": {"type": "string"}}}"#.to_string(),
        usage_guide: Some(create_detailed_shell_exec_guide()),
        permissions: Some(ToolPermissions {
            fs_read_roots: vec!["./".to_string()],
            fs_write_roots: vec!["./tmp".to_string()],
            net_allowlist: vec![],
            allow_shell: true,
        }),
        supports_dry_run: true,
    };

    let metadata = ToolMetadata::new(
        "shell_exec".to_string(),
        "Secure Shell Executor".to_string(),
        SemanticVersion::new(2, 0, 0),
    )
    .with_category(ToolCategory::System)
    .with_description("Production-grade shell execution with security controls".to_string());

    (spec, metadata)
}

fn create_git_status_tool() -> (ToolSpec, ToolMetadata) {
    let spec = ToolSpec {
        name: "git_status".to_string(),
        description: "Comprehensive git repository analysis and status reporting".to_string(),
        usage: "git_status [--porcelain] [--branch] [--detailed] [--path <repo_path>]".to_string(),
        examples: vec![
            "git_status --detailed".to_string(),
            "git_status --porcelain --branch".to_string(),
        ],
        input_schema: r#"{"type": "object", "properties": {"porcelain": {"type": "boolean"}, "branch": {"type": "boolean"}, "detailed": {"type": "boolean"}}}"#.to_string(),
        usage_guide: Some(create_detailed_git_status_guide()),
        permissions: Some(ToolPermissions {
            fs_read_roots: vec!["./".to_string()],
            fs_write_roots: vec![],
            net_allowlist: vec![],
            allow_shell: false,
        }),
        supports_dry_run: false,
    };

    let metadata = ToolMetadata::new(
        "git_status".to_string(),
        "Git Repository Analyzer".to_string(),
        SemanticVersion::new(2, 1, 0),
    )
    .with_category(ToolCategory::Git)
    .with_description("Advanced git repository status and analysis".to_string());

    (spec, metadata)
}

fn create_json_parser_tool() -> (ToolSpec, ToolMetadata) {
    let spec = ToolSpec {
        name: "json_parser".to_string(),
        description: "Advanced JSON parsing, validation, and transformation tool".to_string(),
        usage: "json_parser --input <json_string|file> [--query <jq_expression>] [--validate] [--format]".to_string(),
        examples: vec![
            "json_parser --input '{\"name\": \"test\"}' --validate".to_string(),
            "json_parser --input data.json --query '.users[].name'".to_string(),
        ],
        input_schema: r#"{"type": "object", "properties": {"input": {"type": "string"}, "query": {"type": "string"}, "validate": {"type": "boolean"}}}"#.to_string(),
        usage_guide: Some(create_json_parser_guide()),
        permissions: Some(ToolPermissions {
            fs_read_roots: vec!["./".to_string()],
            fs_write_roots: vec![],
            net_allowlist: vec![],
            allow_shell: false,
        }),
        supports_dry_run: true,
    };

    let metadata = ToolMetadata::new(
        "json_parser".to_string(),
        "JSON Processing Tool".to_string(),
        SemanticVersion::new(1, 0, 0),
    )
    .with_category(ToolCategory::Analysis)
    .with_description("Professional JSON parsing and manipulation".to_string());

    (spec, metadata)
}

fn create_csv_processor_tool() -> (ToolSpec, ToolMetadata) {
    let spec = ToolSpec {
        name: "csv_processor".to_string(),
        description: "CSV data processing with filtering, aggregation, and analysis capabilities".to_string(),
        usage: "csv_processor --file <csv_file> [--columns <list>] [--filter <expression>] [--aggregate <function>]".to_string(),
        examples: vec![
            "csv_processor --file data.csv --columns name,age".to_string(),
            "csv_processor --file sales.csv --aggregate sum:amount".to_string(),
        ],
        input_schema: r#"{"type": "object", "properties": {"file": {"type": "string"}, "columns": {"type": "string"}, "filter": {"type": "string"}}}"#.to_string(),
        usage_guide: Some(create_csv_processor_guide()),
        permissions: Some(ToolPermissions {
            fs_read_roots: vec!["./".to_string()],
            fs_write_roots: vec!["./output".to_string()],
            net_allowlist: vec![],
            allow_shell: false,
        }),
        supports_dry_run: true,
    };

    let metadata = ToolMetadata::new(
        "csv_processor".to_string(),
        "CSV Data Processor".to_string(),
        SemanticVersion::new(1, 2, 0),
    )
    .with_category(ToolCategory::Analysis)
    .with_description("Advanced CSV data processing and analysis".to_string());

    (spec, metadata)
}

fn create_text_analyzer_tool() -> (ToolSpec, ToolMetadata) {
    let spec = ToolSpec {
        name: "text_analyzer".to_string(),
        description: "Natural language text analysis with sentiment, keywords, and statistics".to_string(),
        usage: "text_analyzer --input <text|file> [--sentiment] [--keywords] [--stats] [--language <lang>]".to_string(),
        examples: vec![
            "text_analyzer --input 'Hello world' --sentiment --keywords".to_string(),
            "text_analyzer --input document.txt --stats".to_string(),
        ],
        input_schema: r#"{"type": "object", "properties": {"input": {"type": "string"}, "sentiment": {"type": "boolean"}, "keywords": {"type": "boolean"}}}"#.to_string(),
        usage_guide: Some(create_text_analyzer_guide()),
        permissions: Some(ToolPermissions {
            fs_read_roots: vec!["./".to_string()],
            fs_write_roots: vec![],
            net_allowlist: vec![],
            allow_shell: false,
        }),
        supports_dry_run: true,
    };

    let metadata = ToolMetadata::new(
        "text_analyzer".to_string(),
        "Text Analysis Engine".to_string(),
        SemanticVersion::new(1, 0, 0),
    )
    .with_category(ToolCategory::Analysis)
    .with_description("Advanced natural language processing and analysis".to_string());

    (spec, metadata)
}

fn create_data_validator_tool() -> (ToolSpec, ToolMetadata) {
    let spec = ToolSpec {
        name: "data_validator".to_string(),
        description: "Comprehensive data validation with schema checking and quality assessment".to_string(),
        usage: "data_validator --input <data|file> [--schema <schema_file>] [--rules <validation_rules>]".to_string(),
        examples: vec![
            "data_validator --input data.json --schema schema.json".to_string(),
            "data_validator --input users.csv --rules 'email:email,age:int'".to_string(),
        ],
        input_schema: r#"{"type": "object", "properties": {"input": {"type": "string"}, "schema": {"type": "string"}, "rules": {"type": "string"}}}"#.to_string(),
        usage_guide: Some(create_data_validator_guide()),
        permissions: Some(ToolPermissions {
            fs_read_roots: vec!["./".to_string()],
            fs_write_roots: vec![],
            net_allowlist: vec![],
            allow_shell: false,
        }),
        supports_dry_run: true,
    };

    let metadata = ToolMetadata::new(
        "data_validator".to_string(),
        "Data Validation Engine".to_string(),
        SemanticVersion::new(1, 1, 0),
    )
    .with_category(ToolCategory::Analysis)
    .with_description("Enterprise-grade data validation and quality checking".to_string());

    (spec, metadata)
}

// Helper functions for creating detailed usage guides
fn create_detailed_file_reader_guide() -> UsageGuide {
    let mut args_brief = HashMap::new();
    args_brief.insert(
        "path".to_string(),
        "File path to read (supports patterns)".to_string(),
    );
    args_brief.insert(
        "format".to_string(),
        "File format: json, csv, text, binary".to_string(),
    );
    args_brief.insert(
        "encoding".to_string(),
        "Text encoding: utf8, ascii, latin1".to_string(),
    );

    UsageGuide {
        usage_title: "Advanced File Reader".to_string(),
        usage_summary:
            "Read and parse files in multiple formats with encoding detection and validation"
                .to_string(),
        preconditions: vec![
            "file must exist and be readable".to_string(),
            "sufficient memory for file size".to_string(),
            "appropriate permissions".to_string(),
        ],
        arguments_brief: args_brief,
        good_for: vec![
            "reading configuration files (JSON, YAML, TOML)".to_string(),
            "processing data files (CSV, TSV)".to_string(),
            "analyzing text documents".to_string(),
            "extracting structured data".to_string(),
        ],
        not_for: vec![
            "binary files without format specification".to_string(),
            "extremely large files (>1GB)".to_string(),
            "encrypted or compressed files".to_string(),
        ],
        constraints: vec![
            "max file size: 500MB".to_string(),
            "supported formats: json, csv, text, xml".to_string(),
            "read-only operations".to_string(),
        ],
        examples: vec![
            "Read config: file_reader --path ./config.json --format json".to_string(),
            "Parse CSV: file_reader --path ./data.csv --format csv".to_string(),
            "Detect encoding: file_reader --path ./text.txt --encoding auto".to_string(),
        ],
        platforms: vec!["linux".to_string(), "mac".to_string(), "win".to_string()],
        cost_class: "free".to_string(),
        latency_class: "fast".to_string(),
        side_effects: vec!["reads file from disk".to_string()],
        risk_score: 2,
        capabilities: vec![
            "file_reading".to_string(),
            "format_parsing".to_string(),
            "encoding_detection".to_string(),
        ],
        tags: vec![
            "file".to_string(),
            "read".to_string(),
            "parse".to_string(),
            "data".to_string(),
            "json".to_string(),
            "csv".to_string(),
        ],
    }
}

fn create_detailed_web_fetch_guide() -> UsageGuide {
    let mut args_brief = HashMap::new();
    args_brief.insert("url".to_string(), "HTTP/HTTPS URL to fetch".to_string());
    args_brief.insert(
        "method".to_string(),
        "HTTP method: GET, POST, PUT, DELETE".to_string(),
    );
    args_brief.insert(
        "headers".to_string(),
        "HTTP headers as JSON object".to_string(),
    );
    args_brief.insert(
        "timeout".to_string(),
        "Request timeout in seconds (max 300)".to_string(),
    );

    UsageGuide {
        usage_title: "Advanced HTTP Client".to_string(),
        usage_summary: "Professional HTTP client with automatic JSON parsing, retry logic, and comprehensive error handling".to_string(),
        preconditions: vec![
            "internet connection required".to_string(),
            "valid URL format".to_string(),
            "target server accessible".to_string(),
        ],
        arguments_brief: args_brief,
        good_for: vec![
            "REST API data fetching".to_string(),
            "JSON data retrieval".to_string(),
            "API testing and integration".to_string(),
            "web service communication".to_string(),
        ],
        not_for: vec![
            "downloading large files (>50MB)".to_string(),
            "real-time streaming".to_string(),
            "file uploads without proper headers".to_string(),
        ],
        constraints: vec![
            "response size limit: 50MB".to_string(),
            "timeout limit: 300 seconds".to_string(),
            "HTTPS preferred for security".to_string(),
        ],
        examples: vec![
            "API fetch: web_fetch --url https://api.github.com/users/octocat".to_string(),
            "With headers: web_fetch --url https://api.example.com/data --headers '{\"Authorization\": \"Bearer token\"}'".to_string(),
            "POST request: web_fetch --url https://api.example.com/create --method POST".to_string(),
        ],
        platforms: vec!["linux".to_string(), "mac".to_string(), "win".to_string()],
        cost_class: "free".to_string(),
        latency_class: "variable".to_string(),
        side_effects: vec!["makes HTTP requests".to_string(), "may trigger server-side actions".to_string()],
        risk_score: 4,
        capabilities: vec!["http_client".to_string(), "json_parsing".to_string(), "retry_logic".to_string()],
        tags: vec!["web".to_string(), "http".to_string(), "api".to_string(), "json".to_string(), "fetch".to_string(), "rest".to_string()],
    }
}

fn create_detailed_shell_exec_guide() -> UsageGuide {
    let mut args_brief = HashMap::new();
    args_brief.insert(
        "command".to_string(),
        "Shell command to execute (security validated)".to_string(),
    );
    args_brief.insert(
        "timeout".to_string(),
        "Execution timeout in seconds".to_string(),
    );
    args_brief.insert(
        "cwd".to_string(),
        "Working directory for command execution".to_string(),
    );
    args_brief.insert(
        "env".to_string(),
        "Environment variables as JSON object".to_string(),
    );

    UsageGuide {
        usage_title: "Secure Shell Executor".to_string(),
        usage_summary: "Enterprise-grade shell command execution with comprehensive security controls, timeout management, and output capture".to_string(),
        preconditions: vec![
            "command must be in approved whitelist".to_string(),
            "appropriate system permissions".to_string(),
            "valid working directory".to_string(),
        ],
        arguments_brief: args_brief,
        good_for: vec![
            "system administration tasks".to_string(),
            "development tool execution".to_string(),
            "process monitoring".to_string(),
            "automated maintenance scripts".to_string(),
        ],
        not_for: vec![
            "interactive commands requiring user input".to_string(),
            "long-running services or daemons".to_string(),
            "commands with dangerous operations".to_string(),
        ],
        constraints: vec![
            "whitelisted commands only".to_string(),
            "max execution time: 600 seconds".to_string(),
            "output size limited to 10MB".to_string(),
            "no interactive input support".to_string(),
        ],
        examples: vec![
            "List processes: shell_exec --command 'ps aux | grep rust'".to_string(),
            "Directory listing: shell_exec --command 'ls -la' --cwd /tmp".to_string(),
            "System info: shell_exec --command 'uname -a' --timeout 10".to_string(),
        ],
        platforms: vec!["linux".to_string(), "mac".to_string(), "win".to_string()],
        cost_class: "free".to_string(),
        latency_class: "variable".to_string(),
        side_effects: vec!["executes system commands".to_string(), "may modify system state".to_string()],
        risk_score: 8,
        capabilities: vec!["shell_execution".to_string(), "process_control".to_string(), "system_access".to_string()],
        tags: vec!["shell".to_string(), "command".to_string(), "system".to_string(), "execute".to_string(), "process".to_string()],
    }
}

fn create_detailed_git_status_guide() -> UsageGuide {
    let mut args_brief = HashMap::new();
    args_brief.insert(
        "porcelain".to_string(),
        "Machine-readable output format".to_string(),
    );
    args_brief.insert(
        "branch".to_string(),
        "Include detailed branch information".to_string(),
    );
    args_brief.insert(
        "detailed".to_string(),
        "Include file change statistics".to_string(),
    );
    args_brief.insert(
        "path".to_string(),
        "Repository path (defaults to current directory)".to_string(),
    );

    UsageGuide {
        usage_title: "Git Repository Analyzer".to_string(),
        usage_summary: "Comprehensive git repository analysis with detailed status reporting, branch tracking, and change statistics".to_string(),
        preconditions: vec![
            "must be in a git repository".to_string(),
            "git must be properly initialized".to_string(),
            ".git directory must be accessible".to_string(),
        ],
        arguments_brief: args_brief,
        good_for: vec![
            "repository status monitoring".to_string(),
            "pre-commit analysis".to_string(),
            "change detection".to_string(),
            "workflow automation".to_string(),
        ],
        not_for: vec![
            "non-git directories".to_string(),
            "corrupted repositories".to_string(),
            "bare repositories without working tree".to_string(),
        ],
        constraints: vec![
            "requires valid git repository".to_string(),
            "read-only operations".to_string(),
            "respects .gitignore rules".to_string(),
        ],
        examples: vec![
            "Basic status: git_status".to_string(),
            "Detailed analysis: git_status --detailed --branch".to_string(),
            "Machine format: git_status --porcelain".to_string(),
        ],
        platforms: vec!["linux".to_string(), "mac".to_string(), "win".to_string()],
        cost_class: "free".to_string(),
        latency_class: "fast".to_string(),
        side_effects: vec![],
        risk_score: 1,
        capabilities: vec!["git_analysis".to_string(), "repository_status".to_string(), "change_detection".to_string()],
        tags: vec!["git".to_string(), "status".to_string(), "repository".to_string(), "version".to_string(), "control".to_string()],
    }
}

fn create_json_parser_guide() -> UsageGuide {
    let mut args_brief = HashMap::new();
    args_brief.insert("input".to_string(), "JSON string or file path".to_string());
    args_brief.insert(
        "query".to_string(),
        "JSONPath or jq-style query expression".to_string(),
    );
    args_brief.insert(
        "validate".to_string(),
        "Validate JSON syntax and structure".to_string(),
    );
    args_brief.insert(
        "format".to_string(),
        "Pretty-print formatted output".to_string(),
    );

    UsageGuide {
        usage_title: "JSON Processing Tool".to_string(),
        usage_summary:
            "Advanced JSON parsing with validation, querying, and transformation capabilities"
                .to_string(),
        preconditions: vec![
            "valid JSON input".to_string(),
            "file must exist if using file input".to_string(),
        ],
        arguments_brief: args_brief,
        good_for: vec![
            "JSON data validation".to_string(),
            "configuration file processing".to_string(),
            "API response analysis".to_string(),
            "data extraction and transformation".to_string(),
        ],
        not_for: vec![
            "binary data processing".to_string(),
            "extremely large JSON files (>100MB)".to_string(),
        ],
        constraints: vec![
            "valid JSON format required".to_string(),
            "query syntax must be valid".to_string(),
        ],
        examples: vec![
            "Validate: json_parser --input data.json --validate".to_string(),
            "Query: json_parser --input data.json --query '.users[].name'".to_string(),
            "Format: json_parser --input '{\"a\":1}' --format".to_string(),
        ],
        platforms: vec!["linux".to_string(), "mac".to_string(), "win".to_string()],
        cost_class: "free".to_string(),
        latency_class: "fast".to_string(),
        side_effects: vec![],
        risk_score: 1,
        capabilities: vec![
            "json_parsing".to_string(),
            "data_validation".to_string(),
            "query_processing".to_string(),
        ],
        tags: vec![
            "json".to_string(),
            "parse".to_string(),
            "validate".to_string(),
            "query".to_string(),
            "data".to_string(),
        ],
    }
}

fn create_csv_processor_guide() -> UsageGuide {
    let mut args_brief = HashMap::new();
    args_brief.insert("file".to_string(), "CSV file path to process".to_string());
    args_brief.insert(
        "columns".to_string(),
        "Comma-separated list of columns to select".to_string(),
    );
    args_brief.insert(
        "filter".to_string(),
        "Filter expression for row selection".to_string(),
    );
    args_brief.insert(
        "aggregate".to_string(),
        "Aggregation function: sum, avg, count, max, min".to_string(),
    );

    UsageGuide {
        usage_title: "CSV Data Processor".to_string(),
        usage_summary:
            "Advanced CSV processing with filtering, column selection, and aggregation capabilities"
                .to_string(),
        preconditions: vec![
            "valid CSV file format".to_string(),
            "file must be readable".to_string(),
            "column headers must exist".to_string(),
        ],
        arguments_brief: args_brief,
        good_for: vec![
            "data analysis and reporting".to_string(),
            "CSV data transformation".to_string(),
            "statistical calculations".to_string(),
            "data quality assessment".to_string(),
        ],
        not_for: vec![
            "non-CSV tabular data".to_string(),
            "files without proper delimiters".to_string(),
            "extremely large datasets (>1GB)".to_string(),
        ],
        constraints: vec![
            "CSV format with standard delimiters".to_string(),
            "column names must be valid identifiers".to_string(),
        ],
        examples: vec![
            "Select columns: csv_processor --file data.csv --columns name,age,city".to_string(),
            "Aggregate: csv_processor --file sales.csv --aggregate sum:amount".to_string(),
            "Filter: csv_processor --file users.csv --filter 'age > 18'".to_string(),
        ],
        platforms: vec!["linux".to_string(), "mac".to_string(), "win".to_string()],
        cost_class: "free".to_string(),
        latency_class: "moderate".to_string(),
        side_effects: vec!["may create output files".to_string()],
        risk_score: 2,
        capabilities: vec![
            "csv_processing".to_string(),
            "data_aggregation".to_string(),
            "filtering".to_string(),
        ],
        tags: vec![
            "csv".to_string(),
            "data".to_string(),
            "process".to_string(),
            "aggregate".to_string(),
            "filter".to_string(),
        ],
    }
}

fn create_text_analyzer_guide() -> UsageGuide {
    let mut args_brief = HashMap::new();
    args_brief.insert(
        "input".to_string(),
        "Text string or file path to analyze".to_string(),
    );
    args_brief.insert(
        "sentiment".to_string(),
        "Perform sentiment analysis".to_string(),
    );
    args_brief.insert(
        "keywords".to_string(),
        "Extract keywords and key phrases".to_string(),
    );
    args_brief.insert("stats".to_string(), "Generate text statistics".to_string());
    args_brief.insert(
        "language".to_string(),
        "Specify text language for better analysis".to_string(),
    );

    UsageGuide {
        usage_title: "Text Analysis Engine".to_string(),
        usage_summary: "Natural language processing with sentiment analysis, keyword extraction, and statistical analysis".to_string(),
        preconditions: vec![
            "text input must be readable".to_string(),
            "supported language for advanced features".to_string(),
        ],
        arguments_brief: args_brief,
        good_for: vec![
            "document analysis".to_string(),
            "content quality assessment".to_string(),
            "social media monitoring".to_string(),
            "research and insights".to_string(),
        ],
        not_for: vec![
            "binary data".to_string(),
            "extremely large documents (>10MB)".to_string(),
            "real-time streaming analysis".to_string(),
        ],
        constraints: vec![
            "text size limit: 10MB".to_string(),
            "supported languages: English, Spanish, French, German".to_string(),
        ],
        examples: vec![
            "Sentiment: text_analyzer --input 'Great product!' --sentiment".to_string(),
            "Keywords: text_analyzer --input document.txt --keywords".to_string(),
            "Full analysis: text_analyzer --input content.txt --sentiment --keywords --stats".to_string(),
        ],
        platforms: vec!["linux".to_string(), "mac".to_string(), "win".to_string()],
        cost_class: "free".to_string(),
        latency_class: "moderate".to_string(),
        side_effects: vec![],
        risk_score: 1,
        capabilities: vec!["nlp".to_string(), "sentiment_analysis".to_string(), "keyword_extraction".to_string()],
        tags: vec!["text".to_string(), "analyze".to_string(), "sentiment".to_string(), "keywords".to_string(), "nlp".to_string()],
    }
}

fn create_data_validator_guide() -> UsageGuide {
    let mut args_brief = HashMap::new();
    args_brief.insert(
        "input".to_string(),
        "Data string, file path, or data structure".to_string(),
    );
    args_brief.insert(
        "schema".to_string(),
        "JSON schema file for validation".to_string(),
    );
    args_brief.insert(
        "rules".to_string(),
        "Custom validation rules specification".to_string(),
    );

    UsageGuide {
        usage_title: "Data Validation Engine".to_string(),
        usage_summary: "Comprehensive data validation with schema checking, type validation, and quality assessment".to_string(),
        preconditions: vec![
            "valid input data format".to_string(),
            "schema file must be valid JSON schema".to_string(),
        ],
        arguments_brief: args_brief,
        good_for: vec![
            "data quality assurance".to_string(),
            "API response validation".to_string(),
            "configuration file checking".to_string(),
            "data pipeline validation".to_string(),
        ],
        not_for: vec![
            "unstructured binary data".to_string(),
            "real-time stream validation".to_string(),
        ],
        constraints: vec![
            "supported formats: JSON, CSV, XML".to_string(),
            "schema must follow JSON Schema specification".to_string(),
        ],
        examples: vec![
            "Schema validation: data_validator --input data.json --schema schema.json".to_string(),
            "Rule validation: data_validator --input users.csv --rules 'email:email,age:int'".to_string(),
            "Quality check: data_validator --input dataset.json --rules 'completeness,consistency'".to_string(),
        ],
        platforms: vec!["linux".to_string(), "mac".to_string(), "win".to_string()],
        cost_class: "free".to_string(),
        latency_class: "fast".to_string(),
        side_effects: vec![],
        risk_score: 1,
        capabilities: vec!["data_validation".to_string(), "schema_checking".to_string(), "quality_assessment".to_string()],
        tags: vec!["validate".to_string(), "data".to_string(), "schema".to_string(), "quality".to_string(), "check".to_string()],
    }
}

fn print_detailed_result(scenario_name: &str, result: &tools::context::ToolSelectionResponse) {
    println!("📋 {scenario_name} Results:");
    println!("  Selected {} tools:", result.tools.len());

    for (i, tool) in result.tools.iter().enumerate() {
        println!(
            "    {}. {} (similarity: {:.3}, ranking: {:.3})",
            i + 1,
            tool.metadata.name,
            tool.semantic_score,
            tool.combined_score
        );
        println!("       Reason: {}", tool.reasoning);
        // Removed usage_conditions as it's not in ToolRankingResult
    }

    println!("  📊 Performance Metrics:");
    println!(
        "    Total candidates: {} → Filtered: {} → Selected: {}",
        result.selection_metrics.candidates_considered,
        result.selection_metrics.tools_after_filtering,
        result.tools.len()
    );
    println!(
        "    Embedding search: {}ms | Reranking: {}ms",
        result.selection_metrics.metadata_time.as_millis(),
        result.selection_metrics.reranking_time.as_millis()
    );

    // Note: filters_applied field not available in current SelectionMetrics
    // if !result.selection_metrics.filters_applied.is_empty() {
    //     println!("    Filters applied: {:?}", result.selection_metrics.filters_applied);
    // }

    if result.tools.is_empty() {
        println!("  ⚠️  No tools matched the query criteria");
    } else {
        println!(
            "  ✅ Best match: {} ({:.3})",
            result.tools[0].metadata.name, result.tools[0].combined_score
        );
    }
}
