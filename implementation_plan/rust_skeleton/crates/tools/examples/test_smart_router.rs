// Тест SmartRouter для создания файлов
use llm::LlmClient;
use tools::SmartRouter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("Тестируем SmartRouter для создания файла...");
    
    // Создаем mock LLM клиент (для теста)
    let llm_client = match LlmClient::from_env() {
        Ok(client) => client,
        Err(_) => {
            println!("⚠️ LLM клиент недоступен, создаем mock");
            return Ok(());
        }
    };
    
    let router = SmartRouter::new(llm_client);
    
    let test_queries = vec![
        "создай файл hello.txt с текстом привет мир",
        "создай папку test_dir",
        "покажи содержимое текущей папки",
    ];
    
    for query in test_queries {
        println!("\n[TEST] Запрос: {}", query);
        
        match router.process_smart_request(query).await {
            Ok(result) => {
                println!("✓ Результат:\n{}", result);
            }
            Err(e) => {
                println!("✗ Ошибка: {}", e);
            }
        }
        
        println!("{}", "─".repeat(50));
    }
    
    Ok(())
}