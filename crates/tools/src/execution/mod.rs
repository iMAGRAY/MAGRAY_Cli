// @component: {"k":"M","id":"execution_module","t":"Tool execution module with improved pipeline","m":{"cur":0,"tgt":95,"u":"%"},"f":["execution","module","pipeline","resource"]}

pub mod pipeline;
pub mod resource_manager;
pub mod security_enforcer;

pub use pipeline::{
    CircuitBreakerConfig, CircuitBreakerState, ExecutionContext, ExecutionPipeline,
    ExecutionResult, ExecutionStrategy, RetryConfig,
};

pub use resource_manager::{
    ResourceAllocation, ResourceLimits as ExecResourceLimits, ResourceManager, ResourceMonitor,
    ResourceUsage,
};

pub use security_enforcer::{
    ExecutionPermission, ProcessIsolation, SandboxConfig, SecurityEnforcer,
};
