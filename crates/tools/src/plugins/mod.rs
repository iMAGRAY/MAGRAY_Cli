// @component: {"k":"M","id":"plugin_system_module","t":"Plugin system module with WASM and external process support","m":{"cur":0,"tgt":95,"u":"%"},"f":["plugins","module","wasm","external"]}

pub mod external_process;
pub mod hot_reload;
pub mod plugin_manager;
pub mod wasm_plugin;

pub use wasm_plugin::{
    WasmConfig, WasmPlugin, WasmPluginError, WasmResourceLimits, WasmRuntime, WasmSandbox,
};

pub use external_process::{
    ExternalProcessPlugin, ProcessConfig, ProcessIsolation as PluginProcessIsolation,
    ProcessResourceLimits, ProcessSandbox,
};

pub use plugin_manager::{
    PluginConfiguration, PluginDependency, PluginManager, PluginMetadata, PluginRegistry,
    PluginState, PluginVersion,
};

pub use hot_reload::{FileWatcher, HotReloadManager, ReloadEvent, ReloadPolicy};
