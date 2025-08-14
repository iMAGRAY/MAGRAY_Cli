// @component: {"k":"M","id":"execution_module","t":"Tool execution module with improved pipeline","m":{"cur":0,"tgt":95,"u":"%"},"f":["execution","module","pipeline","resource"]}

pub mod pipeline;
pub mod resource_manager;
pub mod security_enforcer;
// P1.2.5: JSON-RPC Subprocess Framework
pub mod subprocess_runner;
// P1.2.6: Dry-run Support
pub mod dry_run;
// P1.2.7: Auto-diff Support  
pub mod auto_diff;
// P1.2.8: Tool Signing
pub mod tool_signing;
// P1.2.9: Timeout Management
pub mod timeout_manager;
// P1.2.10: Telemetry System
pub mod telemetry;

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

pub use subprocess_runner::{
    JsonRpcRequest, JsonRpcResponse, ProcessLifecycleManager, SubprocessConfig,
    SubprocessError, SubprocessRunner,
};

pub use dry_run::{
    Change, ChangeType, DryRunExecutor, DryRunResult, Risk, RiskCategory, SafetyAssessment,
};

pub use auto_diff::{
    AutoDiffEngine, DiffResult, FileChange, FileChangeType, FileSystemSnapshot,
};

pub use tool_signing::{
    SignedToolManifest, SigningCertificate, ToolSignature, ToolSigner, VerificationResult,
};

pub use timeout_manager::{
    OperationStatus, TimeoutConfig, TimeoutManager, TimeoutReason, TimeoutResult,
};

pub use telemetry::{
    HealthStatus, MetricsSnapshot, SecurityEventType, SecuritySeverity, TelemetryCollector,
    TelemetryEvent, ToolUsageMetrics,
};
