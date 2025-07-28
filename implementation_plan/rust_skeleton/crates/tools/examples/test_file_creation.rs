// Тест для проверки создания файлов через инструменты
use std::collections::HashMap;
use tools::{ToolInput, file_ops::FileWriter, Tool};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("Тестируем создание файла через FileWriter...");
    
    let writer = FileWriter::new();
    
    let mut args = HashMap::new();
    args.insert("path".to_string(), "test_created_file.txt".to_string());
    args.insert("content".to_string(), "Привет, мир! Это тестовый файл.".to_string());
    
    let input = ToolInput {
        command: "file_write".to_string(),
        args,
        context: Some("Тест создания файла".to_string()),
    };
    
    match writer.execute(input).await {
        Ok(output) => {
            println!("✓ Успех: {}", output.result);
            if let Some(formatted) = output.formatted_output {
                println!("{}", formatted);
            }
        }
        Err(e) => {
            println!("✗ Ошибка: {}", e);
        }
    }
    
    Ok(())
}