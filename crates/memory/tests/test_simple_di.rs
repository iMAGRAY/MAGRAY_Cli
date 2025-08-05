//! Простейший тест DI системы
//! Без сложных зависимостей

use anyhow::Result;
use memory::{default_config, MemoryDIConfigurator};

#[tokio::test]
async fn test_simple_di_system() -> Result<()> {
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

// Убираем main функцию, это должен быть тест, а не бинарный файл