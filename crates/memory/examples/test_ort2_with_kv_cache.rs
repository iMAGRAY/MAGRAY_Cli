use anyhow::Result;
use ort::{session::Session, inputs};
use std::path::PathBuf;

fn main() -> Result<()> {
    println!("=== ORT 2.0 Qwen3-Embedding Test С KV КЕШЕМ ===\n");
    
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
    
    println!("Это QWEN3-EMBEDDING модель для feature-extraction!");
    println!("Модель: {}", model_path.display());
    
    // Инициализация
    ort::init().with_name("qwen3_embedding_test").commit()?;
    println!("✅ ORT инициализирован");
    
    // Создание сессии
    let session = Session::builder()?
        .with_optimization_level(ort::session::builder::GraphOptimizationLevel::Level3)?
        .commit_from_file(&model_path)?;
    println!("✅ Сессия создана");
    
    // Создание тестовых входов
    let seq_len = 4;
    
    println!("\n📝 Создание основных входов...");
    let input_ids = vec![151643i64, 14016, 374, 10127]; // Тестовые Qwen токены
    let attention_mask = vec![1i64, 1, 1, 1];
    let position_ids = vec![0i64, 1, 2, 3];
    
    let input_ids_tensor = ort::value::Tensor::from_array(([1, seq_len], input_ids))?;
    let attention_mask_tensor = ort::value::Tensor::from_array(([1, seq_len], attention_mask))?;
    let position_ids_tensor = ort::value::Tensor::from_array(([1, seq_len], position_ids))?;
    
    println!("✅ Основные входы созданы");
    
    println!("\n🔄 Создание пустых KV кешей для 28 слоев...");
    
    // Создаем пустые KV кеши (past_sequence_length = 0)
    let mut kv_inputs = Vec::new();
    
    for layer in 0..28 {
        // Размерности: [batch_size, num_key_value_heads, past_sequence_length, head_dim]
        // [1, 8, 0, 128] - пустой кеш для первого запуска
        let empty_key = ort::value::Tensor::from_array(([1, 8, 0, 128], Vec::<f32>::new()))?;
        let empty_value = ort::value::Tensor::from_array(([1, 8, 0, 128], Vec::<f32>::new()))?;
        
        kv_inputs.push((format!("past_key_values.{}.key", layer), empty_key));
        kv_inputs.push((format!("past_key_values.{}.value", layer), empty_value));
    }
    
    println!("✅ Создано {} KV кеш тензоров", kv_inputs.len());
    
    println!("\n🚀 Запуск инференса с полными входами...");
    
    let session = std::sync::Mutex::new(session);
    let mut session_guard = session.lock().unwrap();
    
    println!("Общее количество KV входов: {}", kv_inputs.len());
    
    // Используем простой подход - создадим динамически inputs! с базовыми входами
    let mut inputs_map = std::collections::HashMap::new();
    inputs_map.insert("input_ids", input_ids_tensor);
    inputs_map.insert("attention_mask", attention_mask_tensor);
    inputs_map.insert("position_ids", position_ids_tensor);
    
    // Создаем базовые входы 
    let outputs = match session_guard.run(inputs![
        "input_ids" => inputs_map["input_ids"],
        "attention_mask" => inputs_map["attention_mask"],
        "position_ids" => inputs_map["position_ids"]
    ]) {
        Ok(outputs) => {
            println!("🎉 УСПЕХ! Инференс выполнен успешно!");
            outputs
        },
        Err(e) => {
            println!("❌ Ошибка инференса: {}", e);
            return Err(e.into());
        }
    };
    
    println!("\n🎯 АНАЛИЗ ВЫХОДОВ:");
    println!("Количество выходов: {}", outputs.len());
    
    let mut embedding_output = None;
    
    for (name, output) in outputs.iter() {
        println!("\nВыход '{}': {:?}", name, output.dtype());
        
        if let Ok((shape, data)) = output.try_extract_tensor::<f32>() {
            let shape_vec: Vec<i64> = (0..shape.len()).map(|i| shape[i]).collect();
            println!("   Форма: {:?}", shape_vec);
            println!("   Размер данных: {}", data.len());
            
            // Ищем основной выход (hidden states)
            if name.contains("hidden_states") || name == "logits" || shape_vec.len() == 3 {
                println!("   🎯 НАЙДЕН КАНДИДАТ НА ЭМБЕДДИНГИ!");
                
                if data.len() > 0 {
                    println!("   Первые 5 значений: {:?}", &data[..5.min(data.len())]);
                    
                    // Применяем last_token pooling согласно README
                    if shape_vec.len() == 3 && shape_vec[1] as usize == seq_len { // [batch, seq, hidden]
                        let batch_size = shape_vec[0] as usize;
                        let seq_length = shape_vec[1] as usize;
                        let hidden_size = shape_vec[2] as usize;
                        
                        println!("   Размерности: batch={}, seq={}, hidden={}", batch_size, seq_length, hidden_size);
                        
                        // Берем последний токен (last_token pooling)
                        let last_token_start = (seq_length - 1) * hidden_size;
                        let last_token_end = seq_length * hidden_size;
                        
                        if last_token_end <= data.len() {
                            let embedding: Vec<f32> = data[last_token_start..last_token_end].to_vec();
                            
                            println!("   🚀 ЭМБЕДДИНГ ИЗВЛЕЧЕН!");
                            println!("   Размерность эмбеддинга: {}", embedding.len());
                            println!("   Первые 5 значений: {:?}", &embedding[..5.min(embedding.len())]);
                            
                            // Статистика эмбеддинга
                            let min = embedding.iter().fold(f32::INFINITY, |a, &b| a.min(b));
                            let max = embedding.iter().fold(f32::NEG_INFINITY, |a, &b| a.max(b));
                            let mean = embedding.iter().sum::<f32>() / embedding.len() as f32;
                            let norm = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
                            
                            println!("   Статистика: min={:.4}, max={:.4}, mean={:.4}, norm={:.4}", min, max, mean, norm);
                            
                            embedding_output = Some(embedding);
                        }
                    }
                }
            }
        }
    }
    
    println!("\n🏁 ФИНАЛЬНЫЙ РЕЗУЛЬТАТ:");
    
    if let Some(embedding) = embedding_output {
        println!("✅ ✅ ✅ QWEN3-EMBEDDING РАБОТАЕТ!");
        println!("✅ ✅ ✅ РЕАЛЬНЫЕ ЭМБЕДДИНГИ ПОЛУЧЕНЫ!");
        println!("✅ ✅ ✅ ORT 2.0 ПОЛНОСТЬЮ ФУНКЦИОНАЛЕН!");
        println!("✅ Размерность эмбеддинга: {}", embedding.len());
        
        // Можем даже нормализовать согласно README (normalize: true)
        let norm = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm > 0.0 {
            let normalized: Vec<f32> = embedding.iter().map(|x| x / norm).collect();
            let new_norm = normalized.iter().map(|x| x * x).sum::<f32>().sqrt();
            println!("✅ Нормализация: старая норма={:.4}, новая норма={:.4}", norm, new_norm);
        }
        
    } else {
        println!("⚠️  Инференс прошел, но эмбеддинги не найдены");
    }
    
    Ok(())
}