// Comprehensive tests for P1.2.3 Capability System
// Tests all capability types, permission checking, and security enforcement

use std::path::PathBuf;
use tools::capabilities::checker::*;
// use tools::capabilities::validation::*; // Unused import removed
use tools::capabilities::*;

#[cfg(test)]
mod capability_system_tests {
    use super::*;

    #[test]
    fn test_capability_enum_creation() {
        let fs_cap = Capability::Filesystem {
            mode: AccessMode::Read,
            paths: vec![PathBuf::from("/tmp")],
        };

        assert_eq!(fs_cap.risk_level(), 3);
        assert!(fs_cap.description().contains("Filesystem"));
    }

    #[test]
    fn test_network_capability() {
        let net_cap = Capability::Network {
            mode: NetworkMode::Outbound,
            domains: vec!["example.com".to_string()],
        };

        assert_eq!(net_cap.risk_level(), 5);
        assert!(net_cap.description().contains("Network"));
    }

    #[test]
    fn test_shell_capability_elevated() {
        let shell_cap = Capability::Shell {
            commands: vec!["sudo".to_string()],
            elevated: true,
        };

        assert_eq!(shell_cap.risk_level(), 9);
        assert!(shell_cap.description().contains("Shell"));
    }

    #[test]
    fn test_ui_capability() {
        let ui_cap = Capability::UI {
            modes: vec![UIMode::Display, UIMode::Input],
        };

        assert_eq!(ui_cap.risk_level(), 4);
    }

    #[test]
    fn test_memory_capability() {
        let memory_cap = Capability::Memory { max_mb: 512 };
        assert_eq!(memory_cap.risk_level(), 2);

        let high_memory_cap = Capability::Memory { max_mb: 2048 };
        assert_eq!(high_memory_cap.risk_level(), 5);
    }

    #[test]
    fn test_compute_capability() {
        let compute_cap = Capability::Compute {
            max_cpu_percent: 50,
            max_duration_ms: 30000,
        };
        assert_eq!(compute_cap.risk_level(), 3);

        let high_compute_cap = Capability::Compute {
            max_cpu_percent: 90,
            max_duration_ms: 300000,
        };
        assert_eq!(high_compute_cap.risk_level(), 6);
    }

    #[test]
    fn test_capability_conflicts() {
        let fs1 = Capability::Filesystem {
            mode: AccessMode::Read,
            paths: vec![PathBuf::from("/tmp")],
        };

        let fs2 = Capability::Filesystem {
            mode: AccessMode::Write,
            paths: vec![PathBuf::from("/tmp")],
        };

        assert!(fs1.conflicts_with(&fs2));

        let fs3 = Capability::Filesystem {
            mode: AccessMode::Read,
            paths: vec![PathBuf::from("/var")],
        };

        assert!(!fs1.conflicts_with(&fs3));
    }

    #[test]
    fn test_capability_spec_creation() {
        let spec = CapabilitySpec::new(vec![
            Capability::Filesystem {
                mode: AccessMode::Read,
                paths: vec![PathBuf::from(".")],
            },
            Capability::Network {
                mode: NetworkMode::Outbound,
                domains: vec!["api.example.com".to_string()],
            },
        ])
        .with_justification("Tool needs to read files and make API calls".to_string());

        assert_eq!(spec.required.len(), 2);
        assert_eq!(spec.optional.len(), 0);
        assert!(!spec.justification.is_empty());
        assert_eq!(spec.all_capabilities().len(), 2);
    }

    #[test]
    fn test_capability_spec_minimal_check() {
        let minimal_spec = CapabilitySpec::new(vec![Capability::Memory { max_mb: 64 }])
            .with_justification("Small memory requirement for basic operation".to_string());

        assert!(minimal_spec.is_minimal());

        let complex_spec = CapabilitySpec::new(vec![
            Capability::Filesystem {
                mode: AccessMode::ReadWrite,
                paths: vec![PathBuf::from("/"), PathBuf::from("/tmp")],
            },
            Capability::Network {
                mode: NetworkMode::Both,
                domains: vec!["*".to_string()],
            },
            Capability::Shell {
                commands: vec!["*".to_string()],
                elevated: true,
            },
            Capability::UI {
                modes: vec![UIMode::Display, UIMode::Input, UIMode::Notification],
            },
        ]);

        assert!(!complex_spec.is_minimal());
    }

    #[test]
    fn test_default_capability_checker() {
        let mut checker = DefaultCapabilityChecker::new();

        let safe_capability = Capability::Memory { max_mb: 100 };
        assert!(checker
            .check_capability(&safe_capability)
            .expect("Test operation should succeed"));

        assert!(checker.grant_capability(safe_capability.clone()).is_ok());
        assert!(checker.has_capability(&safe_capability));
    }

    #[test]
    fn test_capability_policy_strict() {
        let strict_policy = CapabilityPolicy::strict();
        let checker = DefaultCapabilityChecker::with_policy(strict_policy);

        let dangerous_capability = Capability::Shell {
            commands: vec!["rm".to_string()],
            elevated: true,
        };

        assert!(checker.check_capability(&dangerous_capability).is_err());
    }

    #[test]
    fn test_capability_policy_permissive() {
        let permissive_policy = CapabilityPolicy::permissive();
        let checker = DefaultCapabilityChecker::with_policy(permissive_policy);

        let capability = Capability::Shell {
            commands: vec!["ls".to_string()],
            elevated: true,
        };

        assert!(checker
            .check_capability(&capability)
            .expect("Test operation should succeed"));
    }

    #[test]
    fn test_filesystem_path_validation() {
        let policy = CapabilityPolicy::default();
        let checker = DefaultCapabilityChecker::with_policy(policy);

        let valid_fs_cap = Capability::Filesystem {
            mode: AccessMode::Read,
            paths: vec![PathBuf::from(".")],
        };

        assert!(checker
            .check_capability(&valid_fs_cap)
            .expect("Test operation should succeed"));

        let invalid_fs_cap = Capability::Filesystem {
            mode: AccessMode::Write,
            paths: vec![PathBuf::from("/etc/passwd")],
        };

        assert!(checker.check_capability(&invalid_fs_cap).is_err());
    }

    #[test]
    fn test_network_domain_validation() {
        let policy = CapabilityPolicy {
            allowed_domains: vec!["example.com".to_string()],
            ..CapabilityPolicy::default()
        };
        let checker = DefaultCapabilityChecker::with_policy(policy);

        let valid_net_cap = Capability::Network {
            mode: NetworkMode::Outbound,
            domains: vec!["example.com".to_string()],
        };

        assert!(checker
            .check_capability(&valid_net_cap)
            .expect("Test operation should succeed"));

        let invalid_net_cap = Capability::Network {
            mode: NetworkMode::Outbound,
            domains: vec!["malicious.com".to_string()],
        };

        assert!(checker.check_capability(&invalid_net_cap).is_err());
    }

    #[test]
    fn test_capability_utils_spec_checking() {
        let checker = DefaultCapabilityChecker::new();

        let spec = CapabilitySpec::new(vec![
            Capability::Memory { max_mb: 128 },
            Capability::UI {
                modes: vec![UIMode::Display],
            },
        ]);

        let missing = CapabilityUtils::check_capability_spec(&checker, &spec)
            .expect("Test operation should succeed");
        assert!(missing.is_empty());
    }

    #[test]
    fn test_capability_utils_default_specs() {
        let file_reader_spec = CapabilityUtils::default_capabilities_for_tool_type("file_reader");
        assert!(!file_reader_spec.required.is_empty());
        assert!(file_reader_spec.justification.contains("read files"));

        let web_scraper_spec = CapabilityUtils::default_capabilities_for_tool_type("web_scraper");
        assert!(!web_scraper_spec.required.is_empty());
        assert!(web_scraper_spec.justification.contains("access websites"));

        let shell_executor_spec =
            CapabilityUtils::default_capabilities_for_tool_type("shell_executor");
        assert!(!shell_executor_spec.required.is_empty());
        assert!(shell_executor_spec.justification.contains("shell commands"));
    }

    #[test]
    fn test_resource_limit_enforcement() {
        let policy = CapabilityPolicy::default();
        let checker = DefaultCapabilityChecker::with_policy(policy);

        let within_limit = Capability::Memory { max_mb: 256 };
        assert!(checker
            .check_capability(&within_limit)
            .expect("Test operation should succeed"));

        let exceeds_limit = Capability::Memory { max_mb: 1024 };
        assert!(checker.check_capability(&exceeds_limit).is_err());
    }

    #[test]
    fn test_capability_spec_builder_pattern() {
        let spec = CapabilitySpec::default()
            .require(Capability::Memory { max_mb: 128 })
            .optional(Capability::UI {
                modes: vec![UIMode::Display],
            })
            .with_justification("Test tool with memory and optional UI".to_string());

        assert_eq!(spec.required.len(), 1);
        assert_eq!(spec.optional.len(), 1);
        assert!(spec.justification.contains("Test tool"));
    }

    #[test]
    fn test_multiple_capability_request() {
        let mut checker = DefaultCapabilityChecker::new();

        let capabilities = vec![
            Capability::Memory { max_mb: 64 },
            Capability::UI {
                modes: vec![UIMode::Display],
            },
        ];

        assert!(checker.request_capabilities(capabilities.clone()).is_ok());

        for cap in &capabilities {
            assert!(checker.has_capability(cap));
        }
    }

    #[test]
    fn test_capability_error_types() {
        use std::error::Error;

        let denied_error = CapabilityError::Denied {
            capability: Capability::Memory { max_mb: 1024 },
        };

        assert!(denied_error.source().is_none());
        assert!(!denied_error.to_string().is_empty());

        let policy_error = CapabilityError::PolicyViolation {
            details: "Test violation".to_string(),
        };

        assert!(policy_error.to_string().contains("Test violation"));
    }

    #[test]
    fn test_capability_access_modes() {
        let read_mode = AccessMode::Read;
        let _write_mode = AccessMode::Write;
        let _readwrite_mode = AccessMode::ReadWrite;
        let execute_mode = AccessMode::Execute;

        // Test that different modes have different characteristics
        let read_cap = Capability::Filesystem {
            mode: read_mode,
            paths: vec![PathBuf::from("/tmp")],
        };

        let exec_cap = Capability::Filesystem {
            mode: execute_mode,
            paths: vec![PathBuf::from("/tmp")],
        };

        assert!(exec_cap.risk_level() > read_cap.risk_level());
    }

    #[test]
    fn test_network_modes() {
        let outbound = NetworkMode::Outbound;
        let _inbound = NetworkMode::Inbound;
        let both = NetworkMode::Both;

        let outbound_cap = Capability::Network {
            mode: outbound,
            domains: vec!["example.com".to_string()],
        };

        let both_cap = Capability::Network {
            mode: both,
            domains: vec!["example.com".to_string()],
        };

        assert!(both_cap.risk_level() >= outbound_cap.risk_level());
    }

    #[test]
    fn test_ui_modes() {
        let display_mode = UIMode::Display;
        let input_mode = UIMode::Input;
        let notification_mode = UIMode::Notification;

        let ui_cap = Capability::UI {
            modes: vec![display_mode, input_mode, notification_mode],
        };

        assert_eq!(ui_cap.risk_level(), 4);
        assert!(ui_cap.description().contains("UI interactions"));
    }

    #[test]
    fn test_capability_serialization() {
        let capability = Capability::Filesystem {
            mode: AccessMode::ReadWrite,
            paths: vec![PathBuf::from("/tmp"), PathBuf::from("/var/tmp")],
        };

        let serialized = serde_json::to_string(&capability).expect("Test operation should succeed");
        let deserialized: Capability =
            serde_json::from_str(&serialized).expect("Test operation should succeed");

        assert_eq!(capability, deserialized);
    }

    #[test]
    fn test_capability_spec_serialization() {
        let spec = CapabilitySpec::new(vec![
            Capability::Memory { max_mb: 256 },
            Capability::Network {
                mode: NetworkMode::Outbound,
                domains: vec!["api.example.com".to_string()],
            },
        ])
        .with_justification("API client tool".to_string());

        let serialized = serde_json::to_string(&spec).expect("Test operation should succeed");
        let deserialized: CapabilitySpec =
            serde_json::from_str(&serialized).expect("Test operation should succeed");

        assert_eq!(spec.required.len(), deserialized.required.len());
        assert_eq!(spec.justification, deserialized.justification);
    }

    #[test]
    fn test_security_enforcement_integration() {
        let checker = DefaultCapabilityChecker::new();

        // Test that security policies are properly enforced
        let high_risk_cap = Capability::Shell {
            commands: vec!["rm".to_string(), "dd".to_string()],
            elevated: true,
        };

        assert!(high_risk_cap.risk_level() >= 9);

        // Should be denied by default policy
        assert!(checker.check_capability(&high_risk_cap).is_err());
    }
}

// Integration tests with manifest system
#[cfg(test)]
mod manifest_integration_tests {
    use super::*;
    use tools::manifest::{ToolManifest, ToolType};

    #[test]
    fn test_manifest_capability_integration() {
        let mut manifest = ToolManifest::new(
            "test_tool".to_string(),
            "1.0.0".to_string(),
            "Test tool".to_string(),
            ToolType::Wasm,
            "test.wasm".to_string(),
            "Test Author".to_string(),
            "MIT".to_string(),
        );

        let capability_spec = CapabilitySpec::new(vec![
            Capability::Memory { max_mb: 128 },
            Capability::Filesystem {
                mode: AccessMode::Read,
                paths: vec![PathBuf::from(".")],
            },
        ]);

        manifest = manifest.with_capability_spec(capability_spec.clone());

        let effective_spec = manifest.effective_capability_spec();
        assert!(effective_spec.required.len() >= capability_spec.required.len());
    }

    #[test]
    fn test_manifest_risk_assessment() {
        let mut manifest = ToolManifest::new(
            "risky_tool".to_string(),
            "1.0.0".to_string(),
            "Dangerous tool".to_string(),
            ToolType::Native,
            "dangerous.exe".to_string(),
            "Test Author".to_string(),
            "MIT".to_string(),
        );

        manifest = manifest.require_capability(Capability::Shell {
            commands: vec!["*".to_string()],
            elevated: true,
        });

        assert!(manifest.requires_elevated_privileges());
        assert!(manifest.max_risk_level() >= 9);
    }

    #[test]
    fn test_manifest_legacy_capability_conversion() {
        use tools::manifest::ToolCapability;

        let mut manifest = ToolManifest::new(
            "legacy_tool".to_string(),
            "1.0.0".to_string(),
            "Legacy tool".to_string(),
            ToolType::Script,
            "script.py".to_string(),
            "Test Author".to_string(),
            "MIT".to_string(),
        );

        manifest = manifest.with_capability(ToolCapability::Filesystem);
        manifest = manifest.with_capability(ToolCapability::Network);

        let effective_spec = manifest.effective_capability_spec();
        assert!(!effective_spec.required.is_empty());

        // Should convert legacy capabilities to enhanced format
        let has_fs = effective_spec
            .required
            .iter()
            .any(|cap| matches!(cap, Capability::Filesystem { .. }));

        let has_net = effective_spec
            .required
            .iter()
            .any(|cap| matches!(cap, Capability::Network { .. }));

        assert!(has_fs);
        assert!(has_net);
    }
}
