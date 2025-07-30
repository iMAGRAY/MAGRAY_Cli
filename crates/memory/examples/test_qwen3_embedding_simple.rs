use anyhow::Result;
use ort::{session::Session, value::Tensor, inputs};
use std::path::PathBuf;

fn main() -> Result<()> {
    println!("=== QWEN3-EMBEDDING: ТЕСТ С ПУСТЫМИ KV КЕШАМИ ===\n");
    
    // Установка пути к DLL
    let dll_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent().unwrap()
        .parent().unwrap()
        .join("scripts")
        .join("onnxruntime")
        .join("lib")
        .join("onnxruntime.dll");
    std::env::set_var("ORT_DYLIB_PATH", dll_path.to_str().unwrap());
    
    println!("💡 ГИПОТЕЗА: Qwen3-Embedding + пустые KV кеши = обычная embedding модель");
    println!("💡 Кеши нужны только для ONNX совместимости, не для functionality");
    
    // Путь к модели
    let model_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("models")
        .join("Qwen3-Embedding-0.6B-ONNX")
        .join("model.onnx");
    
    println!("📁 Модель: {}", model_path.display());
    
    // Инициализация ORT
    println!("\n1. Инициализация ONNX Runtime...");
    ort::init()
        .with_name("qwen3_embedding_simple")
        .commit()?;
    println!("✅ ORT инициализирован");
    
    // Создание сессии
    println!("\n2. Создание ONNX сессии...");
    let mut session = Session::builder()?
        .with_optimization_level(ort::session::builder::GraphOptimizationLevel::Level3)?
        .with_intra_threads(4)?
        .commit_from_file(&model_path)?;
    
    println!("✅ Сессия создана");
    println!("   Входов: {}", session.inputs.len());
    println!("   Выходов: {}", session.outputs.len());
    
    // Подготовка данных
    println!("\n3. Подготовка входных данных...");
    
    let seq_len = 4;
    let input_ids = vec![151643i64, 3555, 374, 15592]; // "What is"
    let attention_mask = vec![1i64; seq_len];
    let position_ids: Vec<i64> = (0..seq_len as i64).collect();
    
    println!("✅ Базовые тензоры готовы: input_ids, attention_mask, position_ids");
    
    // КРИТИЧЕСКИЙ МОМЕНТ: Создаем пустые KV кеши для всех 28 слоев
    println!("\n4. 🔧 Создание пустых KV кешей для 28 слоев...");
    
    let batch_size = 1;
    let num_heads = 8;
    let head_dim = 128;
    let past_seq_len = 0; // ПУСТЫЕ кеши!
    
    // Базовые тензоры
    let input_ids_tensor = Tensor::from_array(([batch_size, seq_len], input_ids))?;
    let attention_mask_tensor = Tensor::from_array(([batch_size, seq_len], attention_mask))?;
    let position_ids_tensor = Tensor::from_array(([batch_size, seq_len], position_ids))?;
    
    println!("   📦 Создаем пустые KV кеши для всех 28 слоев...");
    
    // Создаем пустые KV кеши для каждого слоя
    let mut kv_inputs = Vec::new();
    for layer in 0..28 {
        // Пустые тензоры с формой [1, 8, 0, 128] - правильные размеры для пустых кешей
        let empty_key = Tensor::from_array(([batch_size, num_heads, past_seq_len, head_dim], Vec::<f32>::new()))?;
        let empty_value = Tensor::from_array(([batch_size, num_heads, past_seq_len, head_dim], Vec::<f32>::new()))?;
        
        kv_inputs.push((format!("past_key_values.{}.key", layer), empty_key));
        kv_inputs.push((format!("past_key_values.{}.value", layer), empty_value));
        
        if layer == 0 {
            println!("   Layer {}: Пустые key/value [1,8,{},128]", layer, past_seq_len);
        } else if layer == 1 {
            println!("   ... (создано для всех 28 слоев) ...");
        }
    }
    
    println!("   ✅ Создано {} KV кешей для 28 слоев", kv_inputs.len());
    
    // ЭКСПЕРИМЕНТ: Запуск с пустыми кешами используя inputs! макрос
    println!("\n5. 🚀 КРИТИЧЕСКИЙ ТЕСТ: Qwen3-Embedding с пустыми KV кешами...");
    
    // Создаем inputs динамически - это сложно с макросом, попробуем по-другому
    println!("   💡 Попытка с макросом inputs! ограничена, используем прямой API...");
    
    // Создаем вручную все входы
    let mut all_kv_pairs = vec![
        ("input_ids", input_ids_tensor.into()),
        ("attention_mask", attention_mask_tensor.into()),
        ("position_ids", position_ids_tensor.into()),
    ];
    
    // Добавляем KV кеши
    for (name, tensor) in kv_inputs {
        all_kv_pairs.push((name.as_str(), tensor.into()));
    }
    
    println!("   📝 Всего входов: {}", all_kv_pairs.len());
    
    let result = session.run(all_kv_pairs);
    
    match result {
        Ok(outputs) => {
            println!("🎉🎉🎉 НЕВЕРОЯТНО! QWEN3-EMBEDDING РАБОТАЕТ!");
            println!("   Получено {} выходов", outputs.len());
            
            // Ищем главный выход - hidden states
            for (name, output) in outputs.iter() {
                if name == "last_hidden_state" {
                    if let Ok((shape, data)) = output.try_extract_tensor::<f32>() {
                        let shape_vec: Vec<i64> = (0..shape.len()).map(|i| shape[i]).collect();
                        println!("   🎯 НАЙДЕН HIDDEN STATE: форма {:?}, данных {}", shape_vec, data.len());
                        
                        // Должно быть [1, seq_len, 1024] для Qwen3
                        if shape_vec.len() == 3 && shape_vec[0] == 1 && shape_vec[1] == seq_len as i64 {
                            let hidden_size = shape_vec[2] as usize;
                            
                            println!("   ✅ ПРАВИЛЬНАЯ ФОРМА: [1, {}, {}]", seq_len, hidden_size);
                            
                            // Last token pooling как обычно
                            let last_token_start = (seq_len - 1) * hidden_size;
                            let last_token_end = last_token_start + hidden_size;
                            
                            if last_token_end <= data.len() {
                                let embedding: Vec<f32> = data[last_token_start..last_token_end].to_vec();
                                let norm = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
                                
                                println!("   🏆 QWEN3 ЭМБЕДДИНГ ИЗВЛЕЧЕН!");
                                println!("   Размерность: {}", embedding.len());
                                println!("   Норма: {:.4}", norm);
                                println!("   Образец: {:?}", &embedding[..3.min(embedding.len())]);
                                
                                if norm > 0.0 {
                                    println!("\n🎊🎊🎊 ТРИУМФАЛЬНОЕ ОТКРЫТИЕ!");
                                    println!("✅ Qwen3-Embedding РАБОТАЕТ как обычная embedding модель!");
                                    println!("✅ KV кеши нужны только для ONNX совместимости!");
                                    println!("✅ Можно использовать с пустыми кешами = 0 overhead!");
                                    println!("✅ Архитектура CausalLM, но задача EMBEDDING!");
                                }
                            }
                        }
                    }
                }
            }
            
        },
        Err(e) => {
            println!("❌ Ошибка: {}", e);
            
            if format!("{}", e).to_lowercase().contains("invalid dimension") {
                println!("💡 Возможно, нужно правильно создать пустые тензоры");
            } else if format!("{}", e).to_lowercase().contains("missing input") {
                println!("💡 Возможно, пропущен какой-то обязательный вход");
            }
        }
    }
    
    println!("\n📊 РЕЗУЛЬТАТ ЭКСПЕРИМЕНТА:");
    println!("- Qwen3-Embedding + пустые KV кеши = ?");
    println!("- Если работает: это обычная embedding модель с ONNX overhead");
    println!("- Если не работает: действительно сложная архитектура");
    
    Ok(())
}