
use anyhow::{anyhow, Result};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::{mpsc, RwLock};
use tracing::{debug, error, info, warn};

/// Hot reload events
#[derive(Debug, Clone)]
pub enum ReloadEvent {
    PluginModified { plugin_id: String, path: PathBuf },
    PluginAdded { path: PathBuf },
    PluginRemoved { plugin_id: String },
    ConfigChanged { plugin_id: String, path: PathBuf },
}

/// Reload policies
#[derive(Debug, Clone)]
pub enum ReloadPolicy {
    Automatic, // Reload immediately on change
    Manual,    // Require manual trigger
    Scheduled, // Reload at scheduled intervals
    OnDemand,  // Reload only when plugin is accessed
}

/// File watcher for plugin changes
pub struct FileWatcher {
    watched_paths: Arc<RwLock<HashMap<PathBuf, String>>>, // path -> plugin_id
    #[allow(dead_code)] // Отправляет события перезагрузки
    event_sender: mpsc::UnboundedSender<ReloadEvent>,
    _watcher: tokio::task::JoinHandle<()>,
}

impl FileWatcher {
    pub fn new() -> (Self, mpsc::UnboundedReceiver<ReloadEvent>) {
        let (event_sender, event_receiver) = mpsc::unbounded_channel();
        let watched_paths = Arc::new(RwLock::new(HashMap::new()));

        let watcher_paths = Arc::clone(&watched_paths);
        let watcher_sender = event_sender.clone();

        let watcher = tokio::spawn(async move {
            Self::watch_loop(watcher_paths, watcher_sender).await;
        });

        let file_watcher = Self {
            watched_paths,
            event_sender,
            _watcher: watcher,
        };

        (file_watcher, event_receiver)
    }

    /// Add path to watch list
    pub async fn watch_path(&self, path: PathBuf, plugin_id: String) -> Result<()> {
        if !path.exists() {
            return Err(anyhow!("Path does not exist: {:?}", path));
        }

        let mut paths = self.watched_paths.write().await;
        paths.insert(path.clone(), plugin_id.clone());

        debug!("👁️ Watching path for plugin {}: {:?}", plugin_id, path);
        Ok(())
    }

    /// Remove path from watch list
    pub async fn unwatch_path(&self, path: &Path) -> Result<()> {
        let mut paths = self.watched_paths.write().await;
        paths.remove(path);

        debug!("👁️ Stopped watching path: {:?}", path);
        Ok(())
    }

    /// Watch loop implementation
    async fn watch_loop(
        watched_paths: Arc<RwLock<HashMap<PathBuf, String>>>,
        event_sender: mpsc::UnboundedSender<ReloadEvent>,
    ) {
        let mut interval = tokio::time::interval(Duration::from_secs(1));
        let mut last_modified: HashMap<PathBuf, SystemTime> = HashMap::new();

        loop {
            interval.tick().await;

            let paths = {
                let paths_guard = watched_paths.read().await;
                paths_guard.clone()
            };

            for (path, plugin_id) in paths {
                if let Ok(metadata) = std::fs::metadata(&path) {
                    if let Ok(modified) = metadata.modified() {
                        let should_reload = match last_modified.get(&path) {
                            Some(&last_time) => modified > last_time,
                            None => true, // First time seeing this file
                        };

                        if should_reload {
                            let was_existing =
                                last_modified.insert(path.clone(), modified).is_some();

                            if was_existing {
                                // File was modified (not first time)
                                let event = ReloadEvent::PluginModified {
                                    plugin_id: plugin_id.clone(),
                                    path: path.clone(),
                                };

                                if event_sender.send(event).is_err() {
                                    error!("Failed to send reload event - receiver dropped");
                                    break;
                                }
                            }
                        }
                    }
                } else if last_modified.contains_key(&path) {
                    // File was deleted
                    last_modified.remove(&path);
                    let event = ReloadEvent::PluginRemoved {
                        plugin_id: plugin_id.clone(),
                    };

                    if event_sender.send(event).is_err() {
                        error!("Failed to send reload event - receiver dropped");
                        break;
                    }
                }
            }
        }
    }
}

/// Hot reload manager
pub struct HotReloadManager {
    file_watcher: FileWatcher,
    event_receiver: Arc<tokio::sync::Mutex<mpsc::UnboundedReceiver<ReloadEvent>>>,
    reload_policies: Arc<RwLock<HashMap<String, ReloadPolicy>>>,
    pending_reloads: Arc<RwLock<HashMap<String, ReloadEvent>>>,
    reload_handlers: Arc<RwLock<Vec<Box<dyn ReloadHandler>>>>,
}

impl HotReloadManager {
    pub fn new() -> Self {
        let (file_watcher, event_receiver) = FileWatcher::new();

        Self {
            file_watcher,
            event_receiver: Arc::new(tokio::sync::Mutex::new(event_receiver)),
            reload_policies: Arc::new(RwLock::new(HashMap::new())),
            pending_reloads: Arc::new(RwLock::new(HashMap::new())),
            reload_handlers: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Start hot reload monitoring
    pub async fn start(&self) -> Result<()> {
        info!("🔥 Starting hot reload manager");

        let event_receiver = Arc::clone(&self.event_receiver);
        let reload_handlers = Arc::clone(&self.reload_handlers);

        // Start event processing loop
        tokio::spawn(async move {
            let mut receiver = event_receiver.lock().await;

            while let Some(event) = receiver.recv().await {
                // Handle reload event
                let handlers = reload_handlers.read().await;
                for handler in handlers.iter() {
                    // Extract plugin_id from event
                    let plugin_id = match &event {
                        ReloadEvent::PluginModified { plugin_id, .. } => plugin_id.clone(),
                        ReloadEvent::PluginAdded { path } => path.to_string_lossy().to_string(),
                        ReloadEvent::PluginRemoved { plugin_id } => plugin_id.clone(),
                        ReloadEvent::ConfigChanged { plugin_id, .. } => plugin_id.clone(),
                    };

                    if let Err(e) = handler.handle_reload(&plugin_id).await {
                        error!("Failed to handle reload event: {}", e);
                    }
                }
            }
        });

        Ok(())
    }

    /// Register plugin for hot reload
    pub async fn register_plugin(
        &self,
        plugin_id: String,
        plugin_path: PathBuf,
        policy: ReloadPolicy,
    ) -> Result<()> {
        // Set reload policy
        {
            let mut policies = self.reload_policies.write().await;
            policies.insert(plugin_id.clone(), policy);
        }

        // Start watching plugin files
        self.file_watcher
            .watch_path(plugin_path, plugin_id.clone())
            .await?;

        info!("📝 Registered plugin for hot reload: {}", plugin_id);
        Ok(())
    }

    /// Unregister plugin from hot reload
    pub async fn unregister_plugin(&self, plugin_id: &str, plugin_path: &Path) -> Result<()> {
        // Remove reload policy
        {
            let mut policies = self.reload_policies.write().await;
            policies.remove(plugin_id);
        }

        // Remove pending reloads
        {
            let mut pending = self.pending_reloads.write().await;
            pending.remove(plugin_id);
        }

        // Stop watching files
        self.file_watcher.unwatch_path(plugin_path).await?;

        info!("📝 Unregistered plugin from hot reload: {}", plugin_id);
        Ok(())
    }

    /// Add reload handler
    pub async fn add_reload_handler(&self, handler: Box<dyn ReloadHandler>) {
        let mut handlers = self.reload_handlers.write().await;
        handlers.push(handler);
    }

    /// Trigger manual reload for plugin
    pub async fn trigger_reload(&self, plugin_id: &str) -> Result<()> {
        let policies = self.reload_policies.read().await;
        let policy = policies
            .get(plugin_id)
            .ok_or_else(|| anyhow!("Plugin not registered for hot reload: {}", plugin_id))?;

        match policy {
            ReloadPolicy::Manual | ReloadPolicy::OnDemand => {
                self.execute_reload(plugin_id).await?;
            }
            _ => {
                warn!(
                    "Manual reload triggered for plugin with automatic policy: {}",
                    plugin_id
                );
                self.execute_reload(plugin_id).await?;
            }
        }

        Ok(())
    }

    /// Get pending reloads
    pub async fn get_pending_reloads(&self) -> HashMap<String, ReloadEvent> {
        let pending = self.pending_reloads.read().await;
        pending.clone()
    }

    /// Event processing loop

    /// Handle reload event
    #[allow(dead_code)] // Метод для обработки событий перезагрузки
    async fn handle_reload_event(&self, event: ReloadEvent) -> Result<()> {
        let plugin_id = match &event {
            ReloadEvent::PluginModified { plugin_id, .. } => plugin_id,
            ReloadEvent::PluginAdded { .. } => return Ok(()), // Handled separately
            ReloadEvent::PluginRemoved { plugin_id } => plugin_id,
            ReloadEvent::ConfigChanged { plugin_id, .. } => plugin_id,
        };

        debug!("🔄 Processing reload event for plugin: {}", plugin_id);

        let policies = self.reload_policies.read().await;
        let policy = policies
            .get(plugin_id)
            .cloned()
            .unwrap_or(ReloadPolicy::Manual);

        match policy {
            ReloadPolicy::Automatic => {
                self.execute_reload(plugin_id).await?;
            }
            ReloadPolicy::Manual | ReloadPolicy::OnDemand => {
                let mut pending = self.pending_reloads.write().await;
                pending.insert(plugin_id.clone(), event);
            }
            ReloadPolicy::Scheduled => {
                let mut pending = self.pending_reloads.write().await;
                pending.insert(plugin_id.clone(), event);
            }
        }

        Ok(())
    }

    /// Execute plugin reload
    async fn execute_reload(&self, plugin_id: &str) -> Result<()> {
        info!("🔄 Executing reload for plugin: {}", plugin_id);

        let handlers = self.reload_handlers.read().await;

        for handler in handlers.iter() {
            if let Err(e) = handler.handle_reload(plugin_id).await {
                error!("Reload handler failed for plugin {}: {}", plugin_id, e);
                // Continue with other handlers
            }
        }

        // Remove from pending reloads
        {
            let mut pending = self.pending_reloads.write().await;
            pending.remove(plugin_id);
        }

        Ok(())
    }
}

/// Trait for handling plugin reloads
#[async_trait::async_trait]
pub trait ReloadHandler: Send + Sync {
    async fn handle_reload(&self, plugin_id: &str) -> Result<()>;
}

/// Plugin manager reload handler
pub struct PluginManagerReloadHandler {
    // This would hold a reference to the plugin manager
}

#[async_trait::async_trait]
impl ReloadHandler for PluginManagerReloadHandler {
    async fn handle_reload(&self, plugin_id: &str) -> Result<()> {
        info!("🔄 Plugin manager handling reload for: {}", plugin_id);

        // In a real implementation, this would:
        // 1. Stop the current plugin instance
        // 2. Reload the plugin from disk
        // 3. Start the new instance
        // 4. Update any references

        Ok(())
    }
}

impl Default for HotReloadManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_hot_reload_manager_creation() {
        let manager = HotReloadManager::new();
        let pending = manager.get_pending_reloads().await;
        assert!(pending.is_empty());
    }

    #[tokio::test]
    async fn test_file_watcher_creation() {
        let (watcher, _receiver) = FileWatcher::new();

        let temp_dir = TempDir::new().unwrap();
        let temp_file = temp_dir.path().join("test.txt");
        std::fs::write(&temp_file, "test content").unwrap();

        assert!(watcher
            .watch_path(temp_file.clone(), "test_plugin".to_string())
            .await
            .is_ok());
        assert!(watcher.unwatch_path(&temp_file).await.is_ok());
    }

    #[tokio::test]
    async fn test_plugin_registration() {
        let manager = HotReloadManager::new();

        let temp_dir = TempDir::new().unwrap();
        let plugin_path = temp_dir.path().join("plugin.wasm");
        std::fs::write(&plugin_path, "fake wasm content").unwrap();

        let result = manager
            .register_plugin(
                "test_plugin".to_string(),
                plugin_path,
                ReloadPolicy::Automatic,
            )
            .await;

        assert!(result.is_ok());
    }
}
