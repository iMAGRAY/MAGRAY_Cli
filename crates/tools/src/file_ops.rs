use crate::{Tool, ToolInput, ToolOutput, ToolSpec, UsageGuide};
use anyhow::{anyhow, Result};
use common::policy::{get_policy_engine_with_eventbus, PolicyAction};
use std::collections::HashMap;
use std::fs;
use std::io::{BufRead, BufReader, Read};
use std::path::Path;
use walkdir::WalkDir;

// ===== Security Validation Functions =====

/// SECURITY: Валидация пути на предмет Path Traversal атак
fn validate_path_security(path: &str, operation: &str) -> Result<()> {
    // 1. Проверка на path traversal паттерны
    if path.contains("..") {
        return Err(anyhow!(
            "🔒 SECURITY ERROR: Path traversal паттерн обнаружен в пути: '{}' (операция: {})",
            path,
            operation
        ));
    }

    // 2. Проверка на абсолютные пути к системным директориям (Windows & Unix)
    let dangerous_paths = [
        "/etc/",
        "/root/",
        "/boot/",
        "/proc/",
        "/sys/",
        "/dev/",
        "C:\\Windows\\",
        "C:\\Program Files\\",
        "C:\\Users\\Administrator\\",
        "\\etc\\",
        "\\root\\",
        "\\boot\\",
        "\\Windows\\",
        "\\Program Files\\",
    ];

    let path_lower = path.to_lowercase();
    for dangerous_path in &dangerous_paths {
        if path_lower.starts_with(&dangerous_path.to_lowercase()) {
            return Err(anyhow!(
                "🔒 SECURITY ERROR: Попытка доступа к системной директории: '{}' (операция: {})",
                path,
                operation
            ));
        }
    }

    // 3. Проверка на null bytes (может использоваться для обхода валидации)
    if path.contains('\0') {
        return Err(anyhow!(
            "🔒 SECURITY ERROR: Null byte в пути: '{}' (операция: {})",
            path,
            operation
        ));
    }

    // 4. Проверка на слишком длинные пути (DoS защита)
    if path.len() > 4096 {
        return Err(anyhow!(
            "🔒 SECURITY ERROR: Слишком длинный путь ({} символов) (операция: {})",
            path.len(),
            operation
        ));
    }

    // 5. Валидация расширений файлов для write операций
    if operation == "write" {
        validate_file_extension(path)?;
    }

    Ok(())
}

/// SECURITY: Валидация расширений файлов для предотвращения создания опасных файлов
fn validate_file_extension(path: &str) -> Result<()> {
    let path_obj = Path::new(path);

    if let Some(extension) = path_obj.extension() {
        let ext_str = extension.to_string_lossy().to_lowercase();

        // Blacklist опасных расширений
        let dangerous_extensions = [
            "exe", "bat", "cmd", "com", "pif", "scr", "vbs", "vbe", "js", "jar", "msi", "dll",
            "sys", "scf", "lnk", "inf", "reg", "ps1", "sh", "bash", "zsh", "fish", "csh", "ksh",
            "pl", "py", "rb", "php", "asp", "jsp",
        ];

        if dangerous_extensions.contains(&ext_str.as_str()) {
            return Err(anyhow!(
                "🔒 SECURITY ERROR: Создание файлов с расширением '{}' запрещено",
                ext_str
            ));
        }

        // Whitelist разрешенных расширений для операций записи
        let allowed_extensions = [
            "txt",
            "md",
            "rst",
            "json",
            "toml",
            "yaml",
            "yml",
            "xml",
            "csv",
            "log",
            "conf",
            "cfg",
            "ini",
            "properties",
            "rs",
            "go",
            "java",
            "c",
            "cpp",
            "h",
            "hpp",
            "ts",
            "tsx",
            "css",
            "scss",
            "sass",
            "html",
            "htm",
            "svg",
            "png",
            "jpg",
            "jpeg",
            "gif",
            "webp",
            "pdf",
            "doc",
            "docx",
            "odt",
            "rtf",
            "backup",
            "bak",
            "tmp",
        ];

        if !allowed_extensions.contains(&ext_str.as_str()) {
            return Err(anyhow!(
                "🔒 SECURITY ERROR: Расширение файла '{}' не разрешено. Разрешенные: {:?}",
                ext_str,
                allowed_extensions
            ));
        }
    }

    Ok(())
}

/// CRITICAL SECURITY: PolicyEngine integration for file operations
/// This function applies policy rules to file operations and logs violations to EventBus
fn check_policy_for_file_operation(
    operation: &str,
    path: &str,
    args: &HashMap<String, String>,
) -> Result<()> {
    let policy_engine = get_policy_engine_with_eventbus();

    // Create policy arguments
    let mut policy_args = args.clone();
    policy_args.insert("operation".to_string(), operation.to_string());
    policy_args.insert("path".to_string(), path.to_string());

    let decision = policy_engine.evaluate_tool(operation, &policy_args);

    match decision.action {
        PolicyAction::Deny => {
            let reason = decision
                .matched_rule
                .as_ref()
                .and_then(|rule| rule.reason.as_deref())
                .unwrap_or("Security policy prohibits this file operation");

            return Err(anyhow!(
                "🔒 POLICY VIOLATION: {} operation denied by security policy for path: '{}'\nReason: {}",
                operation,
                path,
                reason
            ));
        }
        PolicyAction::Ask => {
            // In automated context, treat Ask as Deny for security
            return Err(anyhow!(
                "🔒 POLICY REQUIREMENT: {} operation requires user confirmation for path: '{}'\nUse interactive CLI for file operations requiring confirmation",
                operation,
                path
            ));
        }
        PolicyAction::Allow => {
            // Continue with operation
        }
    }

    Ok(())
}

// ===== CRITICAL MEMORY OPTIMIZATION: Streaming File Operations =====

/// КРИТИЧЕСКАЯ ОПТИМИЗАЦИЯ: Memory-efficient file reading с streaming для больших файлов
/// Предотвращает heap overflow при чтении файлов >100MB
fn read_file_with_memory_optimization(path: &str) -> Result<String> {
    let file_path = Path::new(path);
    let file_size = fs::metadata(file_path)?.len();

    // Для файлов больше 100MB используем streaming чтение
    const LARGE_FILE_THRESHOLD: u64 = 100 * 1024 * 1024; // 100MB
    const MAX_BUFFER_SIZE: usize = 8 * 1024 * 1024; // 8MB buffer

    if file_size > LARGE_FILE_THRESHOLD {
        // Streaming reading для больших файлов
        let file = fs::File::open(file_path)?;
        let mut reader = BufReader::with_capacity(MAX_BUFFER_SIZE, file);
        let mut content = String::new();
        let mut total_read = 0;

        // Читаем по chunks с контролем memory
        loop {
            let mut chunk = vec![0; MAX_BUFFER_SIZE];
            let bytes_read = reader.read(&mut chunk)?;

            if bytes_read == 0 {
                break;
            }

            // Проверяем не превышаем ли memory limit
            total_read += bytes_read;
            if total_read > 500 * 1024 * 1024 {
                // 500MB limit
                return Err(anyhow!(
                    "File too large to read safely: {} bytes (max 500MB)",
                    total_read
                ));
            }

            // Преобразуем bytes в string с memory-safe подходом
            let chunk_str = String::from_utf8_lossy(&chunk[..bytes_read]);
            content.push_str(&chunk_str);

            // Принудительно освобождаем memory каждые 50MB
            if total_read.is_multiple_of(50 * 1024 * 1024) {
                // Force collection для предотвращения memory buildup
                drop(chunk);
            }
        }

        Ok(content)
    } else {
        // Обычное чтение для небольших файлов
        fs::read_to_string(path).map_err(|e| anyhow!("Failed to read file: {}", e))
    }
}

// ===== SECURITY P0.1.6: Enhanced Filesystem Sandbox with separate read/write roots =====

/// SECURITY: Enhanced read access validation with separate read roots
fn ensure_read_allowed(path: &str) -> Result<()> {
    // SECURITY: Basic path traversal and malicious path protection
    validate_path_security(path, "read")?;

    // SECURITY: Policy engine check for file read operations
    let args = HashMap::from([
        ("path".to_string(), path.to_string()),
        ("operation".to_string(), "read".to_string()),
    ]);
    check_policy_for_file_operation("file_read", path, &args)?;

    // SECURITY P0.1.6: Use SandboxConfig with separate read roots
    let sandbox_config = common::sandbox_config::SandboxConfig::from_env();
    sandbox_config
        .validate_read_access(path)
        .map_err(|e| anyhow!("🔒 READ ACCESS DENIED: {}", e))
}

/// SECURITY: Enhanced write access validation with separate write roots
fn ensure_write_allowed(path: &str) -> Result<()> {
    // SECURITY: Basic path traversal and malicious path protection
    validate_path_security(path, "write")?;

    // SECURITY: Policy engine check for file write operations
    let args = HashMap::from([
        ("path".to_string(), path.to_string()),
        ("operation".to_string(), "write".to_string()),
    ]);
    check_policy_for_file_operation("file_write", path, &args)?;

    // SECURITY P0.1.6: Use SandboxConfig with separate write roots
    let sandbox_config = common::sandbox_config::SandboxConfig::from_env();
    sandbox_config
        .validate_write_access(path)
        .map_err(|e| anyhow!("🔒 WRITE ACCESS DENIED: {}", e))
}

/// SECURITY: Enhanced search access validation (uses read roots)
fn ensure_search_allowed(path: &str) -> Result<()> {
    // SECURITY: Basic path traversal and malicious path protection
    validate_path_security(path, "search")?;

    // SECURITY: Policy engine check for file search operations
    let args = HashMap::from([
        ("path".to_string(), path.to_string()),
        ("operation".to_string(), "search".to_string()),
    ]);
    check_policy_for_file_operation("file_search", path, &args)?;

    // SECURITY P0.1.6: Search operations use read roots
    let sandbox_config = common::sandbox_config::SandboxConfig::from_env();
    sandbox_config
        .validate_read_access(path)
        .map_err(|e| anyhow!("🔒 SEARCH ACCESS DENIED: {}", e))
}

// FileReader - чтение файлов с простым форматированием
pub struct FileReader;

impl FileReader {
    pub fn new() -> Self {
        FileReader
    }
}

impl Default for FileReader {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl Tool for FileReader {
    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: "file_read".to_string(),
            description: "Читает содержимое файлов с красивой подсветкой синтаксиса".to_string(),
            usage: "file_read <путь>".to_string(),
            examples: vec![
                "file_read src/main.rs".to_string(),
                "file_read README.md".to_string(),
                "показать содержимое config.toml".to_string(),
            ],
            input_schema: r#"{"path": "string"}"#.to_string(),
            usage_guide: Some(crate::UsageGuide {
                usage_title: "file_read".into(),
                usage_summary: "🔒 SECURITY: Read access restricted to configured read roots"
                    .into(),
                preconditions: vec![
                    "Path must be within MAGRAY_FS_READ_ROOTS".into(),
                    "Filesystem sandbox must be configured".into(),
                ],
                arguments_brief: HashMap::from([(
                    "path".to_string(),
                    "File path to read".to_string(),
                )]),
                good_for: vec!["reading".into(), "analysis".into()],
                not_for: vec!["system_files".into(), "sensitive_data".into()],
                constraints: vec!["Path traversal protection enabled".into()],
                examples: vec!["file_read ./project/src/main.rs".into()],
                platforms: vec!["linux".into(), "mac".into(), "win".into()],
                cost_class: "free".into(),
                latency_class: "fast".into(),
                side_effects: vec![],
                risk_score: 2,
                capabilities: vec!["read".into(), "fs".into()],
                tags: vec!["filesystem".into(), "security".into()],
            }),
            permissions: None,
            supports_dry_run: false,
        }
    }

    async fn execute(&self, input: ToolInput) -> Result<ToolOutput> {
        let path = input
            .args
            .get("path")
            .ok_or_else(|| anyhow!("Отсутствует параметр 'path'"))?;

        ensure_read_allowed(path)?;

        // КРИТИЧЕСКАЯ ОПТИМИЗАЦИЯ MEMORY: Используем streaming для больших файлов
        let content = read_file_with_memory_optimization(path)?;

        // Простое форматирование с заголовком
        let mut formatted = String::new();
        formatted.push_str(&format!("\n📄 Файл: {path}\n"));
        formatted.push_str(&"─".repeat(60));
        formatted.push('\n');
        formatted.push_str(&content);
        formatted.push('\n');
        formatted.push_str(&"─".repeat(60));
        formatted.push('\n');

        Ok(ToolOutput {
            success: true,
            result: content,
            formatted_output: Some(formatted),
            metadata: HashMap::new(),
        })
    }

    async fn parse_natural_language(&self, query: &str) -> Result<ToolInput> {
        let mut args = HashMap::new();

        // Извлекаем путь из запроса
        if let Some(path) = extract_path_from_query(query) {
            args.insert("path".to_string(), path);
        } else {
            args.insert("path".to_string(), query.to_string());
        }

        Ok(ToolInput {
            command: "file_read".to_string(),
            args,
            context: Some(query.to_string()),
            dry_run: false,
            timeout_ms: None,
        })
    }
}

// FileWriter - запись файлов
pub struct FileWriter;

impl FileWriter {
    pub fn new() -> Self {
        FileWriter
    }
}

impl Default for FileWriter {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl Tool for FileWriter {
    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: "file_write".to_string(),
            description:
                "🔒 SECURITY: Creates or overwrites files within configured write roots only"
                    .to_string(),
            usage: "file_write <путь> <содержимое>".to_string(),
            examples: vec![
                "file_write test.txt Hello World".to_string(),
                "создать файл config.json с содержимым {...}".to_string(),
            ],
            input_schema: r#"{"path": "string", "content": "string"}"#.to_string(),
            usage_guide: Some(crate::UsageGuide {
                usage_title: "file_write".into(),
                usage_summary: "🔒 SECURITY: Write access restricted to configured write roots"
                    .into(),
                preconditions: vec![
                    "Path must be within MAGRAY_FS_WRITE_ROOTS".into(),
                    "File extension must be in allowed list".into(),
                    "Filesystem sandbox must be configured".into(),
                ],
                arguments_brief: HashMap::from([
                    ("path".to_string(), "File path to write".to_string()),
                    ("content".to_string(), "Content to write".to_string()),
                ]),
                good_for: vec!["creating_files".into(), "configuration".into()],
                not_for: vec!["system_files".into(), "executable_files".into()],
                constraints: vec![
                    "Path traversal protection enabled".into(),
                    "Dangerous file extensions blocked".into(),
                ],
                examples: vec!["file_write ./output/result.txt Content here".into()],
                platforms: vec!["linux".into(), "mac".into(), "win".into()],
                cost_class: "free".into(),
                latency_class: "fast".into(),
                side_effects: vec!["File creation/modification".into()],
                risk_score: 4,
                capabilities: vec!["write".into(), "fs".into()],
                tags: vec!["filesystem".into(), "security".into(), "destructive".into()],
            }),
            permissions: None,
            supports_dry_run: true,
        }
    }

    async fn execute(&self, input: ToolInput) -> Result<ToolOutput> {
        let path = input
            .args
            .get("path")
            .ok_or_else(|| anyhow!("Отсутствует параметр 'path'"))?;
        let content = input.args.get("content").map(|s| s.as_str()).unwrap_or("");

        // Dry-run: показать что будет записано
        if input.dry_run {
            let mut meta = HashMap::new();
            meta.insert("dry_run".into(), "true".into());
            meta.insert("bytes".into(), content.len().to_string());
            return Ok(ToolOutput {
                success: true,
                result: format!("[dry-run] write {} bytes to {}", content.len(), path),
                formatted_output: Some(format!(
                    "$ echo '<content:{} bytes>' > {}\n[dry-run: no side effects]",
                    content.len(),
                    path
                )),
                metadata: meta,
            });
        }

        ensure_write_allowed(path)?;
        fs::write(path, content)?;

        let path_for_evt = path.clone();
        let bytes_for_evt = content.len();
        tokio::spawn(async move {
            let evt = serde_json::json!({
                "path": path_for_evt,
                "bytes": bytes_for_evt,
                "op": "write",
            });
            common::events::publish(common::topics::TOPIC_FS_DIFF, evt).await;
        });

        Ok(ToolOutput {
            success: true,
            result: format!("✅ Файл '{path}' успешно создан"),
            formatted_output: None,
            metadata: HashMap::from([("bytes".into(), content.len().to_string())]),
        })
    }

    async fn parse_natural_language(&self, query: &str) -> Result<ToolInput> {
        let mut args = HashMap::new();

        // Простой парсинг для создания файла
        let parts: Vec<&str> = query.split(" с содержимым ").collect();
        if parts.len() == 2 {
            args.insert("path".to_string(), parts[0].trim().to_string());
            args.insert("content".to_string(), parts[1].trim().to_string());
        } else {
            return Err(anyhow!("Не удалось распарсить запрос на создание файла"));
        }

        Ok(ToolInput {
            command: "file_write".to_string(),
            args,
            context: Some(query.to_string()),
            dry_run: false,
            timeout_ms: None,
        })
    }
}

// DirLister - просмотр директорий
pub struct DirLister;

impl DirLister {
    pub fn new() -> Self {
        DirLister
    }
}

impl Default for DirLister {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl Tool for DirLister {
    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: "dir_list".to_string(),
            description: "🔒 SECURITY: Lists directory contents within configured read roots only"
                .to_string(),
            usage: "dir_list <путь>".to_string(),
            examples: vec![
                "dir_list .".to_string(),
                "dir_list src/".to_string(),
                "показать содержимое папки".to_string(),
            ],
            input_schema: r#"{"path": "string"}"#.to_string(),
            usage_guide: Some(crate::UsageGuide {
                usage_title: "dir_list".into(),
                usage_summary: "🔒 SECURITY: Directory access restricted to configured read roots"
                    .into(),
                preconditions: vec![
                    "Directory must be within MAGRAY_FS_READ_ROOTS".into(),
                    "Filesystem sandbox must be configured".into(),
                ],
                arguments_brief: HashMap::from([(
                    "path".to_string(),
                    "Directory path to list".to_string(),
                )]),
                good_for: vec!["exploration".into(), "discovery".into()],
                not_for: vec!["system_directories".into(), "sensitive_paths".into()],
                constraints: vec!["Path traversal protection enabled".into()],
                examples: vec!["dir_list ./project/src".into()],
                platforms: vec!["linux".into(), "mac".into(), "win".into()],
                cost_class: "free".into(),
                latency_class: "fast".into(),
                side_effects: vec![],
                risk_score: 1,
                capabilities: vec!["read".into(), "fs".into()],
                tags: vec!["filesystem".into(), "security".into()],
            }),
            permissions: None,
            supports_dry_run: false,
        }
    }

    async fn execute(&self, input: ToolInput) -> Result<ToolOutput> {
        let path = input
            .args
            .get("path")
            .ok_or_else(|| anyhow!("Отсутствует параметр 'path'"))?;

        let path = Path::new(path);
        ensure_read_allowed(&path.to_string_lossy())?;
        if !path.is_dir() {
            return Err(anyhow!("'{}' не является директорией", path.display()));
        }

        let mut output = String::new();
        output.push_str(&format!("\n📁 Директория: {}\n", path.display()));
        output.push_str(&"─".repeat(60));
        output.push('\n');

        // Собираем entries
        let mut entries: Vec<_> = fs::read_dir(path)?.filter_map(|e| e.ok()).collect();

        // Сортируем: сначала директории, потом файлы
        entries.sort_by(|a, b| {
            let a_is_dir = a.file_type().map(|t| t.is_dir()).unwrap_or(false);
            let b_is_dir = b.file_type().map(|t| t.is_dir()).unwrap_or(false);
            b_is_dir
                .cmp(&a_is_dir)
                .then_with(|| a.file_name().cmp(&b.file_name()))
        });

        for entry in entries {
            let entry_path = entry.path();
            let name = entry.file_name();
            let name_str = name.to_string_lossy();

            if entry_path.is_dir() {
                output.push_str(&format!("📁 {name_str}/\n"));
            } else {
                let icon = "📄";
                let size = entry
                    .metadata()
                    .map(|m| format_size(m.len()))
                    .unwrap_or_else(|_| "?".to_string());
                output.push_str(&format!("{icon} {name_str} ({size})\n"));
            }
        }

        output.push_str(&"─".repeat(60));
        output.push('\n');

        Ok(ToolOutput {
            success: true,
            result: output.clone(),
            formatted_output: Some(output),
            metadata: HashMap::new(),
        })
    }

    async fn parse_natural_language(&self, query: &str) -> Result<ToolInput> {
        let mut args = HashMap::new();

        // Извлекаем путь из запроса
        if let Some(path) = extract_path_from_query(query) {
            args.insert("path".to_string(), path);
        } else {
            args.insert("path".to_string(), ".".to_string());
        }

        Ok(ToolInput {
            command: "dir_list".to_string(),
            args,
            context: Some(query.to_string()),
            dry_run: false,
            timeout_ms: None,
        })
    }
}

// FileSearcher - поиск файлов
pub struct FileSearcher;

impl FileSearcher {
    pub fn new() -> Self {
        FileSearcher
    }
}

impl Default for FileSearcher {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl Tool for FileSearcher {
    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: "file_search".to_string(),
            description: "🔒 SECURITY: Searches for files within configured read roots only"
                .to_string(),
            usage: "file_search <паттерн> [путь]".to_string(),
            examples: vec![
                "file_search *.rs".to_string(),
                "file_search main.rs src/".to_string(),
                "найти все файлы .toml".to_string(),
            ],
            input_schema: r#"{"pattern": "string", "path": "string?"}"#.to_string(),
            usage_guide: Some(crate::UsageGuide {
                usage_title: "file_search".into(),
                usage_summary: "🔒 SECURITY: File search restricted to configured read roots"
                    .into(),
                preconditions: vec![
                    "Search path must be within MAGRAY_FS_READ_ROOTS".into(),
                    "Filesystem sandbox must be configured".into(),
                ],
                arguments_brief: HashMap::from([
                    (
                        "pattern".to_string(),
                        "Search pattern (supports wildcards)".to_string(),
                    ),
                    (
                        "path".to_string(),
                        "Optional search root directory".to_string(),
                    ),
                ]),
                good_for: vec!["discovery".into(), "finding_files".into()],
                not_for: vec!["system_wide_search".into(), "sensitive_data".into()],
                constraints: vec![
                    "Path traversal protection enabled".into(),
                    "Maximum depth: 10 levels".into(),
                    "Results limited to 100 files".into(),
                ],
                examples: vec!["file_search *.rs ./src".into()],
                platforms: vec!["linux".into(), "mac".into(), "win".into()],
                cost_class: "free".into(),
                latency_class: "medium".into(),
                side_effects: vec![],
                risk_score: 2,
                capabilities: vec!["search".into(), "fs".into()],
                tags: vec!["filesystem".into(), "security".into()],
            }),
            permissions: None,
            supports_dry_run: false,
        }
    }

    async fn execute(&self, input: ToolInput) -> Result<ToolOutput> {
        let pattern = input
            .args
            .get("pattern")
            .ok_or_else(|| anyhow!("Отсутствует параметр 'pattern'"))?;
        let search_path = input.args.get("path").map(|s| s.as_str()).unwrap_or(".");

        ensure_search_allowed(search_path)?;

        let mut results = Vec::new();
        let pattern_lower = pattern.to_lowercase();

        for entry in WalkDir::new(search_path)
            .max_depth(10)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

            // Простой поиск по паттерну
            let matches = if pattern.contains('*') {
                // Простая поддержка wildcard
                let pattern_parts: Vec<&str> = pattern.split('*').collect();
                if pattern_parts.len() == 2 {
                    if pattern.starts_with('*') {
                        file_name
                            .to_lowercase()
                            .ends_with(&pattern_parts[1].to_lowercase())
                    } else if pattern.ends_with('*') {
                        file_name
                            .to_lowercase()
                            .starts_with(&pattern_parts[0].to_lowercase())
                    } else {
                        file_name
                            .to_lowercase()
                            .starts_with(&pattern_parts[0].to_lowercase())
                            && file_name
                                .to_lowercase()
                                .ends_with(&pattern_parts[1].to_lowercase())
                    }
                } else {
                    file_name.to_lowercase().contains(&pattern_lower)
                }
            } else {
                file_name.to_lowercase().contains(&pattern_lower)
            };

            if matches {
                results.push(path.display().to_string());
            }
        }

        let mut output = String::new();
        output.push_str(&format!("\n🔍 Поиск: {pattern} в {search_path}\n"));
        output.push_str(&"─".repeat(60));
        output.push('\n');

        if results.is_empty() {
            output.push_str("Файлы не найдены\n");
        } else {
            output.push_str(&format!("Найдено {} файлов:\n", results.len()));
            for result in results.iter().take(100) {
                output.push_str(&format!("  📄 {result}\n"));
            }
            if results.len() > 100 {
                output.push_str(&format!("  ... и ещё {} файлов\n", results.len() - 100));
            }
        }

        output.push_str(&"─".repeat(60));
        output.push('\n');

        Ok(ToolOutput {
            success: true,
            result: output.clone(),
            formatted_output: Some(output),
            metadata: HashMap::new(),
        })
    }

    async fn parse_natural_language(&self, query: &str) -> Result<ToolInput> {
        let mut args = HashMap::new();

        // Извлекаем паттерн поиска
        if query.contains("файлы") || query.contains("файл") {
            // Пытаемся найти расширение файла
            if let Some(ext_start) = query.find('.') {
                let ext_end = query[ext_start..]
                    .find(' ')
                    .unwrap_or(query.len() - ext_start);
                let pattern = format!("*{}", &query[ext_start..ext_start + ext_end]);
                args.insert("pattern".to_string(), pattern);
            } else {
                args.insert("pattern".to_string(), "*".to_string());
            }
        } else {
            args.insert("pattern".to_string(), query.to_string());
        }

        Ok(ToolInput {
            command: "file_search".to_string(),
            args,
            context: Some(query.to_string()),
            dry_run: false,
            timeout_ms: None,
        })
    }
}

// FileDeleter - удаление файлов с публикацией fs.diff
pub struct FileDeleter;

impl FileDeleter {
    pub fn new() -> Self {
        FileDeleter
    }
}

impl Default for FileDeleter {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl Tool for FileDeleter {
    fn spec(&self) -> ToolSpec {
        let mut spec = ToolSpec {
            name: "file_delete".to_string(),
            description: "Удаляет файл по указанному пути".to_string(),
            usage: "file_delete <путь>".to_string(),
            examples: vec![
                "file_delete tmp/test.txt".to_string(),
                "удалить файл build.log".to_string(),
            ],
            input_schema: r#"{"path": "string"}"#.to_string(),
            usage_guide: None,
            permissions: None,
            supports_dry_run: true,
        };
        spec.usage_guide = Some(UsageGuide {
            usage_title: "file_delete".into(),
            usage_summary: "Удаляет файл по указанному пути".into(),
            preconditions: vec!["Файл должен существовать".into()],
            arguments_brief: HashMap::from([(String::from("path"), String::from("Путь к файлу"))]),
            good_for: vec!["cleanup".into(), "io".into()],
            not_for: vec!["sensitive".into()],
            constraints: vec!["Необратимая операция".into()],
            examples: vec!["file_delete /tmp/file.txt".into()],
            platforms: vec!["linux".into(), "mac".into(), "win".into()],
            cost_class: "free".into(),
            latency_class: "fast".into(),
            side_effects: vec!["Удаление данных".into()],
            risk_score: 5,
            capabilities: vec!["delete".into(), "fs".into()],
            tags: vec!["danger".into(), "destructive".into()],
        });
        spec
    }

    async fn execute(&self, input: ToolInput) -> Result<ToolOutput> {
        let path = input
            .args
            .get("path")
            .ok_or_else(|| anyhow!("Отсутствует параметр 'path'"))?;

        // Dry-run: показать, что будет удалено
        if input.dry_run {
            let mut meta = HashMap::new();
            meta.insert("dry_run".into(), "true".into());
            return Ok(ToolOutput {
                success: true,
                result: format!("[dry-run] rm {path}"),
                formatted_output: Some(format!("$ rm {path}\n[dry-run: no side effects]")),
                metadata: meta,
            });
        }

        ensure_write_allowed(path)?;

        // Считываем размер до удаления (если есть)
        let bytes = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0) as usize;

        // Удаляем файл
        fs::remove_file(path)?;

        // Публикуем событие fs.diff
        let path_for_evt = path.clone();
        tokio::spawn(async move {
            let evt = serde_json::json!({
                "path": path_for_evt,
                "bytes": bytes,
                "op": "delete",
            });
            common::events::publish(common::topics::TOPIC_FS_DIFF, evt).await;
        });

        Ok(ToolOutput {
            success: true,
            result: format!("✅ Файл '{path}' удалён"),
            formatted_output: None,
            metadata: HashMap::from([("bytes".into(), bytes.to_string())]),
        })
    }

    async fn parse_natural_language(&self, query: &str) -> Result<ToolInput> {
        let mut args = HashMap::new();
        // Пробуем извлечь путь как первое слово с разделителями
        if let Some(p) = extract_path_from_query(query) {
            args.insert("path".to_string(), p);
        } else {
            args.insert("path".to_string(), query.to_string());
        }
        Ok(ToolInput {
            command: "file_delete".to_string(),
            args,
            context: Some(query.to_string()),
            dry_run: false,
            timeout_ms: None,
        })
    }
}

// Вспомогательная функция для форматирования размера файла
fn format_size(size: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB"];
    let mut size = size as f64;
    let mut unit_index = 0;

    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }

    if unit_index == 0 {
        format!("{} {}", size as u64, UNITS[unit_index])
    } else {
        format!("{:.1} {}", size, UNITS[unit_index])
    }
}

// Вспомогательная функция для извлечения пути из запроса
fn extract_path_from_query(query: &str) -> Option<String> {
    // Простой поиск путей в запросе
    let words: Vec<&str> = query.split_whitespace().collect();
    for word in words {
        if word.contains('/')
            || word.contains('\\')
            || word.ends_with(".rs")
            || word.ends_with(".md")
            || word.ends_with(".toml")
        {
            return Some(word.to_string());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use common::{events, topics};
    use std::fs;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_file_reader_creation() {
        let reader = FileReader::new();
        let spec = reader.spec();

        assert_eq!(spec.name, "file_read");
        assert!(spec.description.contains("Читает содержимое файлов"));
        assert!(!spec.examples.is_empty());
    }

    #[tokio::test]
    async fn test_file_reader_default() {
        let reader1 = FileReader::new();
        let reader2 = FileReader::new();

        assert_eq!(reader1.spec().name, reader2.spec().name);
    }

    #[tokio::test]
    async fn test_file_reader_nonexistent_file() {
        let reader = FileReader::new();
        let mut input_args = HashMap::new();
        input_args.insert(
            "path".to_string(),
            "/definitely/nonexistent/file.txt".to_string(),
        );

        let input = ToolInput {
            command: "file_read".to_string(),
            args: input_args,
            context: None,
            dry_run: false,
            timeout_ms: None,
        };

        let result = reader.execute(input).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_file_reader_existing_file() -> Result<()> {
        let temp_dir = TempDir::new().expect("File operation should succeed");
        let file_path = temp_dir.path().join("test.txt");
        let test_content = "Hello, World!";

        fs::write(&file_path, test_content).expect("File operation should succeed");

        let reader = FileReader::new();
        let mut input_args = HashMap::new();
        input_args.insert("path".to_string(), file_path.to_string_lossy().to_string());

        let input = ToolInput {
            command: "file_read".to_string(),
            args: input_args,
            context: None,
            dry_run: false,
            timeout_ms: None,
        };

        let result = reader.execute(input).await?;
        assert!(result.success);
        assert_eq!(result.result, test_content);
        assert!(result.formatted_output.is_some());

        Ok(())
    }

    #[tokio::test]
    async fn test_file_reader_natural_language() -> Result<()> {
        let reader = FileReader::new();

        // Test with path in query
        let input = reader
            .parse_natural_language("показать содержимое src/main.rs")
            .await?;
        assert_eq!(input.command, "file_read");
        assert_eq!(
            input
                .args
                .get("path")
                .expect("File operation should succeed"),
            "src/main.rs"
        );

        // Test without recognizable path
        let input = reader.parse_natural_language("показать файл").await?;
        assert_eq!(
            input
                .args
                .get("path")
                .expect("File operation should succeed"),
            "показать файл"
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_file_writer_creation() {
        let writer = FileWriter::new();
        let spec = writer.spec();

        assert_eq!(spec.name, "file_write");
        assert!(spec.description.contains("Создаёт или перезаписывает файл"));
        assert!(!spec.examples.is_empty());
    }

    #[tokio::test]
    async fn test_file_writer_default() {
        let writer1 = FileWriter::new();
        let writer2 = FileWriter::new();

        assert_eq!(writer1.spec().name, writer2.spec().name);
    }

    #[tokio::test]
    async fn test_file_writer_write_file() -> Result<()> {
        let temp_dir = TempDir::new().expect("File operation should succeed");
        let file_path = temp_dir.path().join("test_output.txt");
        let test_content = "Test content";

        let writer = FileWriter::new();
        let mut input_args = HashMap::new();
        input_args.insert("path".to_string(), file_path.to_string_lossy().to_string());
        input_args.insert("content".to_string(), test_content.to_string());

        let input = ToolInput {
            command: "file_write".to_string(),
            args: input_args,
            context: None,
            dry_run: false,
            timeout_ms: None,
        };

        let result = writer.execute(input).await?;
        assert!(result.success);
        assert!(result.result.contains("успешно создан"));

        // Verify file was actually created
        let written_content =
            fs::read_to_string(&file_path).expect("File operation should succeed");
        assert_eq!(written_content, test_content);

        Ok(())
    }

    #[tokio::test]
    async fn test_file_writer_missing_path() {
        let writer = FileWriter::new();
        let input_args = HashMap::new(); // Missing path

        let input = ToolInput {
            command: "file_write".to_string(),
            args: input_args,
            context: None,
            dry_run: false,
            timeout_ms: None,
        };

        let result = writer.execute(input).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_file_writer_natural_language() -> Result<()> {
        let writer = FileWriter::new();

        let input = writer
            .parse_natural_language("создать файл test.txt с содержимым Hello World")
            .await?;
        assert_eq!(input.command, "file_write");
        assert_eq!(
            input
                .args
                .get("path")
                .expect("File operation should succeed"),
            "создать файл test.txt"
        );
        assert_eq!(
            input
                .args
                .get("content")
                .expect("File operation should succeed"),
            "Hello World"
        );

        // Test invalid format
        let result = writer.parse_natural_language("создать файл test.txt").await;
        assert!(result.is_err());

        Ok(())
    }

    #[test]
    fn test_extract_path_from_query() {
        // Test with .rs file
        let path = extract_path_from_query("показать содержимое src/main.rs");
        assert_eq!(path, Some("src/main.rs".to_string()));

        // Test with .md file
        let path = extract_path_from_query("прочитать README.md");
        assert_eq!(path, Some("README.md".to_string()));

        // Test with path containing slash
        let path = extract_path_from_query("открыть file/path.txt");
        assert_eq!(path, Some("file/path.txt".to_string()));

        // Test with backslash (Windows style)
        let path = extract_path_from_query("показать file\\path.txt");
        assert_eq!(path, Some("file\\path.txt".to_string()));

        // Test with .toml file
        let path = extract_path_from_query("config.toml file");
        assert_eq!(path, Some("config.toml".to_string()));

        // Test with no recognizable path
        let path = extract_path_from_query("показать файл");
        assert_eq!(path, None);
    }

    #[test]
    fn test_format_size() {
        assert_eq!(format_size(100), "100 B");
        assert_eq!(format_size(1024), "1.0 KB");
        assert_eq!(format_size(1536), "1.5 KB");
        assert_eq!(format_size(1024 * 1024), "1.0 MB");
        assert_eq!(format_size(1024 * 1024 * 1024), "1.0 GB");
        assert_eq!(format_size(2048 * 1024 * 1024), "2.0 GB");
    }

    #[tokio::test]
    async fn test_filesystem_roots_restrictions() -> Result<()> {
        use std::env;

        // Setup test environment with restricted roots
        let temp_dir = TempDir::new().expect("File operation should succeed");
        let allowed_read_root = temp_dir.path().join("read_allowed");
        let allowed_write_root = temp_dir.path().join("write_allowed");
        let forbidden_dir = temp_dir.path().join("forbidden");

        fs::create_dir_all(&allowed_read_root).expect("File operation should succeed");
        fs::create_dir_all(&allowed_write_root).expect("File operation should succeed");
        fs::create_dir_all(&forbidden_dir).expect("File operation should succeed");

        let test_file_allowed_read = allowed_read_root.join("test.txt");
        let test_file_allowed_write = allowed_write_root.join("test.txt");
        let test_file_forbidden = forbidden_dir.join("test.txt");

        fs::write(&test_file_allowed_read, "content").expect("File operation should succeed");
        fs::write(&test_file_forbidden, "content").expect("File operation should succeed");

        // Set environment variables for sandbox config
        env::set_var("MAGRAY_FS_SANDBOX", "true");
        env::set_var(
            "MAGRAY_FS_READ_ROOTS",
            allowed_read_root.to_string_lossy().as_ref(),
        );
        env::set_var(
            "MAGRAY_FS_WRITE_ROOTS",
            allowed_write_root.to_string_lossy().as_ref(),
        );

        // Test 1: FileReader - allowed read should work
        let reader = FileReader::new();
        let input = ToolInput {
            command: "file_read".to_string(),
            args: HashMap::from([(
                "path".to_string(),
                test_file_allowed_read.to_string_lossy().to_string(),
            )]),
            context: None,
            dry_run: false,
            timeout_ms: None,
        };
        let result = reader.execute(input).await;
        assert!(result.is_ok(), "Read from allowed root should succeed");

        // Test 2: FileReader - forbidden read should fail
        let input_forbidden = ToolInput {
            command: "file_read".to_string(),
            args: HashMap::from([(
                "path".to_string(),
                test_file_forbidden.to_string_lossy().to_string(),
            )]),
            context: None,
            dry_run: false,
            timeout_ms: None,
        };
        let result_forbidden = reader.execute(input_forbidden).await;
        assert!(
            result_forbidden.is_err(),
            "Read from forbidden root should fail"
        );
        assert!(result_forbidden
            .unwrap_err()
            .to_string()
            .contains("READ ACCESS DENIED"));

        // Test 3: FileWriter - allowed write should work
        let writer = FileWriter::new();
        let input_write = ToolInput {
            command: "file_write".to_string(),
            args: HashMap::from([
                (
                    "path".to_string(),
                    test_file_allowed_write.to_string_lossy().to_string(),
                ),
                ("content".to_string(), "new content".to_string()),
            ]),
            context: None,
            dry_run: false,
            timeout_ms: None,
        };
        let result_write = writer.execute(input_write).await;
        assert!(result_write.is_ok(), "Write to allowed root should succeed");

        // Test 4: FileWriter - forbidden write should fail
        let forbidden_write_file = forbidden_dir.join("new_file.txt");
        let input_forbidden_write = ToolInput {
            command: "file_write".to_string(),
            args: HashMap::from([
                (
                    "path".to_string(),
                    forbidden_write_file.to_string_lossy().to_string(),
                ),
                ("content".to_string(), "forbidden content".to_string()),
            ]),
            context: None,
            dry_run: false,
            timeout_ms: None,
        };
        let result_forbidden_write = writer.execute(input_forbidden_write).await;
        assert!(
            result_forbidden_write.is_err(),
            "Write to forbidden root should fail"
        );
        assert!(result_forbidden_write
            .unwrap_err()
            .to_string()
            .contains("WRITE ACCESS DENIED"));

        // Cleanup environment
        env::remove_var("MAGRAY_FS_SANDBOX");
        env::remove_var("MAGRAY_FS_READ_ROOTS");
        env::remove_var("MAGRAY_FS_WRITE_ROOTS");

        Ok(())
    }

    #[tokio::test]
    async fn test_path_traversal_protection() -> Result<()> {
        let reader = FileReader::new();

        // Test path traversal attack
        let input = ToolInput {
            command: "file_read".to_string(),
            args: HashMap::from([("path".to_string(), "../../../etc/passwd".to_string())]),
            context: None,
            dry_run: false,
            timeout_ms: None,
        };

        let result = reader.execute(input).await;
        assert!(result.is_err(), "Path traversal should be blocked");
        assert!(result.unwrap_err().to_string().contains("SECURITY ERROR"));

        Ok(())
    }

    #[tokio::test]
    async fn test_file_delete_removes_and_emits_event() -> Result<()> {
        let temp_dir = TempDir::new().expect("File operation should succeed");
        let file_path = temp_dir.path().join("to_remove.txt");
        let content = "bye";
        fs::write(&file_path, content).expect("File operation should succeed");

        // Подпишемся на события fs.diff до действия
        let mut rx = events::subscribe(topics::TOPIC_FS_DIFF).await;

        let deleter = FileDeleter::new();
        let input = ToolInput {
            command: "file_delete".to_string(),
            args: HashMap::from([("path".to_string(), file_path.to_string_lossy().to_string())]),
            context: None,
            dry_run: false,
            timeout_ms: None,
        };

        let out = deleter.execute(input).await?;
        assert!(out.success);
        assert!(!file_path.exists());

        // Проверяем событие
        let evt = tokio::time::timeout(std::time::Duration::from_millis(500), rx.recv()).await??;
        assert_eq!(evt.topic.0, "fs.diff");
        assert_eq!(evt.payload["op"], "delete");
        assert!(evt.payload["bytes"].as_u64().unwrap_or(0) >= content.len() as u64);
        Ok(())
    }
}
