use anyhow::Result as AnyResult;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

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

#[derive(Debug, Clone, Default)]
pub struct PolicyEngine {
    rules: Vec<PolicyRule>,
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
        Self { rules: Vec::new() }
    }

    pub fn with_rules(mut self, rules: Vec<PolicyRule>) -> Self {
        self.rules = rules;
        self
    }

    pub fn from_document(doc: PolicyDocument) -> Self {
        Self { rules: doc.rules }
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

    fn evaluate(
        &self,
        kind: PolicySubjectKind,
        name: &str,
        args: &HashMap<String, String>,
    ) -> PolicyDecision {
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
        if let Some(rule) = last_match.clone() {
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
            PolicyDecision {
                allowed: true,
                matched_rule: None,
                action: PolicyAction::Allow,
                risk: RiskLevel::Low,
            }
        }
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
            if cfg.fs.roots.is_empty() {
                return false;
            }
            reqs.iter()
                .all(|r| cfg.fs.roots.iter().any(|a| r.starts_with(a)))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_allow() {
        let engine = PolicyEngine::new();
        let args = HashMap::from([("path".into(), "/tmp".into())]);
        let d = engine.evaluate_tool("file_read", &args);
        assert!(d.allowed);
        assert_eq!(d.action, PolicyAction::Allow);
    }

    #[test]
    fn default_document_blocks_shell_exec() {
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
            d.matched_rule.unwrap().reason.unwrap(),
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
        assert!(d2.allowed); // default allow since no rule matched
        assert_eq!(d2.action, PolicyAction::Allow);
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
        let tmp = tempfile::TempDir::new().unwrap();
        let p = tmp.path().join("policy.json");
        fs::write(&p, r#"{"rules":[{"subject_kind":"Tool","subject_name":"shell_exec","when_contains_args":null,"action":"Deny"}]}"#).unwrap();
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
}
