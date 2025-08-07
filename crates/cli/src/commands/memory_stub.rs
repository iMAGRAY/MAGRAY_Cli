use anyhow::Result;
use clap::Args;

/// Заглушка команды управления памятью (недоступна в минимальной сборке)
#[derive(Debug, Args)]
pub struct MemoryCommand {}

impl MemoryCommand {
    pub async fn execute(self) -> Result<()> {
        println!("[memory] Команда недоступна в минимальной сборке. Запустите с --features cpu.");
        Ok(())
    }
}