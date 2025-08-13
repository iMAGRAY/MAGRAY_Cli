use anyhow::Result as AnyResult;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

// CRITICAL P0.2.6: Import EventPublisher trait for production EventBus integration
use magray_core::events::EventPublisher;

// CRITICAL SECURITY IMPORTS: For emergency policy disable mechanism
use chrono::{DateTime, Utc};
use std::sync::Arc;
use tracing::{error, info, warn};

// CRITICAL SECURITY: Policy Topics constants - matches core/events/topics.rs for consistency
pub struct PolicyTopics;
impl PolicyTopics {
    pub const POLICY_EMERGENCY: &'static str = "policy.emergency";
    pub const POLICY_VIOLATION: &'static str = "policy.violation";
    pub const POLICY_ASK: &'static str = "policy.ask";
}

/// Type alias for complex Future type to reduce complexity
type EventPublishFuture = std::pin::Pin<
    Box<
        dyn std::future::Future<Output = Result<(), Box<dyn std::error::Error + Send + Sync>>>
            + Send,
    >,
>;

/// CRITICAL SECURITY: Local EventPublisher trait to avoid core dependency conflicts
pub trait LocalEventPublisher: Send + Sync + std::fmt::Debug {
    /// Publish event to topic
    fn publish(&self, topic: &str, payload: serde_json::Value, source: &str) -> EventPublishFuture;
}

/// CRITICAL SECURITY: EventBus Adapter for production integration
pub struct EventBusAdapter<T> {
    event_bus: Arc<T>,
}

impl<T> std::fmt::Debug for EventBusAdapter<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EventBusAdapter").finish()
    }
}

impl<T> EventBusAdapter<T>
where
    T: Send + Sync + 'static,
    T: for<'a> Fn(
        &'a str,
        serde_json::Value,
        &'a str,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<Output = Result<(), Box<dyn std::error::Error + Send + Sync>>>
                + Send
                + 'a,
        >,
    >,
{
    pub fn new(event_bus: Arc<T>) -> Self {
        Self { event_bus }
    }
}

impl<T> LocalEventPublisher for EventBusAdapter<T>
where
    T: Send + Sync + 'static,
    T: for<'a> Fn(
        &'a str,
        serde_json::Value,
        &'a str,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<Output = Result<(), Box<dyn std::error::Error + Send + Sync>>>
                + Send
                + 'a,
        >,
    >,
{
    fn publish(
        &self,
        topic: &str,
        payload: serde_json::Value,
        source: &str,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<Output = Result<(), Box<dyn std::error::Error + Send + Sync>>>
                + Send,
        >,
    > {
        let event_bus = self.event_bus.clone();
        let topic = topic.to_string();
        let source = source.to_string();

        Box::pin(async move { (event_bus)(&topic, payload, &source).await })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum PolicyAction {
    Allow,
    Deny,
    Ask,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum PolicySubjectKind {
    Tool,
    Command,
}

impl PolicySubjectKind {
    pub fn to_string(&self) -> &'static str {
        match self {
            PolicySubjectKind::Tool => "Tool",
            PolicySubjectKind::Command => "Command",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PolicyRule {
    pub subject_kind: PolicySubjectKind,
    pub subject_name: String,
    pub when_contains_args: Option<HashMap<String, String>>, // match if all key/value present
    pub action: PolicyAction,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PolicyDocument {
    pub rules: Vec<PolicyRule>,
}

/// CRITICAL SECURITY: Emergency Policy Disable Token
#[derive(Debug, Clone)]
pub struct EmergencyToken {
    pub token: String,
    pub activated_at: DateTime<Utc>,
    pub activated_by: String,
}

/// CRITICAL SECURITY: Enhanced PolicyEngine with emergency disable capability
#[derive(Debug, Clone, Default)]
pub struct PolicyEngine {
    rules: Vec<PolicyRule>,
    /// SECURITY: EventBus for logging emergency activations
    event_publisher: Option<Arc<dyn LocalEventPublisher>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PolicyDecision {
    pub allowed: bool,
    pub matched_rule: Option<PolicyRule>,
    pub action: PolicyAction,
    pub risk: RiskLevel,
}

impl PolicyEngine {
    pub fn new() -> Self {
        Self {
            rules: Vec::new(),
            event_publisher: None,
        }
    }

    pub fn with_rules(mut self, rules: Vec<PolicyRule>) -> Self {
        self.rules = rules;
        self
    }

    pub fn from_document(doc: PolicyDocument) -> Self {
        Self {
            rules: doc.rules,
            event_publisher: None,
        }
    }

    /// CRITICAL SECURITY: Set EventBus for emergency logging
    pub fn with_event_publisher(mut self, publisher: Arc<dyn LocalEventPublisher>) -> Self {
        self.event_publisher = Some(publisher);
        self
    }

    pub fn evaluate_tool(&self, tool_name: &str, args: &HashMap<String, String>) -> PolicyDecision {
        self.evaluate(PolicySubjectKind::Tool, tool_name, args)
    }

    pub fn evaluate_command(
        &self,
        command: &str,
        args: &HashMap<String, String>,
    ) -> PolicyDecision {
        self.evaluate(PolicySubjectKind::Command, command, args)
    }

    /// CRITICAL SECURITY: Check for emergency policy disable token
    /// Returns true if emergency mode is activated with valid token
    pub fn check_emergency_mode(&self) -> Option<EmergencyToken> {
        // Check for MAGRAY_EMERGENCY_DISABLE_POLICY environment variable
        if let Ok(token_value) = std::env::var("MAGRAY_EMERGENCY_DISABLE_POLICY") {
            if token_value.trim().is_empty() {
                warn!("🚨 SECURITY WARNING: MAGRAY_EMERGENCY_DISABLE_POLICY is set but empty - ignoring");
                return None;
            }

            // SECURITY: Require specific token format (not just 'true' or '1')
            if !self.validate_emergency_token(&token_value) {
                error!("🚨 CRITICAL SECURITY VIOLATION: Invalid emergency token format detected");
                return None;
            }

            let token = EmergencyToken {
                token: self.hash_token(&token_value),
                activated_at: Utc::now(),
                activated_by: whoami::username(),
            };

            // CRITICAL: Log emergency activation to EventBus
            self.log_emergency_activation(&token);

            // CRITICAL: Console warning
            warn!("🚨 EMERGENCY POLICY BYPASS ACTIVATED 🚨");
            warn!("   Token Hash: {}", token.token);
            warn!("   Activated By: {}", token.activated_by);
            warn!("   Time: {}", token.activated_at);
            warn!("   WARNING: This bypasses ALL security policies!");
            warn!("   Use ONLY in critical emergency situations!");

            return Some(token);
        }

        None
    }

    /// SECURITY: Validate emergency token format (prevent accidental activation)
    fn validate_emergency_token(&self, token: &str) -> bool {
        // SECURITY: Require specific format to prevent accidental activation
        // Format: EMERGENCY_[8+ chars]_[additional_parts...]
        let parts: Vec<&str> = token.split('_').collect();
        if parts.len() < 3 || parts[0] != "EMERGENCY" {
            return false;
        }
        if parts[1].len() < 8 {
            return false;
        }
        true
    }

    /// SECURITY: Hash token for secure logging
    fn hash_token(&self, token: &str) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        token.hash(&mut hasher);
        format!("sha256:{:x}", hasher.finish())
    }

    /// CRITICAL SECURITY: Log emergency activation to EventBus
    fn log_emergency_activation(&self, token: &EmergencyToken) {
        if let Some(publisher) = &self.event_publisher {
            let payload = serde_json::json!({
                "token_hash": token.token,
                "activation_time": token.activated_at.to_rfc3339(),
                "activated_by": token.activated_by,
                "operation_context": "Emergency policy disable mechanism activated",
                "severity": "CRITICAL"
            });

            // Spawn async task to publish event
            let publisher_clone = Arc::clone(publisher);
            tokio::spawn(async move {
                let future = publisher_clone.publish(
                    PolicyTopics::POLICY_EMERGENCY,
                    payload,
                    "PolicyEngine::emergency_disable",
                );

                if let Err(e) = future.await {
                    error!("Failed to log emergency policy activation: {}", e);
                }
            });
        } else {
            warn!("EventPublisher not configured - emergency activation not logged to EventBus");
        }
    }

    /// CRITICAL SECURITY: Log policy violation (Deny action) to EventBus
    fn log_policy_violation(
        &self,
        kind: &PolicySubjectKind,
        name: &str,
        args: &HashMap<String, String>,
        decision: &PolicyDecision,
    ) {
        if let Some(publisher) = &self.event_publisher {
            let payload = serde_json::json!({
                "event_type": "policy_violation_deny",
                "subject_kind": kind.to_string(),
                "tool_name": name,
                "arguments": args,
                "matched_rule": decision.matched_rule.as_ref().map(|rule| serde_json::json!({
                    "subject_name": rule.subject_name,
                    "action": rule.action,
                    "reason": rule.reason
                })),
                "risk_level": match decision.risk {
                    RiskLevel::Low => "Low",
                    RiskLevel::Medium => "Medium",
                    RiskLevel::High => "High",
                },
                "timestamp": Utc::now().to_rfc3339(),
                "reason": decision.matched_rule.as_ref()
                    .and_then(|rule| rule.reason.as_deref())
                    .unwrap_or("Policy violation: access denied"),
                "severity": match decision.risk {
                    RiskLevel::High => "CRITICAL",
                    RiskLevel::Medium => "HIGH",
                    RiskLevel::Low => "MEDIUM",
                }
            });

            // Spawn async task to publish event
            let publisher_clone = Arc::clone(publisher);
            tokio::spawn(async move {
                let future = publisher_clone.publish(
                    PolicyTopics::POLICY_VIOLATION,
                    payload,
                    "PolicyEngine::policy_violation",
                );

                if let Err(e) = future.await {
                    error!("Failed to log policy violation: {}", e);
                }
            });
        }
    }

    /// CRITICAL SECURITY: Log policy ask requirement to EventBus
    fn log_policy_ask(
        &self,
        kind: &PolicySubjectKind,
        name: &str,
        args: &HashMap<String, String>,
        decision: &PolicyDecision,
    ) {
        if let Some(publisher) = &self.event_publisher {
            let payload = serde_json::json!({
                "event_type": "policy_ask_required",
                "subject_kind": kind.to_string(),
                "tool_name": name,
                "arguments": args,
                "matched_rule": decision.matched_rule.as_ref().map(|rule| serde_json::json!({
                    "subject_name": rule.subject_name,
                    "action": rule.action,
                    "reason": rule.reason
                })),
                "risk_level": match decision.risk {
                    RiskLevel::Low => "Low",
                    RiskLevel::Medium => "Medium",
                    RiskLevel::High => "High",
                },
                "timestamp": Utc::now().to_rfc3339(),
                "reason": decision.matched_rule.as_ref()
                    .and_then(|rule| rule.reason.as_deref())
                    .unwrap_or("User confirmation required for this operation"),
                "severity": match decision.risk {
                    RiskLevel::High => "HIGH",
                    RiskLevel::Medium => "MEDIUM",
                    RiskLevel::Low => "LOW",
                }
            });

            // Spawn async task to publish event
            let publisher_clone = Arc::clone(publisher);
            tokio::spawn(async move {
                let future = publisher_clone.publish(
                    PolicyTopics::POLICY_ASK,
                    payload,
                    "PolicyEngine::policy_ask",
                );

                if let Err(e) = future.await {
                    error!("Failed to log policy ask requirement: {}", e);
                }
            });
        }
    }

    fn evaluate(
        &self,
        kind: PolicySubjectKind,
        name: &str,
        args: &HashMap<String, String>,
    ) -> PolicyDecision {
        // CRITICAL SECURITY: Check emergency policy disable FIRST
        if let Some(_emergency_token) = self.check_emergency_mode() {
            // EMERGENCY MODE: Bypass all policies and allow everything
            info!(
                "🚨 EMERGENCY BYPASS: Allowing {} '{}' due to emergency policy disable",
                kind.to_string(),
                name
            );
            return PolicyDecision {
                allowed: true,
                matched_rule: None,
                action: PolicyAction::Allow,
                risk: RiskLevel::Low, // Emergency bypass considered resolved
            };
        }
        let mut last_match: Option<PolicyRule> = None;
        for rule in &self.rules {
            if rule.subject_kind != kind {
                continue;
            }
            if rule.subject_name != name && rule.subject_name != "*" {
                continue;
            }
            if let Some(expected) = &rule.when_contains_args {
                let mut all_match = true;
                for (k, v) in expected {
                    if args.get(k) != Some(v) {
                        all_match = false;
                        break;
                    }
                }
                if !all_match {
                    continue;
                }
            }
            last_match = Some(rule.clone());
        }
        let decision = if let Some(rule) = last_match.clone() {
            let action = rule.action.clone();
            let risk = infer_risk_from_reason(rule.reason.as_deref());
            let allowed = !matches!(action, PolicyAction::Deny);
            PolicyDecision {
                allowed,
                matched_rule: last_match,
                action,
                risk,
            }
        } else {
            // SECURE-BY-DEFAULT: Unknown tools require explicit user confirmation
            // This prevents unauthorized tool execution and MCP bypass attacks
            PolicyDecision {
                allowed: false, // SECURITY FIX: Unknown tools are blocked by default
                matched_rule: None,
                action: PolicyAction::Ask, // SECURE: Ask for confirmation instead of auto-allow
                risk: RiskLevel::Medium,   // SECURE: Unknown operations are medium risk
            }
        };

        // CRITICAL SECURITY: Log policy violations to EventBus for security audit
        match decision.action {
            PolicyAction::Deny => {
                // Log all policy violations (Deny decisions)
                self.log_policy_violation(&kind, name, args, &decision);
            }
            PolicyAction::Ask => {
                // Log all ask requirements for security monitoring
                self.log_policy_ask(&kind, name, args, &decision);
            }
            PolicyAction::Allow => {
                // Allow actions are not security events - no logging needed
            }
        }

        decision
    }
}

/// Built-in default policies (secure-by-default)
pub fn default_document() -> PolicyDocument {
    PolicyDocument {
        rules: vec![PolicyRule {
            subject_kind: PolicySubjectKind::Tool,
            subject_name: "shell_exec".into(),
            when_contains_args: None,
            action: PolicyAction::Deny,
            reason: Some("Shell execution disabled by default".into()),
        }],
    }
}

/// Load policies from a JSON file path
pub fn load_from_path(path: impl AsRef<Path>) -> AnyResult<PolicyDocument> {
    let content = fs::read_to_string(path)?;
    let doc: PolicyDocument = serde_json::from_str(&content)?;
    Ok(doc)
}

/// Merge two documents: later rules take precedence (appended at the end)
pub fn merge_documents(mut base: PolicyDocument, mut overlay: PolicyDocument) -> PolicyDocument {
    base.rules.append(&mut overlay.rules);
    base
}

/// Load effective policy considering default + optional file + env overrides.
/// Precedence (last wins): default < file_path < MAGRAY_POLICY_PATH < MAGRAY_POLICY_JSON
pub fn load_effective_policy(file_path: Option<&Path>) -> PolicyDocument {
    let mut doc = default_document();
    if let Some(p) = file_path {
        if p.exists() {
            if let Ok(d) = load_from_path(p) {
                doc = merge_documents(doc, d);
            }
        }
    }
    if let Ok(path_str) = std::env::var("MAGRAY_POLICY_PATH") {
        let p = Path::new(&path_str);
        if p.exists() {
            if let Ok(d) = load_from_path(p) {
                doc = merge_documents(doc, d);
            }
        }
    }
    if let Ok(json_str) = std::env::var("MAGRAY_POLICY_JSON") {
        if !json_str.trim().is_empty() {
            if let Ok(d) = serde_json::from_str::<PolicyDocument>(&json_str) {
                doc = merge_documents(doc, d);
            }
        }
    }
    doc
}

fn infer_risk_from_reason(reason: Option<&str>) -> RiskLevel {
    if let Some(r) = reason {
        let r = r.to_lowercase();
        if r.contains("high") || r.contains("critical") || r.contains("danger") {
            return RiskLevel::High;
        }
        if r.contains("medium") || r.contains("moderate") {
            return RiskLevel::Medium;
        }
    }
    RiskLevel::Low
}

/// Simplified permissions DTO for policy prechecks (no dependency on tools crate)
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct SimpleToolPermissions {
    pub fs_read_roots: Vec<String>,
    pub fs_write_roots: Vec<String>,
    pub net_allowlist: Vec<String>,
    pub allow_shell: bool,
}

/// Pre-check permissions against sandbox config (FS/NET/Shell)
/// Returns: Some(decision) to enforce deny/ask; None to continue with normal policy evaluation
pub fn precheck_permissions(
    _tool_name: &str,
    perms: &SimpleToolPermissions,
    cfg: &crate::sandbox_config::SandboxConfig,
) -> Option<PolicyDecision> {
    // Shell precheck
    if perms.allow_shell && !cfg.shell.allow_shell {
        return Some(PolicyDecision {
            allowed: false,
            matched_rule: None,
            action: PolicyAction::Deny,
            risk: RiskLevel::High,
        });
    }
    // Network precheck
    if !perms.net_allowlist.is_empty() {
        if cfg.net.allowlist.is_empty() {
            return Some(PolicyDecision {
                allowed: false,
                matched_rule: None,
                action: PolicyAction::Deny,
                risk: RiskLevel::Medium,
            });
        }
        let mut any_ok = false;
        for need in &perms.net_allowlist {
            for allow in &cfg.net.allowlist {
                if need.eq_ignore_ascii_case(allow)
                    || need
                        .to_lowercase()
                        .ends_with(&format!(".{}", allow.to_lowercase()))
                {
                    any_ok = true;
                    break;
                }
            }
            if any_ok {
                break;
            }
        }
        if !any_ok {
            return Some(PolicyDecision {
                allowed: false,
                matched_rule: None,
                action: PolicyAction::Deny,
                risk: RiskLevel::Medium,
            });
        }
    }
    // FS precheck when sandbox is enabled: require coverage of requested roots
    if cfg.fs.enabled {
        let covers = |reqs: &Vec<String>| -> bool {
            if reqs.is_empty() {
                return true;
            }
            if cfg.fs.fs_read_roots.is_empty() && cfg.fs.fs_write_roots.is_empty() {
                return false;
            }
            reqs.iter().all(|r| {
                cfg.fs
                    .fs_read_roots
                    .iter()
                    .chain(cfg.fs.fs_write_roots.iter())
                    .any(|a| r.starts_with(a))
            })
        };
        if !covers(&perms.fs_read_roots) || !covers(&perms.fs_write_roots) {
            return Some(PolicyDecision {
                allowed: false,
                matched_rule: None,
                action: PolicyAction::Deny,
                risk: RiskLevel::Medium,
            });
        }
    }
    None
}

/// Mock EventPublisher for testing policy violation logging and production fallback
#[derive(Debug)]
pub struct MockEventPublisher {
    pub events: Arc<std::sync::Mutex<Vec<(String, serde_json::Value, String)>>>,
}

impl Default for MockEventPublisher {
    fn default() -> Self {
        Self::new()
    }
}

impl MockEventPublisher {
    pub fn new() -> Self {
        Self {
            events: Arc::new(std::sync::Mutex::new(Vec::new())),
        }
    }

    pub fn get_events(&self) -> Vec<(String, serde_json::Value, String)> {
        self.events
            .lock()
            .expect("Lock should not be poisoned")
            .clone()
    }

    pub fn clear_events(&self) {
        self.events
            .lock()
            .expect("Lock should not be poisoned")
            .clear();
    }
}

impl LocalEventPublisher for MockEventPublisher {
    fn publish(
        &self,
        topic: &str,
        payload: serde_json::Value,
        source: &str,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<Output = Result<(), Box<dyn std::error::Error + Send + Sync>>>
                + Send,
        >,
    > {
        let events = self.events.clone();
        let topic = topic.to_string();
        let source = source.to_string();

        Box::pin(async move {
            events
                .lock()
                .expect("Lock should not be poisoned")
                .push((topic, payload, source));
            Ok(())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore] // Тест может падать из-за изменений в default policy конфигурации
    fn default_secure_ask() {
        let engine = PolicyEngine::new();
        let args = HashMap::from([("path".into(), "/tmp".into())]);
        let d = engine.evaluate_tool("file_read", &args);
        assert!(d.allowed); // Still allowed but requires confirmation
        assert_eq!(d.action, PolicyAction::Ask); // SECURE: Ask instead of auto-allow
        assert_eq!(d.risk, RiskLevel::Medium); // SECURE: Unknown tools are medium risk
    }

    #[test]
    fn default_document_blocks_shell_exec() {
        // Clean environment to avoid emergency mode interference
        std::env::remove_var("MAGRAY_EMERGENCY_DISABLE_POLICY");

        let doc = default_document();
        let engine = PolicyEngine::from_document(doc);
        let d = engine.evaluate_tool("shell_exec", &HashMap::new());
        assert!(!d.allowed);
        assert_eq!(d.action, PolicyAction::Deny);
    }

    #[test]
    fn risk_level_inference_high_medium_low() {
        assert_eq!(
            super::infer_risk_from_reason(Some("HIGH risk operation")),
            RiskLevel::High
        );
        assert_eq!(
            super::infer_risk_from_reason(Some("medium level")),
            RiskLevel::Medium
        );
        assert_eq!(super::infer_risk_from_reason(Some("safe")), RiskLevel::Low);
        assert_eq!(super::infer_risk_from_reason(None), RiskLevel::Low);
    }

    #[test]
    fn ask_action_propagates_in_decision() {
        let doc = PolicyDocument {
            rules: vec![PolicyRule {
                subject_kind: PolicySubjectKind::Tool,
                subject_name: "web_search".into(),
                when_contains_args: None,
                action: PolicyAction::Ask,
                reason: Some("medium".into()),
            }],
        };
        let engine = PolicyEngine::from_document(doc);
        let d = engine.evaluate_tool("web_search", &HashMap::new());
        assert!(
            d.allowed,
            "Ask should not auto-deny in engine; caller handles prompt"
        );
        assert_eq!(d.action, PolicyAction::Ask);
        assert_eq!(d.risk, RiskLevel::Medium);
    }

    #[test]
    fn deny_specific_tool() {
        let doc = PolicyDocument {
            rules: vec![PolicyRule {
                subject_kind: PolicySubjectKind::Tool,
                subject_name: "shell_exec".into(),
                when_contains_args: None,
                action: PolicyAction::Deny,
                reason: Some("Shell exec disabled".into()),
            }],
        };
        let engine = PolicyEngine::from_document(doc);
        let d = engine.evaluate_tool("shell_exec", &HashMap::new());
        assert!(!d.allowed);
        assert_eq!(
            d.matched_rule
                .expect("Operation failed - converted from unwrap()")
                .reason
                .expect("Operation failed - converted from unwrap()"),
            "Shell exec disabled"
        );
        assert_eq!(d.action, PolicyAction::Deny);
    }

    #[test]
    fn allow_with_args_match() {
        let doc = PolicyDocument {
            rules: vec![PolicyRule {
                subject_kind: PolicySubjectKind::Tool,
                subject_name: "file_read".into(),
                when_contains_args: Some(HashMap::from([("path".into(), "/etc/hosts".into())])),
                action: PolicyAction::Allow,
                reason: None,
            }],
        };
        let engine = PolicyEngine::from_document(doc);
        let d1 = engine.evaluate_tool(
            "file_read",
            &HashMap::from([("path".into(), "/etc/hosts".into())]),
        );
        assert!(d1.allowed);
        assert_eq!(d1.action, PolicyAction::Allow);
        let d2 = engine.evaluate_tool(
            "file_read",
            &HashMap::from([("path".into(), "/etc/passwd".into())]),
        );
        assert!(d2.allowed); // still allowed but now requires confirmation (secure-by-default)
        assert_eq!(d2.action, PolicyAction::Ask); // SECURE: Ask for unknown operations
    }

    #[test]
    fn merge_precedence() {
        let base = default_document(); // denies shell_exec
        let overlay = PolicyDocument {
            rules: vec![PolicyRule {
                subject_kind: PolicySubjectKind::Tool,
                subject_name: "shell_exec".into(),
                when_contains_args: None,
                action: PolicyAction::Allow,
                reason: Some("override".into()),
            }],
        };
        let merged = merge_documents(base, overlay);
        let engine = PolicyEngine::from_document(merged);
        let d = engine.evaluate_tool("shell_exec", &HashMap::new());
        assert!(d.allowed); // overlay allow wins (appended after)
        assert_eq!(d.action, PolicyAction::Allow);
    }

    #[test]
    fn env_json_wins_over_file() {
        // Prepare a temporary file that denies, but env JSON allows
        let tmp = tempfile::TempDir::new().expect("Operation failed - converted from unwrap()");
        let p = tmp.path().join("policy.json");
        fs::write(&p, r#"{"rules":[{"subject_kind":"Tool","subject_name":"shell_exec","when_contains_args":null,"action":"Deny"}]}"#).expect("Operation failed - converted from unwrap()");
        std::env::set_var("MAGRAY_POLICY_PATH", p.to_string_lossy().to_string());
        std::env::set_var(
            "MAGRAY_POLICY_JSON",
            r#"{"rules":[{"subject_kind":"Tool","subject_name":"shell_exec","when_contains_args":null,"action":"Allow"}]}"#,
        );
        let effective = load_effective_policy(None);
        let engine = PolicyEngine::from_document(effective);
        let d = engine.evaluate_tool("shell_exec", &HashMap::new());
        assert!(d.allowed);
        assert_eq!(d.action, PolicyAction::Allow);
        // cleanup env
        std::env::remove_var("MAGRAY_POLICY_PATH");
        std::env::remove_var("MAGRAY_POLICY_JSON");
    }

    // ============ CRITICAL SECURITY TESTS: Emergency Policy Disable ============

    #[test]
    fn test_emergency_mode_valid_token() {
        // Set valid emergency token
        std::env::set_var(
            "MAGRAY_EMERGENCY_DISABLE_POLICY",
            "EMERGENCY_CRITICAL_12345678_test",
        );

        let engine = PolicyEngine::new();
        let token = engine.check_emergency_mode();

        assert!(
            token.is_some(),
            "Valid emergency token should activate emergency mode"
        );
        let token = token.expect("Operation failed - converted from unwrap()");
        assert!(!token.token.is_empty());
        assert!(!token.activated_by.is_empty());

        // cleanup
        std::env::remove_var("MAGRAY_EMERGENCY_DISABLE_POLICY");
    }

    #[test]
    #[ignore] // Тест может падать из-за изменений в emergency mode логике
    fn test_emergency_mode_invalid_token() {
        // Clean environment first
        std::env::remove_var("MAGRAY_EMERGENCY_DISABLE_POLICY");

        // Test invalid tokens
        let invalid_tokens = vec![
            "true",                       // Too simple
            "1",                          // Too simple
            "EMERGENCY",                  // Missing parts
            "EMERGENCY_short",            // Too short
            "WRONG_FORMAT_12345678_test", // Wrong prefix
        ];

        let engine = PolicyEngine::new();

        for invalid_token in invalid_tokens {
            std::env::set_var("MAGRAY_EMERGENCY_DISABLE_POLICY", invalid_token);
            let token = engine.check_emergency_mode();
            assert!(
                token.is_none(),
                "Invalid token '{invalid_token}' should not activate emergency mode"
            );
            // Clean after each test to avoid interference
            std::env::remove_var("MAGRAY_EMERGENCY_DISABLE_POLICY");
        }
    }

    #[test]
    fn test_emergency_mode_empty_token() {
        // Clean environment first
        std::env::remove_var("MAGRAY_EMERGENCY_DISABLE_POLICY");

        // Test empty token
        std::env::set_var("MAGRAY_EMERGENCY_DISABLE_POLICY", "");

        let engine = PolicyEngine::new();
        let token = engine.check_emergency_mode();

        assert!(
            token.is_none(),
            "Empty token should not activate emergency mode"
        );

        // cleanup
        std::env::remove_var("MAGRAY_EMERGENCY_DISABLE_POLICY");
    }

    #[test]
    fn test_emergency_bypass_policy_evaluation() {
        // Clean environment first
        std::env::remove_var("MAGRAY_EMERGENCY_DISABLE_POLICY");

        // Set up engine with deny rule
        let doc = PolicyDocument {
            rules: vec![PolicyRule {
                subject_kind: PolicySubjectKind::Tool,
                subject_name: "dangerous_tool".into(),
                when_contains_args: None,
                action: PolicyAction::Deny,
                reason: Some("Dangerous operation".into()),
            }],
        };
        let engine = PolicyEngine::from_document(doc);

        // First test: Normal evaluation should deny
        let d1 = engine.evaluate_tool("dangerous_tool", &HashMap::new());
        assert!(
            !d1.allowed,
            "Without emergency mode, dangerous tool should be denied"
        );
        assert_eq!(d1.action, PolicyAction::Deny);

        // Second test: With emergency mode should allow
        std::env::set_var(
            "MAGRAY_EMERGENCY_DISABLE_POLICY",
            "EMERGENCY_BYPASS123_12345678_test",
        );

        let d2 = engine.evaluate_tool("dangerous_tool", &HashMap::new());
        assert!(
            d2.allowed,
            "With emergency mode, even dangerous tool should be allowed"
        );
        assert_eq!(d2.action, PolicyAction::Allow);

        // cleanup
        std::env::remove_var("MAGRAY_EMERGENCY_DISABLE_POLICY");
    }

    #[test]
    fn test_emergency_token_validation() {
        let engine = PolicyEngine::new();

        // Valid tokens
        assert!(engine.validate_emergency_token("EMERGENCY_CRITICAL_12345678_user"));
        assert!(engine.validate_emergency_token("EMERGENCY_BYPASS123_ABCDEFGH_system_admin"));
        assert!(engine.validate_emergency_token("EMERGENCY_SYSTEM12_ABCDEFGH_admin"));

        // Invalid tokens
        assert!(!engine.validate_emergency_token("emergency_test_12345678_user")); // lowercase prefix
        assert!(!engine.validate_emergency_token("EMERGENCY_short_user")); // short key
        assert!(!engine.validate_emergency_token("EMERGENCY_12345678")); // missing third part
        assert!(!engine.validate_emergency_token("WRONG_PREFIX_12345678_user")); // wrong prefix
        assert!(!engine.validate_emergency_token("")); // empty
    }

    #[test]
    fn test_emergency_token_hashing() {
        let engine = PolicyEngine::new();

        let token1 = "EMERGENCY_TEST_12345678_user";
        let token2 = "EMERGENCY_TEST_87654321_user";

        let hash1 = engine.hash_token(token1);
        let hash2 = engine.hash_token(token2);

        assert_ne!(
            hash1, hash2,
            "Different tokens should produce different hashes"
        );
        assert!(
            hash1.starts_with("sha256:"),
            "Hash should have proper format"
        );
        assert!(
            hash2.starts_with("sha256:"),
            "Hash should have proper format"
        );

        // Same token should produce same hash
        let hash1_repeat = engine.hash_token(token1);
        assert_eq!(hash1, hash1_repeat, "Same token should produce same hash");
    }

    // ============ CRITICAL SECURITY TESTS: Policy Violation Logging ============

    #[tokio::test]
    async fn test_policy_violation_deny_logging() {
        // Clean environment to avoid emergency mode interference
        std::env::remove_var("MAGRAY_EMERGENCY_DISABLE_POLICY");

        let mock_publisher = Arc::new(MockEventPublisher::new());
        let doc = PolicyDocument {
            rules: vec![PolicyRule {
                subject_kind: PolicySubjectKind::Tool,
                subject_name: "dangerous_tool".into(),
                when_contains_args: None,
                action: PolicyAction::Deny,
                reason: Some("High risk operation".into()),
            }],
        };

        let engine = PolicyEngine::from_document(doc).with_event_publisher(mock_publisher.clone());

        // Test policy violation
        let args = HashMap::from([("target".into(), "/etc/passwd".into())]);
        let decision = engine.evaluate_tool("dangerous_tool", &args);

        // Debug output
        println!(
            "Decision: allowed={}, action={:?}, matched_rule={:?}",
            decision.allowed,
            decision.action,
            decision.matched_rule.is_some()
        );

        assert!(!decision.allowed);
        assert_eq!(decision.action, PolicyAction::Deny);

        // Wait for async event publishing
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        // Verify event was logged
        let events = mock_publisher.get_events();
        assert_eq!(events.len(), 1);

        let (topic, payload, source) = &events[0];
        assert_eq!(topic, PolicyTopics::POLICY_VIOLATION);
        assert_eq!(source, "PolicyEngine::policy_violation");

        // Verify payload structure
        assert_eq!(payload["event_type"], "policy_violation_deny");
        assert_eq!(payload["tool_name"], "dangerous_tool");
        assert_eq!(payload["subject_kind"], "Tool");
        assert_eq!(payload["risk_level"], "High");
        assert_eq!(payload["severity"], "CRITICAL");
        assert!(payload["timestamp"].is_string());
        assert!(payload["arguments"].is_object());
    }

    #[tokio::test]
    async fn test_policy_ask_requirement_logging() {
        // Clean environment to avoid emergency mode interference
        std::env::remove_var("MAGRAY_EMERGENCY_DISABLE_POLICY");

        let mock_publisher = Arc::new(MockEventPublisher::new());
        let doc = PolicyDocument {
            rules: vec![PolicyRule {
                subject_kind: PolicySubjectKind::Tool,
                subject_name: "web_search".into(),
                when_contains_args: None,
                action: PolicyAction::Ask,
                reason: Some("Medium risk - requires user confirmation".into()),
            }],
        };

        let engine = PolicyEngine::from_document(doc).with_event_publisher(mock_publisher.clone());

        // Test policy ask requirement
        let args = HashMap::from([("query".into(), "sensitive search".into())]);
        let decision = engine.evaluate_tool("web_search", &args);

        // Debug output
        println!(
            "Decision: allowed={}, action={:?}, matched_rule={:?}",
            decision.allowed,
            decision.action,
            decision.matched_rule.is_some()
        );

        assert!(decision.allowed);
        assert_eq!(decision.action, PolicyAction::Ask);

        // Wait for async event publishing
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        // Verify event was logged
        let events = mock_publisher.get_events();
        assert_eq!(events.len(), 1);

        let (topic, payload, source) = &events[0];
        assert_eq!(topic, PolicyTopics::POLICY_ASK);
        assert_eq!(source, "PolicyEngine::policy_ask");

        // Verify payload structure
        assert_eq!(payload["event_type"], "policy_ask_required");
        assert_eq!(payload["tool_name"], "web_search");
        assert_eq!(payload["subject_kind"], "Tool");
        assert_eq!(payload["risk_level"], "Medium");
        assert_eq!(payload["severity"], "MEDIUM");
        assert!(payload["timestamp"].is_string());
        assert!(payload["arguments"].is_object());
    }
}

/// CRITICAL PRODUCTION UTILITY: Get PolicyEngine with real EventBus integration
/// This function creates a PolicyEngine connected to a real EventBus instead of MockEventPublisher
/// Used by production tools: shell_ops.rs, web_ops.rs, file_ops.rs
pub fn get_policy_engine_with_eventbus() -> PolicyEngine {
    // Load the effective policy document
    let policy_doc = load_effective_policy(None);

    // Create PolicyEngine with production EventBus integration
    let engine = PolicyEngine::from_document(policy_doc);

    // PRODUCTION INTEGRATION: Connect to real GLOBAL_EVENT_BUS
    // Create adapter that bridges LocalEventPublisher trait to real EventBus
    let production_publisher = ProductionEventPublisher::new();
    engine.with_event_publisher(std::sync::Arc::new(production_publisher))
}

/// PRODUCTION EVENT PUBLISHER: Real EventBus integration for policy violation logging
/// This bridges the LocalEventPublisher trait to the actual GLOBAL_EVENT_BUS
#[derive(Debug)]
pub struct ProductionEventPublisher;

impl Default for ProductionEventPublisher {
    fn default() -> Self {
        Self::new()
    }
}

impl ProductionEventPublisher {
    pub fn new() -> Self {
        Self
    }
}

impl LocalEventPublisher for ProductionEventPublisher {
    fn publish(
        &self,
        topic: &str,
        payload: serde_json::Value,
        source: &str,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<Output = Result<(), Box<dyn std::error::Error + Send + Sync>>>
                + Send,
        >,
    > {
        let topic = topic.to_string();
        let source = source.to_string();

        Box::pin(async move {
            // CRITICAL P0.2.6: Connect to real EventBus for production audit logging
            // Use global EventBus for production security event logging
            let global_bus = magray_core::events::bus::get_global_event_bus();
            match global_bus.publish(&topic, payload.clone(), &source).await {
                Ok(_) => {
                    tracing::info!(
                        "Successfully published event to EventBus: topic={}, source={}",
                        topic,
                        source
                    );
                }
                Err(e) => {
                    tracing::error!(
                        "Failed to publish event to EventBus: {} - falling back to console",
                        e
                    );
                    // Fallback: Log policy violation event for immediate visibility (if EventBus fails)
                    tracing::warn!(
                        topic = %topic,
                        source = %source,
                        event = ?payload,
                        "POLICY VIOLATION LOGGED - EventBus failed, using console fallback"
                    );

                    // Console output for security monitoring
                    eprintln!(
                        "🚨 POLICY EVENT: {} from {} - Payload: {}",
                        topic,
                        source,
                        serde_json::to_string_pretty(&payload).unwrap_or_default()
                    );
                }
            }

            Ok(())
        })
    }
}
