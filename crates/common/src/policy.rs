use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum PolicyAction {
    Allow,
    Deny,
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
}

impl PolicyEngine {
    pub fn new() -> Self { Self { rules: Vec::new() } }

    pub fn with_rules(mut self, rules: Vec<PolicyRule>) -> Self {
        self.rules = rules;
        self
    }

    pub fn from_document(doc: PolicyDocument) -> Self { Self { rules: doc.rules } }

    pub fn evaluate_tool(&self, tool_name: &str, args: &HashMap<String, String>) -> PolicyDecision {
        self.evaluate(PolicySubjectKind::Tool, tool_name, args)
    }

    pub fn evaluate_command(&self, command: &str, args: &HashMap<String, String>) -> PolicyDecision {
        self.evaluate(PolicySubjectKind::Command, command, args)
    }

    fn evaluate(&self, kind: PolicySubjectKind, name: &str, args: &HashMap<String, String>) -> PolicyDecision {
        for rule in &self.rules {
            if rule.subject_kind != kind { continue; }
            if rule.subject_name != name && rule.subject_name != "*" { continue; }
            if let Some(expected) = &rule.when_contains_args {
                let mut all_match = true;
                for (k, v) in expected {
                    if args.get(k) != Some(v) { all_match = false; break; }
                }
                if !all_match { continue; }
            }
            return PolicyDecision { allowed: matches!(rule.action, PolicyAction::Allow), matched_rule: Some(rule.clone()) };
        }
        // default allow if no rule matched
        PolicyDecision { allowed: true, matched_rule: None }
    }
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
        assert!(d.matched_rule.is_none());
    }

    #[test]
    fn deny_specific_tool() {
        let doc = PolicyDocument { rules: vec![PolicyRule {
            subject_kind: PolicySubjectKind::Tool,
            subject_name: "shell_exec".into(),
            when_contains_args: None,
            action: PolicyAction::Deny,
            reason: Some("Shell exec disabled".into()),
        }]};
        let engine = PolicyEngine::from_document(doc);
        let d = engine.evaluate_tool("shell_exec", &HashMap::new());
        assert!(!d.allowed);
        assert_eq!(d.matched_rule.unwrap().reason.unwrap(), "Shell exec disabled");
    }

    #[test]
    fn allow_with_args_match() {
        let doc = PolicyDocument { rules: vec![PolicyRule {
            subject_kind: PolicySubjectKind::Tool,
            subject_name: "file_read".into(),
            when_contains_args: Some(HashMap::from([("path".into(), "/etc/hosts".into())])),
            action: PolicyAction::Allow,
            reason: None,
        }]};
        let engine = PolicyEngine::from_document(doc);
        let d1 = engine.evaluate_tool("file_read", &HashMap::from([("path".into(), "/etc/hosts".into())]));
        assert!(d1.allowed);
        let d2 = engine.evaluate_tool("file_read", &HashMap::from([("path".into(), "/etc/passwd".into())]));
        assert!(d2.allowed); // default allow since no rule matched
    }
}