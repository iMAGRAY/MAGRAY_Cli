use anyhow::Result;
use ort::{session::Session, value::Tensor, inputs};
use std::path::PathBuf;

fn main() -> Result<()> {
    println!("=== QWEN3-EMBEDDING: ФИНАЛЬНЫЙ ТЕСТ ===\n");
    
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
    
    // Инициализация
    ort::init().with_name("qwen3_final").commit()?;
    println!("✅ ORT инициализирован");
    
    // Создание сессии
    let session = Session::builder()?
        .with_optimization_level(ort::session::builder::GraphOptimizationLevel::Level3)?
        .with_intra_threads(4)?
        .commit_from_file(&model_path)?;
    
    println!("✅ Сессия создана, входов: {}", session.inputs.len());
    
    // Создание основных входов
    let seq_len = 4;
    let input_ids = vec![151643i64, 14016, 374, 10127]; // Qwen токены
    let attention_mask = vec![1i64; seq_len];
    let position_ids: Vec<i64> = (0..seq_len as i64).collect();
    
    let input_ids_tensor = Tensor::from_array(([1, seq_len], input_ids))?;
    let attention_mask_tensor = Tensor::from_array(([1, seq_len], attention_mask))?;
    let position_ids_tensor = Tensor::from_array(([1, seq_len], position_ids))?;
    
    println!("✅ Основные тензоры созданы");
    
    // Создание всех 56 KV кеш тензоров максимально эффективно
    println!("🔧 Создание 56 KV кеш тензоров...");
    
    let empty_shape = [1, 8, 0, 128]; // [batch, heads, past_seq_len, head_dim]
    let empty_data: Vec<f32> = Vec::new(); // Пустой для past_seq_len=0
    
    // Создаем все KV тензоры одним циклом
    let mut kv_tensors = Vec::new();
    for layer in 0..28 {
        let key_tensor = Tensor::from_array((empty_shape, empty_data.clone()))?;
        let value_tensor = Tensor::from_array((empty_shape, empty_data.clone()))?;
        kv_tensors.push((format!("past_key_values.{}.key", layer), key_tensor));
        kv_tensors.push((format!("past_key_values.{}.value", layer), value_tensor));
    }
    
    println!("✅ Создано {} KV кеш тензоров", kv_tensors.len());
    
    // Запуск инференса
    println!("\n🚀 КРИТИЧЕСКИЙ ТЕСТ: Полный инференс...");
    let inference_start = std::time::Instant::now();
    
    let session = std::sync::Mutex::new(session);
    let mut session_guard = session.lock().unwrap();
    
    // Пытаемся запустить хотя бы с базовыми входами + первые несколько KV
    // Если не сработает, то модель действительно требует ВСЕ входы
    
    println!("Попытка 1: Только базовые входы...");
    let result_basic = session_guard.run(inputs![
        "input_ids" => input_ids_tensor.clone(),
        "attention_mask" => attention_mask_tensor.clone(),
        "position_ids" => position_ids_tensor.clone()
    ]);
    
    match result_basic {
        Ok(outputs) => {
            println!("🎉 ЧУДО! Работает с базовыми входами!");
        },
        Err(e) => {
            println!("❌ Ожидаемо: нужны KV кеши: {}", e);
            
            // ВАЖНО: Здесь нам нужно создать полный session.run() вызов
            // с ВСЕМИ 59 входами. К сожалению, inputs! макрос не может
            // динамически принимать переменное количество входов.
            
            // Поэтому создадим входы вручную через низкоуровневый API
            println!("\nПопытка 2: Создание полного набора входов вручную...");
            
            // Создаем Vec для всех входов
            let mut all_inputs: Vec<(&str, ort::value::Value)> = Vec::new();
            
            // Базовые входы
            all_inputs.push(("input_ids", input_ids_tensor.into()));
            all_inputs.push(("attention_mask", attention_mask_tensor.into()));
            all_inputs.push(("position_ids", position_ids_tensor.into()));
            
            // Все KV кеши
            for (name, tensor) in kv_tensors {
                all_inputs.push((name.as_str(), tensor.into()));
            }
            
            println!("   Подготовлено {} входов", all_inputs.len());
            
            // Преобразуем в формат для run()
            let session_inputs: Vec<(std::borrow::Cow<str>, ort::session::SessionInputValue)> = 
                all_inputs.into_iter()
                    .map(|(name, value)| (std::borrow::Cow::Borrowed(name), value.into()))
                    .collect();
            
            // Финальный запуск с ВСЕМИ входами
            let outputs = match session_guard.run(session_inputs) {
                Ok(outputs) => {
                    let inference_time = inference_start.elapsed().as_secs_f64() * 1000.0;
                    println!("🎉🎉🎉 ПОЛНЫЙ УСПЕХ! Инференс за {:.2}ms", inference_time);
                    outputs
                },
                Err(e) => {
                    println!("❌ ФИНАЛЬНАЯ ОШИБКА: {}", e);
                    return Err(e.into());
                }
            };
            
            // Извлечение эмбеддингов
            println!("\n🎯 Извлечение эмбеддингов (last_token pooling)...");
            
            let mut found_embedding = false;
            for (name, output) in outputs.iter() {
                if let Ok((shape, data)) = output.try_extract_tensor::<f32>() {
                    let shape_vec: Vec<i64> = (0..shape.len()).map(|i| shape[i]).collect();
                    
                    println!("   Выход '{}': форма {:?}, данных {}", name, shape_vec, data.len());
                    
                    // Ищем hidden states [batch, seq, hidden]
                    if shape_vec.len() == 3 && shape_vec[0] == 1 && shape_vec[1] == seq_len as i64 {
                        let sequence_length = shape_vec[1] as usize;
                        let hidden_size = shape_vec[2] as usize;
                        
                        println!("   🎯 НАЙДЕНЫ HIDDEN STATES!");
                        println!("   Размерности: [1, {}, {}]", sequence_length, hidden_size);
                        
                        // Last token pooling
                        let last_token_idx = sequence_length - 1;
                        let start_idx = last_token_idx * hidden_size;
                        let end_idx = start_idx + hidden_size;
                        
                        if end_idx <= data.len() {
                            let embedding: Vec<f32> = data[start_idx..end_idx].to_vec();
                            
                            println!("   ✅ ЭМБЕДДИНГ ИЗВЛЕЧЕН!");
                            println!("   Размерность: {}", embedding.len());
                            
                            // Статистика
                            let min = embedding.iter().fold(f32::INFINITY, |a, &b| a.min(b));
                            let max = embedding.iter().fold(f32::NEG_INFINITY, |a, &b| a.max(b));
                            let mean = embedding.iter().sum::<f32>() / embedding.len() as f32;
                            let norm = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
                            
                            println!("   Статистика: min={:.4}, max={:.4}, mean={:.4}, norm={:.4}", min, max, mean, norm);
                            println!("   Образец: {:?}", &embedding[..5.min(embedding.len())]);
                            
                            // Нормализация
                            if norm > 0.0 {
                                let normalized: Vec<f32> = embedding.iter().map(|x| x / norm).collect();
                                let new_norm = normalized.iter().map(|x| x * x).sum::<f32>().sqrt();
                                println!("   ✅ Нормализовано: норма={:.6}", new_norm);
                            }
                            
                            found_embedding = true;
                            break;
                        }
                    }
                }
            }
            
            if found_embedding {
                println!("\n🎉🎉🎉 ФИНАЛЬНЫЙ УСПЕХ!");
                println!("✅ Qwen3-Embedding полностью работает!");
                println!("✅ ORT 2.0 полностью функционален!");
                println!("✅ Реальные эмбеддинги получены!");
                println!("⚡ Общее время: {:.2}ms", start_time.elapsed().as_secs_f64() * 1000.0);
            } else {
                println!("⚠️  Инференс прошел, но эмбеддинги не найдены");
            }
        }
    }
    
    Ok(())
}