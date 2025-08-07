// @component: {"k":"M","id":"tool_registry_module","t":"Tool registry module with metadata and security","m":{"cur":0,"tgt":95,"u":"%"},"f":["registry","module","metadata","security"]}

pub mod secure_registry;
pub mod tool_metadata;

pub use tool_metadata::{
    FileSystemPermissions, NetworkPermissions, PerformanceMetrics, ResourceRequirements,
    SecurityLevel, SemanticVersion, SystemPermissions, ToolCategory, ToolDependency, ToolExample,
    ToolMetadata, ToolPermissions,
};

pub use secure_registry::{
    AuditEvent, AuditEventType, InputValidator, ResourceLimits, SecureToolRegistry, SecurityConfig,
    SecurityContext, UserPermissions, UserTrustLevel,
};
