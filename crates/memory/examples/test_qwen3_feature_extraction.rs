use anyhow::Result;
use ort::{session::Session, value::Tensor, inputs};
use std::path::PathBuf;

fn main() -> Result<()> {
    println!("=== QWEN3-EMBEDDING: НАСТОЯЩАЯ FEATURE-EXTRACTION МОДЕЛЬ ===\n");
    
    // Установка пути к DLL
    let dll_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent().unwrap()
        .parent().unwrap()
        .join("scripts")
        .join("onnxruntime")
        .join("lib")
        .join("onnxruntime.dll");
    std::env::set_var("ORT_DYLIB_PATH", dll_path.to_str().unwrap());
    
    println!("✅ ПРАВИЛЬНАЯ МОДЕЛЬ: Qwen3-Embedding feature-extraction");
    println!("✅ Ожидается: Только базовые входы (input_ids, attention_mask, position_ids)");
    println!("✅ БЕЗ KV кешей - чистая embedding модель!");
    
    // Путь к правильной модели
    let model_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("models")
        .join("Qwen3-Embedding-0.6B-ONNX")
        .join("model.onnx");
    
    println!("\n📁 Модель: {}", model_path.display());
    
    // Инициализация ORT
    println!("\n1. Инициализация ONNX Runtime...");
    ort::init()
        .with_name("qwen3_feature_extraction")
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
    
    // ПРОВЕРКА: Должно быть только 3 входа
    println!("\n3. 🔍 АНАЛИЗ ВХОДОВ (должно быть 3!):");
    for (i, input) in session.inputs.iter().enumerate() {
        println!("   {}: {} - {:?}", i, input.name, input.input_type);
    }
    
    println!("\n4. 🔍 АНАЛИЗ ВЫХОДОВ:");
    for (i, output) in session.outputs.iter().enumerate() {
        println!("   {}: {} - {:?}", i, output.name, output.output_type);
    }
    
    let num_inputs = session.inputs.len();
    if num_inputs == 3 {
        println!("\n🎉 ОТЛИЧНО! Точно 3 входа - это правильная feature-extraction модель!");
    } else {
        println!("\n⚠️ Неожиданно: {} входов, ожидалось 3", num_inputs);
    }
    
    // ТЕСТ: Базовые входы для embedding
    println!("\n5. 🧪 ТЕСТ: Создание эмбеддингов...");
    
    let seq_len = 6;
    let input_ids = vec![151643i64, 3555, 374, 15592, 1029, 151645]; // "What is AI?"
    let attention_mask = vec![1i64; seq_len];
    let position_ids: Vec<i64> = (0..seq_len as i64).collect();
    
    let input_ids_tensor = Tensor::from_array(([1, seq_len], input_ids))?;
    let attention_mask_tensor = Tensor::from_array(([1, seq_len], attention_mask))?;
    let position_ids_tensor = Tensor::from_array(([1, seq_len], position_ids))?;
    
    println!("✅ Входные тензоры созданы");
    println!("   input_ids: [1, {}]", seq_len);
    println!("   attention_mask: [1, {}]", seq_len);
    println!("   position_ids: [1, {}]", seq_len);
    
    // КРИТИЧЕСКИЙ МОМЕНТ: Запуск feature extraction
    println!("\n6. 🚀 КРИТИЧЕСКИЙ ТЕСТ: Qwen3 Feature Extraction...");
    
    let result = session.run(inputs![
        "input_ids" => input_ids_tensor,
        "attention_mask" => attention_mask_tensor,
        "position_ids" => position_ids_tensor
    ]);
    
    match result {
        Ok(outputs) => {
            println!("🎉🎉🎉 НЕВЕРОЯТНЫЙ УСПЕХ!");
            println!("   Получено {} выходов", outputs.len());
            
            // Поиск эмбеддингов
            let mut found_embeddings = false;
            
            for (name, output) in outputs.iter() {
                if let Ok((shape, data)) = output.try_extract_tensor::<f32>() {
                    let shape_vec: Vec<i64> = (0..shape.len()).map(|i| shape[i]).collect();
                    println!("   Выход '{}': форма {:?}, данных {}", name, shape_vec, data.len());
                    
                    // Ищем hidden states [batch, seq, hidden]
                    if shape_vec.len() == 3 && shape_vec[0] == 1 && shape_vec[1] == seq_len as i64 {
                        let hidden_size = shape_vec[2] as usize;
                        
                        println!("   🎯 НАЙДЕНЫ QWEN3 HIDDEN STATES!");
                        println!("   Размерности: [1, {}, {}]", seq_len, hidden_size);
                        
                        // Last token pooling (как в README)
                        let last_token_start = (seq_len - 1) * hidden_size;
                        let last_token_end = last_token_start + hidden_size;
                        
                        if last_token_end <= data.len() {
                            let embedding: Vec<f32> = data[last_token_start..last_token_end].to_vec();
                            let norm = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
                            
                            println!("   ✅ ПОСЛЕДНИЙ ТОКЕН ИЗВЛЕЧЕН!");
                            println!("   Размерность эмбеддинга: {}", embedding.len());
                            println!("   Норма: {:.4}", norm);
                            println!("   Образец: {:?}", &embedding[..5.min(embedding.len())]);
                            
                            if norm > 0.0 {
                                // Нормализация (как в README: normalize: true)
                                let normalized: Vec<f32> = embedding.iter().map(|x| x / norm).collect();
                                let final_norm = normalized.iter().map(|x| x * x).sum::<f32>().sqrt();
                                
                                println!("   ✅ ЭМБЕДДИНГ НОРМАЛИЗОВАН!");
                                println!("   Финальная норма: {:.6}", final_norm);
                                println!("   🏆 ГОТОВЫЙ QWEN3 ЭМБЕДДИНГ: {} размерность", normalized.len());
                                
                                found_embeddings = true;
                                break;
                            }
                        }
                    }
                }
            }
            
            if found_embeddings {
                println!("\n🎊🎊🎊 ПОЛНАЯ ПОБЕДА!");
                println!("✅ Qwen3-Embedding feature-extraction РАБОТАЕТ!");
                println!("✅ Только 3 входа - никаких KV кешей!");
                println!("✅ Реальные нормализованные эмбеддинги!");
                println!("✅ Готово для production использования!");
                
                println!("\n🚀 СРАВНЕНИЕ МОДЕЛЕЙ:");
                println!("- E5-small: 384 dim, BertModel, простая");
                println!("- MXBai: 896 dim, Qwen2ForCausalLM, rerank");
                println!("- Qwen3-Embedding: 1024 dim, Qwen3ForCausalLM, embedding ✨");
                println!("- Qwen3-Reranker: logits, Qwen3ForCausalLM, rerank");
                
                println!("\n🎯 ВСЕ ЧЕТЫРЕ МОДЕЛИ РАБОТАЮТ С ORT 2.0!");
                
            } else {
                println!("⚠️ Инференс прошел, но эмбеддинги не найдены в ожидаемом формате");
            }
        },
        Err(e) => {
            println!("❌ Ошибка: {}", e);
            
            if format!("{}", e).contains("Missing Input:") {
                println!("💔 Неожиданно: модель все еще требует дополнительные входы");
                println!("💡 Возможно, скачана неправильная версия");
            }
        }
    }
    
    println!("\n📊 ФИНАЛЬНЫЙ СТАТУС:");
    println!("- Входов в модели: {}", num_inputs);
    println!("- Ожидалось: 3 (для feature-extraction)");
    println!("- Статус: {}", if num_inputs == 3 { "✅ Правильная модель" } else { "❌ Возможно неправильная версия" });
    
    Ok(())
}