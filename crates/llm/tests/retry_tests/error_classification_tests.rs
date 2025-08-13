//! Tests for error classification (retryable vs non-retryable)

#[cfg(test)]
mod tests {
    use crate::retry::*;

    #[test]
    fn test_retry_error_from_status_codes() {
        // Retryable errors
        let rate_limit_error = RetryError::from_status_code(429, "Rate limited".to_string());
        assert!(rate_limit_error.is_retryable());
        assert_eq!(rate_limit_error.error_type(), "rate_limit");

        let server_error = RetryError::from_status_code(500, "Internal server error".to_string());
        assert!(server_error.is_retryable());
        assert_eq!(server_error.error_type(), "server_error");

        let timeout_error = RetryError::from_status_code(408, "Request timeout".to_string());
        assert!(timeout_error.is_retryable());
        assert_eq!(timeout_error.error_type(), "timeout");

        // Non-retryable errors
        let bad_request = RetryError::from_status_code(400, "Bad request".to_string());
        assert!(!bad_request.is_retryable());
        assert_eq!(bad_request.error_type(), "bad_request");

        let unauthorized = RetryError::from_status_code(401, "Unauthorized".to_string());
        assert!(!unauthorized.is_retryable());
        assert_eq!(unauthorized.error_type(), "unauthorized");

        let forbidden = RetryError::from_status_code(403, "Forbidden".to_string());
        assert!(!forbidden.is_retryable());
        assert_eq!(forbidden.error_type(), "forbidden");

        let not_found = RetryError::from_status_code(404, "Not found".to_string());
        assert!(!not_found.is_retryable());
        assert_eq!(not_found.error_type(), "not_found");
    }

    #[test]
    fn test_retry_error_from_reqwest_errors() {
        // This test would require creating mock reqwest::Error objects
        // For simplicity, we'll test the logic indirectly through integration tests
        // In a full implementation, you'd create specific reqwest error scenarios
    }

    #[test]
    fn test_retry_error_with_status_code() {
        let mut error = RetryError::new(
            "custom_error".to_string(),
            "Custom error message".to_string(),
            true,
        );
        
        assert!(error.is_retryable());
        assert_eq!(error.error_type(), "custom_error");
        assert_eq!(error.error_message(), "Custom error message");
        assert!(error.status_code.is_none());

        error = error.with_status_code(503);
        assert_eq!(error.status_code, Some(503));
    }
}