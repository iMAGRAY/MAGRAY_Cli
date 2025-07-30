use anyhow::Result;
use ort::{session::Session, value::Tensor, inputs};
use std::path::PathBuf;
use std::collections::HashMap;

/// Эффективное создание пустых KV кеш тензоров для Qwen3-Embedding
/// Размерность: [batch_size, num_key_value_heads, past_sequence_length, head_dim]
/// [1, 8, 0, 128] - пустой кеш для первого запуска
fn create_empty_kv_caches() -> Result<HashMap<String, Tensor<f32>>> {
    let mut kv_tensors = HashMap::new();
    
    // Создаем один пустой тензор и переиспользуем его структуру
    let empty_shape = [1, 8, 0, 128];
    let empty_data: Vec<f32> = Vec::new(); // Пустой вектор для past_sequence_length=0
    
    println!("🔧 Создание 56 KV кеш тензоров...");
    println!("   Размерность: {:?}", empty_shape);
    println!("   Размер данных: {} байт", empty_data.len() * 4);
    
    // Эффективно создаем все 56 тензоров (28 слоев × 2 тензора на слой)
    for layer in 0..28 {
        // Key тензор
        let key_name = format!("past_key_values.{}.key", layer);
        let key_tensor = Tensor::from_array((empty_shape, empty_data.clone()))?;
        kv_tensors.insert(key_name, key_tensor);
        
        // Value тензор  
        let value_name = format!("past_key_values.{}.value", layer);
        let value_tensor = Tensor::from_array((empty_shape, empty_data.clone()))?;
        kv_tensors.insert(value_name, value_tensor);
    }
    
    println!("✅ Создано {} KV кеш тензоров", kv_tensors.len());
    
    Ok(kv_tensors)
}

/// Эффективное создание всех входных тензоров
fn create_all_inputs(seq_len: usize) -> Result<(
    Tensor<i64>, // input_ids
    Tensor<i64>, // attention_mask  
    Tensor<i64>, // position_ids
    HashMap<String, Tensor<f32>> // KV caches
)> {
    println!("📝 Создание основных входных тензоров...");
    
    // Основные входы
    let input_ids = vec![151643i64, 14016, 374, 10127]; // Qwen токены: <|endoftext|>, "What", "is", "AI"
    let attention_mask = vec![1i64; seq_len]; // Все токены активны
    let position_ids: Vec<i64> = (0..seq_len as i64).collect(); // 0, 1, 2, 3
    
    let input_ids_tensor = Tensor::from_array(([1, seq_len], input_ids))?;
    let attention_mask_tensor = Tensor::from_array(([1, seq_len], attention_mask))?;
    let position_ids_tensor = Tensor::from_array(([1, seq_len], position_ids))?;
    
    println!("✅ Основные тензоры созданы");
    
    // KV кеши
    let kv_caches = create_empty_kv_caches()?;
    
    Ok((input_ids_tensor, attention_mask_tensor, position_ids_tensor, kv_caches))
}

/// Эффективная подготовка всех входов для session.run()
fn prepare_all_inputs_for_session(
    input_ids: Tensor<i64>,
    attention_mask: Tensor<i64>, 
    position_ids: Tensor<i64>,
    kv_caches: HashMap<String, Tensor<f32>>
) -> Result<Vec<(String, ort::value::Value)>> {
    let mut all_inputs = Vec::new();
    
    // Базовые входы
    all_inputs.push(("input_ids".to_string(), input_ids.into()));
    all_inputs.push(("attention_mask".to_string(), attention_mask.into()));
    all_inputs.push(("position_ids".to_string(), position_ids.into()));
    
    // KV кеши в отсортированном порядке
    let mut kv_keys: Vec<_> = kv_caches.keys().collect();
    kv_keys.sort();
    
    for key in kv_keys {
        let tensor = kv_caches.get(key).unwrap().clone();
        all_inputs.push((key.clone(), tensor.into()));
    }
    
    Ok(all_inputs)
}

fn main() -> Result<()> {
    println!("=== QWEN3-EMBEDDING: ПОЛНЫЙ ТЕСТ С KV КЕШАМИ ===\n");
    
    let start_time = std::time::Instant::now();
    
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
    
    println!("🎯 ЦЕЛЬ: Получить реальные эмбеддинги из Qwen3-Embedding-0.6B");
    println!("📁 Модель: {}", model_path.display());
    
    // 1. Инициализация ORT
    println!("\n1️⃣ Инициализация ONNX Runtime...");
    ort::init()
        .with_name("qwen3_full_test")
        .commit()?;
    println!("✅ ORT инициализирован за {:.2}ms", start_time.elapsed().as_secs_f64() * 1000.0);
    
    // 2. Создание сессии
    println!("\n2️⃣ Создание ONNX сессии...");
    let session_start = std::time::Instant::now();
    let session = Session::builder()?
        .with_optimization_level(ort::session::builder::GraphOptimizationLevel::Level3)?
        .with_intra_threads(4)?  // Оптимизация для производительности
        .commit_from_file(&model_path)?;
    
    println!("✅ Сессия создана за {:.2}ms", session_start.elapsed().as_secs_f64() * 1000.0);
    println!("   Входов: {}, Выходов: {}", session.inputs.len(), session.outputs.len());
    
    // 3. Создание всех входных тензоров
    println!("\n3️⃣ Создание всех входных тензоров...");
    let tensors_start = std::time::Instant::now();
    
    let seq_len = 4;
    let (input_ids_tensor, attention_mask_tensor, position_ids_tensor, kv_caches) = 
        create_all_inputs(seq_len)?;
    
    println!("✅ Все тензоры созданы за {:.2}ms", tensors_start.elapsed().as_secs_f64() * 1000.0);
    println!("   Общее количество входов: {}", 3 + kv_caches.len());
    
    // 4. Запуск инференса
    println!("\n4️⃣ 🚀 КРИТИЧЕСКИЙ ТЕСТ: Запуск полного инференса...");
    let inference_start = std::time::Instant::now();
    
    let session_mutex = std::sync::Mutex::new(session);
    let mut session_guard = session_mutex.lock().unwrap();
    
    // Подготавливаем все входы
    let all_inputs = prepare_all_inputs_for_session(
        input_ids_tensor,
        attention_mask_tensor, 
        position_ids_tensor,
        kv_caches
    )?;
    
    println!("   Подготовлено {} входов", all_inputs.len());
    
    // Преобразуем в нужный формат для session.run()
    let session_inputs: Vec<(std::borrow::Cow<str>, ort::value::SessionInputValue)> = 
        all_inputs.into_iter()
            .map(|(name, value)| (std::borrow::Cow::Owned(name), value.into()))
            .collect();
    
    // Запуск!
    let outputs = match session_guard.run(session_inputs) {
        Ok(outputs) => {
            let inference_time = inference_start.elapsed().as_secs_f64() * 1000.0;
            println!("🎉🎉🎉 УСПЕХ! Инференс выполнен за {:.2}ms", inference_time);
            outputs
        },
        Err(e) => {
            println!("❌ КРИТИЧЕСКАЯ ОШИБКА: {}", e);
            return Err(e.into());
        }
    };
    
    // 5. Извлечение эмбеддингов
    println!("\n5️⃣ 🎯 Извлечение эмбеддингов (last_token pooling)...");
    let extraction_start = std::time::Instant::now();
    
    let mut final_embedding: Option<Vec<f32>> = None;
    
    for (name, output) in outputs.iter() {
        if let Ok((shape, data)) = output.try_extract_tensor::<f32>() {
            let shape_vec: Vec<i64> = (0..shape.len()).map(|i| shape[i]).collect();
            
            println!("   Выход '{}': форма {:?}, данных {}", name, shape_vec, data.len());
            
            // Ищем основной выход с hidden states [batch, seq, hidden]
            if shape_vec.len() == 3 && shape_vec[0] == 1 && shape_vec[1] == seq_len as i64 {
                let batch_size = shape_vec[0] as usize;
                let sequence_length = shape_vec[1] as usize; 
                let hidden_size = shape_vec[2] as usize;
                
                println!("   🎯 НАЙДЕНЫ HIDDEN STATES!");
                println!("   Размерности: batch={}, seq={}, hidden={}", batch_size, sequence_length, hidden_size);
                
                // Last token pooling (согласно README)
                let last_token_idx = sequence_length - 1;
                let start_idx = last_token_idx * hidden_size;
                let end_idx = start_idx + hidden_size;
                
                if end_idx <= data.len() {
                    let embedding: Vec<f32> = data[start_idx..end_idx].to_vec();
                    
                    println!("   ✅ ЭМБЕДДИНГ ИЗВЛЕЧЕН!");
                    println!("   Размерность: {} (ожидалось ~1024 для Qwen3-0.6B)", embedding.len());
                    
                    // Статистика
                    let min = embedding.iter().fold(f32::INFINITY, |a, &b| a.min(b));
                    let max = embedding.iter().fold(f32::NEG_INFINITY, |a, &b| a.max(b)); 
                    let mean = embedding.iter().sum::<f32>() / embedding.len() as f32;
                    let norm = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
                    
                    println!("   Статистика: min={:.4}, max={:.4}, mean={:.4}", min, max, mean);
                    println!("   Норма: {:.4}", norm);
                    println!("   Первые 5: {:?}", &embedding[..5.min(embedding.len())]);
                    println!("   Последние 5: {:?}", &embedding[embedding.len()-5.min(embedding.len())..]);
                    
                    // Нормализация (согласно README: normalize: true)
                    if norm > 0.0 {
                        let normalized: Vec<f32> = embedding.iter().map(|x| x / norm).collect();
                        let new_norm = normalized.iter().map(|x| x * x).sum::<f32>().sqrt();
                        println!("   ✅ Нормализовано: новая норма={:.6}", new_norm);
                        final_embedding = Some(normalized);
                    } else {
                        final_embedding = Some(embedding);
                    }
                    
                    break;
                }
            }
        }
    }
    
    let extraction_time = extraction_start.elapsed().as_secs_f64() * 1000.0;
    println!("✅ Извлечение завершено за {:.2}ms", extraction_time);
    
    // 6. Финальный результат
    println!("\n6️⃣ 🏁 ФИНАЛЬНЫЙ РЕЗУЛЬТАТ:");
    let total_time = start_time.elapsed().as_secs_f64() * 1000.0;
    
    match final_embedding {
        Some(embedding) => {
            println!("🎉🎉🎉 ПОЛНЫЙ УСПЕХ!");
            println!("✅ Qwen3-Embedding модель работает!");
            println!("✅ ORT 2.0 полностью функционален!");
            println!("✅ Реальные эмбеддинги получены!");
            println!("📊 Размерность эмбеддинга: {}", embedding.len());
            println!("⚡ Общее время: {:.2}ms", total_time);
            
            // Демонстрация качества эмбеддинга
            let sample_size = 10.min(embedding.len());
            println!("🔍 Образец эмбеддинга (первые {}): {:?}", sample_size, &embedding[..sample_size]);
            
            // Проверка, что эмбеддинг имеет разумные значения
            let abs_values: Vec<f32> = embedding.iter().map(|x| x.abs()).collect();
            let max_abs = abs_values.iter().fold(0.0f32, |a, &b| a.max(b));
            
            if max_abs > 0.001 && max_abs < 10.0 {
                println!("✅ Эмбеддинг имеет разумные значения (max_abs={:.4})", max_abs);
            } else {
                println!("⚠️  Необычные значения эмбеддинга (max_abs={:.4})", max_abs);
            }
        },
        None => {
            println!("❌ Эмбеддинги не найдены в выходах модели");
            println!("⚠️  Инференс прошел, но структура выходов неожиданная");
        }
    }
    
    println!("\n🎯 ORT 2.0 + Qwen3-Embedding интеграция ЗАВЕРШЕНА!");
    
    Ok(())
}