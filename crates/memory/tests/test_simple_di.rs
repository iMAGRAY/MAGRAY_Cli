//! Простейший тест DI системы
//! Без сложных зависимостей

use anyhow::Result;
use memory::{default_config, MemoryDIConfigurator};

#[tokio::main]
async fn main() -> Result<()> {
    println!("🧪 Простейший тест DI системы");
    
    // Тест создания конфигурации
    let config = default_config()?;
    println!("✅ Конфигурация создана");
    
    // Тест создания DI контейнера
    let container = MemoryDIConfigurator::configure_minimal(config).await?;
    println!("✅ DI контейнер создан с {} зависимостями", container.stats().total_types);
    
    println!("🎉 DI система базово работает!");
    
    Ok(())
}