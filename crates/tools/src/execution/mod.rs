// @component: {"k":"M","id":"execution_module","t":"Tool execution module with improved pipeline","m":{"cur":0,"tgt":95,"u":"%"},"f":["execution","module","pipeline","resource"]}

pub mod pipeline;
pub mod resource_manager;
pub mod security_enforcer;

pub use pipeline::{
    ExecutionPipeline, ExecutionResult, ExecutionStrategy, ExecutionContext,
    RetryConfig, CircuitBreakerConfig, CircuitBreakerState
};

pub use resource_manager::{
    ResourceManager, ResourceAllocation, ResourceLimits as ExecResourceLimits,
    ResourceMonitor, ResourceUsage
};

pub use security_enforcer::{
    SecurityEnforcer, ExecutionPermission, SandboxConfig, ProcessIsolation
};