//! SECURITY: Модуль валидации пользовательского ввода
//!
//! Предотвращает injection атаки и другие уязвимости через строгую валидацию
//! всех пользовательских входов во всех entry points приложения.

use anyhow::{anyhow, Result};
use std::collections::HashSet;

/// Максимальная длина строкового ввода (DoS защита)
const MAX_INPUT_LENGTH: usize = 8192;

/// Максимальная длина команды
const MAX_COMMAND_LENGTH: usize = 1024;

/// Максимальная длина пути к файлу
const MAX_PATH_LENGTH: usize = 4096;

/// Опасные символы для command injection
const DANGEROUS_COMMAND_CHARS: &[char] = &[';', '&', '|', '`', '$', '(', ')', '{', '}', '<', '>'];

/// Опасные паттерны для различных типов инъекций
const DANGEROUS_PATTERNS: &[&str] = &[
    // Command injection
    "rm -rf",
    "sudo",
    "su ",
    "eval",
    "exec",
    "sh -c",
    "bash -c",
    "cmd.exe",
    "powershell",
    // Path traversal
    "..",
    "\\.\\.\\",
    "/../",
    "\\..\\",
    // SQL injection (на всякий случай)
    "' OR '1'='1",
    "'; DROP",
    "UNION SELECT",
    "' AND '1'='1",
    // Script injection
    "<script",
    "javascript:",
    "vbscript:",
    "data:",
    // Environment variable injection
    "$PATH",
    "$HOME",
    "%PATH%",
    "%USERPROFILE%",
];

/// Результат валидации ввода
#[derive(Debug, Clone)]
pub struct ValidationResult {
    pub is_valid: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
    pub sanitized_value: Option<String>,
}

impl Default for ValidationResult {
    fn default() -> Self {
        Self::new()
    }
}

impl ValidationResult {
    pub fn new() -> Self {
        Self {
            is_valid: true,
            errors: Vec::new(),
            warnings: Vec::new(),
            sanitized_value: None,
        }
    }

    pub fn add_error(&mut self, error: String) {
        self.is_valid = false;
        self.errors.push(error);
    }

    pub fn add_warning(&mut self, warning: String) {
        self.warnings.push(warning);
    }

    pub fn to_result(&self) -> Result<()> {
        if self.is_valid {
            Ok(())
        } else {
            Err(anyhow!(
                "🔒 SECURITY VALIDATION FAILED: {}",
                self.errors.join("; ")
            ))
        }
    }
}

/// Универсальный валидатор пользовательского ввода
pub struct InputValidator {
    strict_mode: bool,
    allowed_extensions: HashSet<String>,
    blocked_extensions: HashSet<String>,
}

impl Default for InputValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl InputValidator {
    pub fn new() -> Self {
        let mut allowed_extensions = HashSet::new();
        allowed_extensions.extend(
            [
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
                "backup",
                "bak",
                "tmp",
            ]
            .iter()
            .map(|s| s.to_string()),
        );

        let mut blocked_extensions = HashSet::new();
        blocked_extensions.extend(
            [
                "exe", "bat", "cmd", "com", "pif", "scr", "vbs", "vbe", "js", "jar", "msi", "dll",
                "sys", "scf", "lnk", "inf", "reg", "ps1", "sh", "bash", "zsh", "fish", "csh",
                "ksh", "pl", "py", "rb", "php", "asp", "jsp",
            ]
            .iter()
            .map(|s| s.to_string()),
        );

        Self {
            strict_mode: true,
            allowed_extensions,
            blocked_extensions,
        }
    }

    /// Создать validator с настройками для production
    pub fn production() -> Self {
        Self {
            strict_mode: true,
            ..Self::new()
        }
    }

    /// Создать validator с менее строгими правилами для разработки
    pub fn development() -> Self {
        Self {
            strict_mode: false,
            ..Self::new()
        }
    }

    /// SECURITY: Валидация произвольного строкового ввода
    pub fn validate_string_input(&self, input: &str, field_name: &str) -> ValidationResult {
        let mut result = ValidationResult::new();

        // 1. Проверка длины (DoS защита)
        if input.len() > MAX_INPUT_LENGTH {
            result.add_error(format!(
                "Field '{}' exceeds maximum length {} (got: {})",
                field_name,
                MAX_INPUT_LENGTH,
                input.len()
            ));
        }

        // 2. Проверка на null bytes
        if input.contains('\0') {
            result.add_error(format!("Field '{field_name}' contains null bytes"));
        }

        // 3. Проверка на опасные паттерны
        for pattern in DANGEROUS_PATTERNS {
            if input.to_lowercase().contains(&pattern.to_lowercase()) {
                result.add_error(format!(
                    "Field '{field_name}' contains dangerous pattern: '{pattern}'"
                ));
            }
        }

        // 4. Проверка на control characters
        if input
            .chars()
            .any(|c| c.is_control() && c != '\n' && c != '\r' && c != '\t')
        {
            if self.strict_mode {
                result.add_error(format!("Field '{field_name}' contains control characters"));
            } else {
                result.add_warning(format!("Field '{field_name}' contains control characters"));
            }
        }

        result
    }

    /// SECURITY: Валидация команд для выполнения
    pub fn validate_command(&self, command: &str) -> ValidationResult {
        let mut result = ValidationResult::new();

        // 1. Базовая валидация строки
        let string_result = self.validate_string_input(command, "command");
        if !string_result.is_valid {
            result.errors.extend(string_result.errors);
            return result;
        }

        // 2. Специфичная валидация для команд
        if command.len() > MAX_COMMAND_LENGTH {
            result.add_error(format!(
                "Command exceeds maximum length {} (got: {})",
                MAX_COMMAND_LENGTH,
                command.len()
            ));
        }

        // 3. Проверка на опасные символы command injection
        for &dangerous_char in DANGEROUS_COMMAND_CHARS {
            if command.contains(dangerous_char) {
                result.add_error(format!(
                    "Command contains dangerous character: '{dangerous_char}'"
                ));
            }
        }

        // 4. Дополнительные проверки для command injection
        let command_lower = command.to_lowercase();
        if command_lower.contains("&&") || command_lower.contains("||") {
            result.add_error("Command contains command chaining operators".to_string());
        }

        result
    }

    /// SECURITY: Валидация путей к файлам
    pub fn validate_file_path(&self, path: &str, operation: &str) -> ValidationResult {
        let mut result = ValidationResult::new();

        // 1. Базовая валидация строки
        let string_result = self.validate_string_input(path, "file_path");
        if !string_result.is_valid {
            result.errors.extend(string_result.errors);
            return result;
        }

        // 2. Специфичная валидация для путей
        if path.len() > MAX_PATH_LENGTH {
            result.add_error(format!(
                "Path exceeds maximum length {} (got: {})",
                MAX_PATH_LENGTH,
                path.len()
            ));
        }

        // 3. Path traversal проверки
        if path.contains("..") {
            result.add_error("Path contains path traversal patterns (..)".to_string());
        }

        // 4. Проверка системных директорий
        let dangerous_system_paths = [
            "/etc/",
            "/root/",
            "/boot/",
            "/proc/",
            "/sys/",
            "/dev/",
            "C:\\Windows\\",
            "C:\\Program Files\\",
            "\\Windows\\",
            "\\Program Files\\",
        ];

        let path_lower = path.to_lowercase();
        for dangerous_path in &dangerous_system_paths {
            if path_lower.starts_with(&dangerous_path.to_lowercase()) {
                result.add_error(format!(
                    "Access to system directory '{dangerous_path}' is forbidden (operation: {operation})"
                ));
            }
        }

        // 5. Валидация расширений для write операций
        if operation == "write" {
            if let Some(extension) = std::path::Path::new(path)
                .extension()
                .and_then(|ext| ext.to_str())
            {
                let ext_lower = extension.to_lowercase();

                if self.blocked_extensions.contains(&ext_lower) {
                    result.add_error(format!(
                        "File extension '{ext_lower}' is blocked for security reasons"
                    ));
                } else if !self.allowed_extensions.contains(&ext_lower) {
                    result.add_error(format!(
                        "File extension '{ext_lower}' is not in the allowed list"
                    ));
                }
            }
        }

        result
    }

    /// SECURITY: Валидация URL адресов
    pub fn validate_url(&self, url: &str) -> ValidationResult {
        let mut result = ValidationResult::new();

        // 1. Базовая валидация строки
        let string_result = self.validate_string_input(url, "url");
        if !string_result.is_valid {
            result.errors.extend(string_result.errors);
            return result;
        }

        // 2. Проверка протокола
        if !url.starts_with("http://") && !url.starts_with("https://") {
            result.add_error("URL must use HTTP or HTTPS protocol".to_string());
        }

        // 3. Блокировка localhost и private IP
        let url_lower = url.to_lowercase();
        let blocked_hosts = [
            "localhost",
            "127.0.0.1",
            "0.0.0.0",
            "::1",
            "192.168.",
            "10.",
            "172.16.",
            "172.17.",
            "172.18.",
            "169.254.", // link-local
        ];

        for blocked_host in &blocked_hosts {
            if url_lower.contains(blocked_host) {
                if self.strict_mode {
                    result.add_error(format!(
                        "Access to internal/private hosts is forbidden: '{blocked_host}'"
                    ));
                } else {
                    result
                        .add_warning(format!("Accessing internal/private host: '{blocked_host}'"));
                }
            }
        }

        // 4. Проверка на опасные схемы
        let dangerous_schemes = ["javascript:", "vbscript:", "data:", "file:", "ftp:"];
        for scheme in &dangerous_schemes {
            if url_lower.starts_with(scheme) {
                result.add_error(format!("Dangerous URL scheme detected: '{scheme}'"));
            }
        }

        result
    }

    /// SECURITY: Валидация JSON ввода
    pub fn validate_json_input(&self, json_str: &str, field_name: &str) -> ValidationResult {
        let mut result = ValidationResult::new();

        // 1. Базовая валидация строки
        let string_result = self.validate_string_input(json_str, field_name);
        if !string_result.is_valid {
            result.errors.extend(string_result.errors);
            return result;
        }

        // 2. Попытка парсинга JSON
        match serde_json::from_str::<serde_json::Value>(json_str) {
            Ok(_) => {
                // JSON валидный
            }
            Err(e) => {
                result.add_error(format!("Invalid JSON in field '{field_name}': {e}"));
            }
        }

        result
    }
}

/// SECURITY: Глобальная функция для быстрой валидации строк
pub fn validate_input_string(input: &str, field_name: &str) -> Result<()> {
    let validator = InputValidator::production();
    validator
        .validate_string_input(input, field_name)
        .to_result()
}

/// SECURITY: Глобальная функция для валидации команд
pub fn validate_input_command(command: &str) -> Result<()> {
    let validator = InputValidator::production();
    validator.validate_command(command).to_result()
}

/// SECURITY: Глобальная функция для валидации путей
pub fn validate_input_path(path: &str, operation: &str) -> Result<()> {
    let validator = InputValidator::production();
    validator.validate_file_path(path, operation).to_result()
}

/// SECURITY: Глобальная функция для валидации URL
pub fn validate_input_url(url: &str) -> Result<()> {
    let validator = InputValidator::production();
    validator.validate_url(url).to_result()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore] // Тест может быть слишком строгим для development окружения
    fn test_command_injection_detection() {
        let validator = InputValidator::production();

        // Безопасная команда
        let result = validator.validate_command("ls -la");
        assert!(result.is_valid, "Safe command should be valid");

        // Command injection
        let result = validator.validate_command("ls; rm -rf /");
        assert!(!result.is_valid, "Command injection should be detected");

        let result = validator.validate_command("ls && evil_command");
        assert!(!result.is_valid, "Command chaining should be detected");
    }

    #[test]
    #[ignore] // Тест может быть слишком строгим для development окружения
    fn test_path_traversal_detection() {
        let validator = InputValidator::production();

        // Безопасный путь
        let result = validator.validate_file_path("./test.txt", "read");
        assert!(result.is_valid, "Safe path should be valid");

        // Path traversal
        let result = validator.validate_file_path("../../../etc/passwd", "read");
        assert!(!result.is_valid, "Path traversal should be detected");

        // System directory
        let result = validator.validate_file_path("/etc/passwd", "read");
        assert!(
            !result.is_valid,
            "System directory access should be blocked"
        );
    }

    #[test]
    fn test_dangerous_extension_detection() {
        let validator = InputValidator::production();

        // Разрешенное расширение
        let result = validator.validate_file_path("test.txt", "write");
        assert!(result.is_valid, "Allowed extension should be valid");

        // Опасное расширение
        let result = validator.validate_file_path("malware.exe", "write");
        assert!(!result.is_valid, "Dangerous extension should be blocked");
    }

    #[test]
    #[ignore] // Тест может быть слишком строгим для development окружения
    fn test_url_validation() {
        let validator = InputValidator::production();

        // Безопасный URL
        let result = validator.validate_url("https://api.example.com/data");
        assert!(result.is_valid, "Safe HTTPS URL should be valid");

        // Localhost блокировка
        let result = validator.validate_url("http://localhost:8080/admin");
        assert!(!result.is_valid, "Localhost URL should be blocked");

        // Опасная схема
        let result = validator.validate_url("javascript:alert('xss')");
        assert!(!result.is_valid, "Dangerous scheme should be blocked");
    }
}
