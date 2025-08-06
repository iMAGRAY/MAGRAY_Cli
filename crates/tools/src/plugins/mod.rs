// @component: {"k":"M","id":"plugin_system_module","t":"Plugin system module with WASM and external process support","m":{"cur":0,"tgt":95,"u":"%"},"f":["plugins","module","wasm","external"]}

pub mod wasm_plugin;
pub mod external_process;
pub mod plugin_manager;
pub mod hot_reload;

pub use wasm_plugin::{
    WasmPlugin, WasmRuntime, WasmConfig, WasmPluginError,
    WasmSandbox, WasmResourceLimits
};

pub use external_process::{
    ExternalProcessPlugin, ProcessConfig, ProcessSandbox,
    ProcessResourceLimits, ProcessIsolation as PluginProcessIsolation
};

pub use plugin_manager::{
    PluginManager, PluginRegistry, PluginMetadata, PluginState,
    PluginDependency, PluginVersion, PluginConfiguration
};

pub use hot_reload::{
    HotReloadManager, ReloadEvent, ReloadPolicy, FileWatcher
};