// @component: {"k":"M","id":"tool_registry_module","t":"Tool registry module with metadata and security","m":{"cur":0,"tgt":95,"u":"%"},"f":["registry","module","metadata","security"]}

pub mod tool_metadata;
pub mod secure_registry;

pub use tool_metadata::{
    ToolMetadata, SemanticVersion, ToolCategory, ToolPermissions, SecurityLevel,
    FileSystemPermissions, NetworkPermissions, SystemPermissions,
    ResourceRequirements, ToolDependency, PerformanceMetrics, ToolExample
};

pub use secure_registry::{
    SecureToolRegistry, SecurityContext, UserTrustLevel, UserPermissions, 
    ResourceLimits, InputValidator, SecurityConfig, AuditEvent, AuditEventType
};