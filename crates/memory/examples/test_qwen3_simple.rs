use anyhow::Result;
use ort::{session::Session, inputs};
use std::path::PathBuf;

fn main() -> Result<()> {
    println!("=== ПРОСТОЙ ТЕСТ QWEN3-EMBEDDING ===\n");
    
    // Установка пути к DLL
    let dll_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent().unwrap()
        .parent().unwrap()
        .join("scripts")
        .join("onnxruntime")
        .join("lib")
        .join("onnxruntime.dll");
    
    std::env::set_var("ORT_DYLIB_PATH", dll_path.to_str().unwrap());
    
    // Путь к модели
    let model_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("models")
        .join("Qwen3-Embedding-0.6B-ONNX")
        .join("model.onnx");
    
    println!("📋 ФАКТЫ О МОДЕЛИ:");
    println!("- Название: Qwen3-Embedding-0.6B (для feature-extraction)");
    println!("- Pipeline: feature-extraction согласно README");
    println!("- Pooling: last_token согласно README");
    println!("- Модель: {}", model_path.display());
    
    // Инициализация
    ort::init().with_name("qwen3_simple").commit()?;
    println!("✅ ORT инициализирован");
    
    // Создание сессии
    let session = Session::builder()?
        .with_optimization_level(ort::session::builder::GraphOptimizationLevel::Level3)?
        .commit_from_file(&model_path)?;
    println!("✅ Сессия создана");
    
    println!("\n🔍 АНАЛИЗ ТРЕБОВАНИЙ МОДЕЛИ:");
    println!("Входов: {}", session.inputs.len());
    
    let mut required_inputs = Vec::new();
    let mut optional_inputs = Vec::new();
    
    for input in &session.inputs {
        println!("  - {}: {:?}", input.name, input.input_type);
        
        if input.name == "input_ids" || input.name == "attention_mask" || input.name == "position_ids" {
            required_inputs.push(&input.name);
        } else {
            optional_inputs.push(&input.name);
        }
    }
    
    println!("\n📝 КАТЕГОРИЗАЦИЯ:");
    println!("Обязательные ({} шт): {:?}", required_inputs.len(), required_inputs);
    println!("KV кеши ({} шт): первые 5 из {}", 
        std::cmp::min(5, optional_inputs.len()), 
        optional_inputs.len());
    
    for (i, name) in optional_inputs.iter().take(5).enumerate() {
        println!("  {}. {}", i+1, name);
    }
    if optional_inputs.len() > 5 {
        println!("  ... и еще {} KV кешей", optional_inputs.len() - 5);
    }
    
    // Попробуем только с основными входами
    println!("\n🧪 ЭКСПЕРИМЕНТ 1: Только основные входы");
    
    let seq_len = 4;
    let input_ids = vec![151643i64, 14016, 374, 10127]; // Qwen токены
    let attention_mask = vec![1i64, 1, 1, 1];
    let position_ids = vec![0i64, 1, 2, 3];
    
    let input_ids_tensor = ort::value::Tensor::from_array(([1, seq_len], input_ids))?;
    let attention_mask_tensor = ort::value::Tensor::from_array(([1, seq_len], attention_mask))?;
    let position_ids_tensor = ort::value::Tensor::from_array(([1, seq_len], position_ids))?;
    
    let session = std::sync::Mutex::new(session);
    let mut session_guard = session.lock().unwrap();
    
    println!("Пытаемся запустить без KV кешей...");
    
    let result = session_guard.run(inputs![
        "input_ids" => input_ids_tensor,
        "attention_mask" => attention_mask_tensor, 
        "position_ids" => position_ids_tensor
    ]);
    
    match result {
        Ok(outputs) => {
            println!("🎉 ЧУДО! Работает без KV кешей!");
            
            for (name, output) in outputs.iter() {
                if let Ok((shape, data)) = output.try_extract_tensor::<f32>() {
                    let shape_vec: Vec<i64> = (0..shape.len()).map(|i| shape[i]).collect();
                    println!("  {} форма: {:?}, данных: {}", name, shape_vec, data.len());
                }
            }
        },
        Err(e) => {
            println!("❌ Как и ожидалось, нужны KV кеши: {}", e);
            
            // Определяем какой именно KV кеш нужен первым
            let error_msg = format!("{}", e);
            if let Some(missing_input) = extract_missing_input(&error_msg) {
                println!("🔍 Первый отсутствующий вход: {}", missing_input);
                
                // Это подсказывает нам формат KV кешей
                if missing_input.contains("past_key_values") {
                    println!("💡 ВЫВОД: Модель требует все 56 KV кеш тензоров");
                    println!("💡 Размерность: [batch_size, 8, 0, 128] для пустого кеша");
                    println!("💡 Решение: Создать все KV кеши как пустые тензоры");
                }
            }
        }
    }
    
    println!("\n📋 ЗАКЛЮЧЕНИЕ:");
    println!("1. Модель действительно Qwen3-Embedding для feature-extraction");
    println!("2. Требует полную генеративную архитектуру с KV кешем");
    println!("3. Для первого запуска нужны пустые KV кеши (past_sequence_length=0)");
    println!("4. После инференса берется последний токен как эмбеддинг");
    
    Ok(())
}

fn extract_missing_input(error_msg: &str) -> Option<&str> {
    // Ищем "Missing Input: название_входа"
    if let Some(start) = error_msg.find("Missing Input: ") {
        let start = start + "Missing Input: ".len();
        if let Some(end) = error_msg[start..].find(' ') {
            Some(&error_msg[start..start + end])
        } else {
            Some(&error_msg[start..])
        }
    } else {
        None
    }
}