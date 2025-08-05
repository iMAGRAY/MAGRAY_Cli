// @component: {"k":"C","id":"common_lib","t":"Common utilities and logging","m":{"cur":90,"tgt":95,"u":"%"},"f":["common","logging","structured","utils"]}
pub mod structured_logging;

pub use structured_logging::{
    init_structured_logging, 
    LoggingConfig, 
    StructuredLogEntry,
    ExecutionContext,
    PerformanceMetrics,
    OperationTimer,
    RequestContext,
};