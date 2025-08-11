use crate::providers::LlmRequest;
use anyhow::Result;
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use tracing::debug;

/// Analyzes incoming requests to determine complexity and priority
pub struct RequestAnalyzer {
    complexity_patterns: HashMap<String, RequestComplexity>,
    priority_patterns: HashMap<String, TaskPriority>,
}

/// Request complexity levels for intelligent provider selection
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum RequestComplexity {
    Simple,   // Basic questions, simple tasks (< 500 tokens)
    Medium,   // Code review, analysis, moderate reasoning (500-2000 tokens)
    Complex,  // Architecture design, complex reasoning (2000-8000 tokens)
    Expert,   // Advanced technical tasks, research (8000+ tokens)
}

/// Task priority levels affecting provider selection
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TaskPriority {
    Low,      // Background tasks, non-urgent
    Normal,   // Standard requests
    High,     // Important but not time-critical
    Critical, // Time-sensitive, requires best available provider
}

impl RequestAnalyzer {
    pub fn new() -> Self {
        let mut complexity_patterns = HashMap::new();
        let mut priority_patterns = HashMap::new();
        
        // Complexity patterns (keywords that indicate complexity)
        complexity_patterns.insert("architecture".to_string(), RequestComplexity::Expert);
        complexity_patterns.insert("design pattern".to_string(), RequestComplexity::Complex);
        complexity_patterns.insert("refactor".to_string(), RequestComplexity::Complex);
        complexity_patterns.insert("optimize".to_string(), RequestComplexity::Complex);
        complexity_patterns.insert("debug".to_string(), RequestComplexity::Medium);
        complexity_patterns.insert("explain".to_string(), RequestComplexity::Medium);
        complexity_patterns.insert("review".to_string(), RequestComplexity::Medium);
        complexity_patterns.insert("hello".to_string(), RequestComplexity::Simple);
        complexity_patterns.insert("what is".to_string(), RequestComplexity::Simple);
        complexity_patterns.insert("how do i".to_string(), RequestComplexity::Simple);
        
        // Priority patterns (keywords that indicate urgency)
        priority_patterns.insert("urgent".to_string(), TaskPriority::Critical);
        priority_patterns.insert("asap".to_string(), TaskPriority::Critical);
        priority_patterns.insert("critical".to_string(), TaskPriority::Critical);
        priority_patterns.insert("emergency".to_string(), TaskPriority::Critical);
        priority_patterns.insert("important".to_string(), TaskPriority::High);
        priority_patterns.insert("priority".to_string(), TaskPriority::High);
        priority_patterns.insert("when you can".to_string(), TaskPriority::Low);
        priority_patterns.insert("no rush".to_string(), TaskPriority::Low);
        priority_patterns.insert("background".to_string(), TaskPriority::Low);
        
        Self {
            complexity_patterns,
            priority_patterns,
        }
    }
    
    /// Analyze request complexity based on content and size
    pub async fn analyze_complexity(&self, request: &LlmRequest) -> Result<RequestComplexity> {
        let combined_text = format!("{} {}", 
            request.system_prompt.as_deref().unwrap_or(""),
            request.prompt
        ).to_lowercase();
        
        for (pattern, complexity) in &self.complexity_patterns {
            if combined_text.contains(pattern) {
                debug!("🎯 Complexity pattern match: '{}' -> {:?}", pattern, complexity);
                return Ok(complexity.clone());
            }
        }
        
        // Fallback to token-based estimation
        let estimated_tokens = self.estimate_tokens(&combined_text);
        let complexity = match estimated_tokens {
            0..=500 => RequestComplexity::Simple,
            501..=2000 => RequestComplexity::Medium,
            2001..=8000 => RequestComplexity::Complex,
            _ => RequestComplexity::Expert,
        };
        
        debug!("📏 Token-based complexity: {} tokens -> {:?}", estimated_tokens, complexity);
        Ok(complexity)
    }
    
    /// Analyze task priority based on content and context
    pub async fn analyze_priority(&self, request: &LlmRequest) -> Result<TaskPriority> {
        let combined_text = format!("{} {}", 
            request.system_prompt.as_deref().unwrap_or(""),
            request.prompt
        ).to_lowercase();
        
        for (pattern, priority) in &self.priority_patterns {
            if combined_text.contains(pattern) {
                debug!("🚨 Priority pattern match: '{}' -> {:?}", pattern, priority);
                return Ok(priority.clone());
            }
        }
        
        // Implicit priority detection
        let priority = if self.has_urgent_indicators(&combined_text) {
            TaskPriority::High
        } else if self.has_background_indicators(&combined_text) {
            TaskPriority::Low
        } else {
            TaskPriority::Normal
        };
        
        debug!("🔍 Implicit priority analysis: {:?}", priority);
        Ok(priority)
    }
    
    /// Estimate token count (rough approximation)
    fn estimate_tokens(&self, text: &str) -> u32 {
        // English: ~1 token per 4 characters
        // Code: ~1 token per 3 characters (more dense)
        let char_count = text.len();
        
        let code_indicators = ["{", "}", "(", ")", "function", "class", "import", "def", "let", "var"];
        let has_code = code_indicators.iter().any(|&indicator| text.contains(indicator));
        
        let divisor = if has_code { 3.0 } else { 4.0 };
        (char_count as f32 / divisor).ceil() as u32
    }
    
    /// Check for urgent indicators in text
    fn has_urgent_indicators(&self, text: &str) -> bool {
        let urgent_words = [
            "deadline", "due today", "immediately", "fix asap", "broken", "error",
            "failing", "production", "outage", "down", "crash"
        ];
        
        urgent_words.iter().any(|&word| text.contains(word))
    }
    
    /// Check for background task indicators
    fn has_background_indicators(&self, text: &str) -> bool {
        let background_words = [
            "later", "eventually", "sometime", "optional", "nice to have",
            "improvement", "enhancement", "cleanup", "refactor when", "future"
        ];
        
        background_words.iter().any(|&word| text.contains(word))
    }
    
    /// Get detailed analysis report
    pub async fn get_analysis_report(&self, request: &LlmRequest) -> Result<AnalysisReport> {
        let complexity = self.analyze_complexity(request).await?;
        let priority = self.analyze_priority(request).await?;
        let estimated_tokens = self.estimate_tokens(&request.prompt);
        
        let combined_text = format!("{} {}", 
            request.system_prompt.as_deref().unwrap_or(""),
            request.prompt
        );
        
        let report = AnalysisReport {
            complexity,
            priority,
            estimated_tokens,
            text_length: combined_text.len(),
            has_code: self.detect_code_content(&combined_text),
            has_urgent_language: self.has_urgent_indicators(&combined_text.to_lowercase()),
            detected_patterns: self.get_detected_patterns(&combined_text.to_lowercase()),
            recommended_timeout: self.recommend_timeout(&complexity, &priority),
        };
        
        Ok(report)
    }
    
    /// Detect if content contains code
    fn detect_code_content(&self, text: &str) -> bool {
        let code_patterns = [
            "```", "function", "class", "import", "def ", "let ", "var ", "const ",
            "if (", "for (", "while (", "{", "}", "=>", "//", "/*", "*/"
        ];
        
        code_patterns.iter().any(|&pattern| text.contains(pattern))
    }
    
    /// Get all detected patterns from text
    fn get_detected_patterns(&self, text: &str) -> Vec<String> {
        let mut patterns = Vec::new();
        
        for (pattern, _) in &self.complexity_patterns {
            if text.contains(pattern) {
                patterns.push(format!("complexity:{}", pattern));
            }
        }
        
        for (pattern, _) in &self.priority_patterns {
            if text.contains(pattern) {
                patterns.push(format!("priority:{}", pattern));
            }
        }
        
        patterns
    }
    
    /// Recommend timeout based on complexity and priority
    fn recommend_timeout(&self, complexity: &RequestComplexity, priority: &TaskPriority) -> std::time::Duration {
        use std::time::Duration;
        
        let base_timeout = match complexity {
            RequestComplexity::Simple => Duration::from_secs(10),
            RequestComplexity::Medium => Duration::from_secs(30),
            RequestComplexity::Complex => Duration::from_secs(60),
            RequestComplexity::Expert => Duration::from_secs(120),
        };
        
        // Adjust based on priority
        match priority {
            TaskPriority::Critical => base_timeout / 2, // Faster timeout for critical
            TaskPriority::High => base_timeout,
            TaskPriority::Normal => base_timeout,
            TaskPriority::Low => base_timeout * 2, // Allow more time for background tasks
        }
    }
}

/// Detailed analysis report for a request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisReport {
    pub complexity: RequestComplexity,
    pub priority: TaskPriority,
    pub estimated_tokens: u32,
    pub text_length: usize,
    pub has_code: bool,
    pub has_urgent_language: bool,
    pub detected_patterns: Vec<String>,
    pub recommended_timeout: std::time::Duration,
}

impl Default for RequestAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_complexity_analysis() {
        let analyzer = RequestAnalyzer::new();
        
        // Simple request
        let simple_request = LlmRequest::new("Hello, how are you?");
        let complexity = analyzer.analyze_complexity(&simple_request).await.unwrap();
        assert_eq!(complexity, RequestComplexity::Simple);
        
        // Complex request
        let complex_request = LlmRequest::new("Design a microservices architecture for a high-traffic e-commerce platform with considerations for scalability, fault tolerance, and data consistency.");
        let complexity = analyzer.analyze_complexity(&complex_request).await.unwrap();
        assert!(matches!(complexity, RequestComplexity::Complex | RequestComplexity::Expert));
    }
    
    #[tokio::test]
    async fn test_priority_analysis() {
        let analyzer = RequestAnalyzer::new();
        
        // Critical request
        let critical_request = LlmRequest::new("URGENT: Production is down and users can't access the system!");
        let priority = analyzer.analyze_priority(&critical_request).await.unwrap();
        assert!(matches!(priority, TaskPriority::Critical | TaskPriority::High));
        
        // Low priority request
        let low_request = LlmRequest::new("When you have time, could you help clean up this code?");
        let priority = analyzer.analyze_priority(&low_request).await.unwrap();
        assert_eq!(priority, TaskPriority::Low);
    }
    
    #[tokio::test]
    async fn test_token_estimation() {
        let analyzer = RequestAnalyzer::new();
        
        // Short text
        let short_tokens = analyzer.estimate_tokens("Hello world");
        assert!(short_tokens < 10);
        
        // Long text
        let long_text = "This is a much longer text that contains many words and should result in a higher token count estimate.";
        let long_tokens = analyzer.estimate_tokens(long_text);
        assert!(long_tokens > short_tokens);
        
        // Code text (should be more dense)
        let code_text = "function hello() { return 'world'; }";
        let code_tokens = analyzer.estimate_tokens(code_text);
        let regular_tokens = analyzer.estimate_tokens(&"x".repeat(code_text.len()));
        assert!(code_tokens > regular_tokens);
    }
    
    #[tokio::test]
    async fn test_analysis_report() {
        let analyzer = RequestAnalyzer::new();
        let request = LlmRequest::new("URGENT: Please review this complex architecture design and provide feedback ASAP");
        
        let report = analyzer.get_analysis_report(&request).await.unwrap();
        
        assert!(matches!(report.complexity, RequestComplexity::Medium | RequestComplexity::Complex));
        assert_eq!(report.priority, TaskPriority::Critical);
        assert!(!report.detected_patterns.is_empty());
        assert!(report.estimated_tokens > 0);
    }
}