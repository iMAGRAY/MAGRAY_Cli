//! Минимальная локальная реализация совместимости DI для памяти (production‑минимум)
//!
//! Обеспечивает:
//! - Инициализацию локального каталога ~/.magray/memory
//! - Хранение записей в формате JSONL (records.jsonl)
//! - Простейший поиск по подстроке (case-insensitive)
//! - Health check и статистику

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::sync::Arc;

/// Заглушка для DIResolver trait (оставляем для совместимости)
pub trait DIResolver {
    fn resolve<T>(&self) -> Result<Arc<T>>
    where
        T: Send + Sync + 'static;
}

/// Минимальная Legacy конфигурация, ожидаемая CLI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LegacyMemoryConfig {
    pub health_enabled: bool,
    /// Опциональный путь к домашнему каталогу MAGRAY. Если None — используем $HOME/.magray
    pub magray_home: Option<PathBuf>,
}

impl Default for LegacyMemoryConfig {
    fn default() -> Self {
        Self { health_enabled: true, magray_home: None }
    }
}

/// Минимальная структура записи памяти
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryRecord {
    pub id: String,
    pub text: String,
    pub created_ms: i64,
    pub tags: Vec<String>,
}

/// Минимальный статус здоровья
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SystemHealthStatusStub {
    pub healthy: bool,
    pub records: usize,
}

fn magray_home_dir(cfg: &LegacyMemoryConfig) -> PathBuf {
    if let Some(dir) = &cfg.magray_home {
        return dir.clone();
    }
    let mut dir = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
    dir.push(".magray");
    dir
}

fn memory_dir(cfg: &LegacyMemoryConfig) -> PathBuf {
    let mut dir = magray_home_dir(cfg);
    dir.push("memory");
    dir
}

fn records_path(cfg: &LegacyMemoryConfig) -> PathBuf {
    let mut p = memory_dir(cfg);
    p.push("records.jsonl");
    p
}

/// Production‑минимум: локальный файловый сервис памяти
pub struct DIMemoryService {
    cfg: LegacyMemoryConfig,
}

impl DIMemoryService {
    pub async fn new(config: LegacyMemoryConfig) -> Result<Self> {
        let svc = Self { cfg: config };
        svc.ensure_fs()?;
        Ok(svc)
    }

    fn ensure_fs(&self) -> Result<()> {
        let mem_dir = memory_dir(&self.cfg);
        fs::create_dir_all(&mem_dir)?;
        let rec_path = records_path(&self.cfg);
        if !rec_path.exists() {
            OpenOptions::new().create(true).append(true).open(&rec_path)?;
        }
        Ok(())
    }

    pub async fn initialize(&self) -> Result<()> { Ok(()) }

    pub async fn check_health(&self) -> Result<SystemHealthStatusStub> {
        if !self.cfg.health_enabled {
            return Ok(SystemHealthStatusStub { healthy: true, records: 0 });
        }
        let recs = self.count_records()?;
        Ok(SystemHealthStatusStub { healthy: true, records: recs })
    }

    fn count_records(&self) -> Result<usize> {
        let path = records_path(&self.cfg);
        if !path.exists() { return Ok(0); }
        let f = OpenOptions::new().read(true).open(path)?;
        let reader = BufReader::new(f);
        Ok(reader.lines().count())
    }

    /// Сохранить запись в память
    pub async fn store(&self, text: &str, tags: Vec<String>) -> Result<String> {
        let id = uuid::Uuid::new_v4().to_string();
        let rec = MemoryRecord {
            id: id.clone(),
            text: text.to_string(),
            created_ms: chrono::Utc::now().timestamp_millis(),
            tags,
        };
        let line = serde_json::to_string(&rec)?;
        let mut f = OpenOptions::new().create(true).append(true).open(records_path(&self.cfg))?;
        writeln!(f, "{}", line)?;
        Ok(id)
    }

    /// Простой поиск по подстроке (case-insensitive), возвращает первые top_k совпадений
    pub async fn search(&self, query: &str, top_k: usize) -> Result<Vec<MemoryRecord>> {
        if top_k == 0 { return Ok(Vec::new()); }
        let path = records_path(&self.cfg);
        if !path.exists() { return Ok(Vec::new()); }
        let f = OpenOptions::new().read(true).open(path)?;
        let reader = BufReader::new(f);
        let q = query.to_lowercase();
        let mut hits = Vec::new();
        for line in reader.lines() {
            let line = line?;
            if line.trim().is_empty() { continue; }
            if let Ok(rec) = serde_json::from_str::<MemoryRecord>(&line) {
                if rec.text.to_lowercase().contains(&q) || rec.tags.iter().any(|t| t.to_lowercase().contains(&q)) {
                    hits.push(rec);
                    if hits.len() >= top_k { break; }
                }
            }
        }
        Ok(hits)
    }
}

// Traits module compatibility
pub mod traits {
    pub use super::DIResolver;
}

// Container core compatibility
pub mod container_core {
    pub struct ContainerCore;
    impl ContainerCore { pub fn new() -> Self { Self } }
}

// Type safe resolver compatibility
pub struct TypeSafeResolver;
impl TypeSafeResolver { pub fn new() -> Self { Self } }

// Пустой конфигуратор (совместимость)
pub struct UnifiedMemoryConfigurator;
impl UnifiedMemoryConfigurator { pub fn new() -> Self { Self } }
