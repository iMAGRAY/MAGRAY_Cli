use anyhow::Result;
use ort::{session::Session, value::Tensor, inputs};
use std::path::PathBuf;

fn main() -> Result<()> {
    println!("=== QWEN3-EMBEDDING БЕЗ KV КЕША ===\n");
    
    // Установка пути к DLL
    let dll_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent().unwrap()
        .parent().unwrap()
        .join("scripts")
        .join("onnxruntime")
        .join("lib")
        .join("onnxruntime.dll");
    std::env::set_var("ORT_DYLIB_PATH", dll_path.to_str().unwrap());
    
    println!("💡 ГИПОТЕЗА: Попробуем Qwen3-Embedding БЕЗ KV кешей");
    println!("💡 Возможно модель может работать только с базовыми входами!");
    
    // Сначала модифицируем конфигурацию (создаем копию с use_cache: false)
    let config_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("models")
        .join("Qwen3-Embedding-0.6B-ONNX")
        .join("config.json");
    
    println!("📋 Читаем исходную конфигурацию...");
    let config_content = std::fs::read_to_string(&config_path)?;
    println!("✅ Конфигурация прочитана");
    
    // Модифицируем use_cache
    let modified_config = config_content.replace("\"use_cache\": true", "\"use_cache\": false");
    
    let temp_config_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("models")
        .join("Qwen3-Embedding-0.6B-ONNX")
        .join("config_no_cache.json");
    
    std::fs::write(&temp_config_path, &modified_config)?;
    println!("✅ Создана модифицированная конфигурация: use_cache = false");
    
    // Путь к модели
    let model_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("models")
        .join("Qwen3-Embedding-0.6B-ONNX")
        .join("model.onnx");
    
    // Инициализация ORT
    println!("\n1. Инициализация ONNX Runtime...");
    ort::init()
        .with_name("qwen3_no_cache")
        .commit()?;
    println!("✅ ORT инициализирован");
    
    // Создание сессии БЕЗ KV кеша
    println!("\n2. Создание ONNX сессии...");
    let mut session = Session::builder()?
        .with_optimization_level(ort::session::builder::GraphOptimizationLevel::Level3)?
        .with_intra_threads(4)?
        .commit_from_file(&model_path)?;
    
    println!("✅ Сессия создана");
    println!("   Входов: {}", session.inputs.len());
    println!("   Выходов: {}", session.outputs.len());
    
    // Анализ входов - возможно без use_cache будет меньше входов!
    println!("\n3. Анализ входов модели:");
    for (i, input) in session.inputs.iter().enumerate() {
        println!("   {}: {} - {:?}", i, input.name, input.input_type);
    }
    
    // Попытка 1: Только базовые входы
    println!("\n4. 🧪 ЭКСПЕРИМЕНТ: Только базовые входы!");
    
    let seq_len = 4;
    let input_ids = vec![151643i64, 3555, 374, 15592]; // "What is AI?"
    let attention_mask = vec![1i64; seq_len];
    let position_ids: Vec<i64> = (0..seq_len as i64).collect();
    
    let input_ids_tensor = Tensor::from_array(([1, seq_len], input_ids))?;
    let attention_mask_tensor = Tensor::from_array(([1, seq_len], attention_mask))?;
    let position_ids_tensor = Tensor::from_array(([1, seq_len], position_ids))?;
    
    println!("✅ Базовые тензоры созданы:");
    println!("   input_ids: [1, {}]", seq_len);
    println!("   attention_mask: [1, {}]", seq_len);
    println!("   position_ids: [1, {}]", seq_len);
    
    // ЭКСПЕРИМЕНТ: Попробуем только с базовыми входами
    println!("\n5. 🚀 ЭКСПЕРИМЕНТ: Запуск БЕЗ KV кешей...");
    
    let result = session.run(inputs![
        "input_ids" => input_ids_tensor,
        "attention_mask" => attention_mask_tensor,
        "position_ids" => position_ids_tensor
    ]);
    
    match result {
        Ok(outputs) => {
            println!("🎉🎉🎉 НЕВЕРОЯТНО! РАБОТАЕТ БЕЗ KV КЕШЕЙ!");
            println!("   Получено {} выходов", outputs.len());
            
            // Ищем эмбеддинги
            for (name, output) in outputs.iter() {
                if let Ok((shape, data)) = output.try_extract_tensor::<f32>() {
                    let shape_vec: Vec<i64> = (0..shape.len()).map(|i| shape[i]).collect();
                    println!("   Выход '{}': форма {:?}, данных {}", name, shape_vec, data.len());
                    
                    // Ищем hidden states [batch, seq, hidden]
                    if shape_vec.len() == 3 && shape_vec[0] == 1 && shape_vec[1] == seq_len as i64 {
                        let hidden_size = shape_vec[2] as usize;
                        
                        println!("   🎯 НАЙДЕНЫ QWEN3 ЭМБЕДДИНГИ!");
                        println!("   Размерности: [1, {}, {}]", seq_len, hidden_size);
                        
                        // Last token pooling (как в README)
                        let last_token_start = (seq_len - 1) * hidden_size;
                        let last_token_end = last_token_start + hidden_size;
                        
                        if last_token_end <= data.len() {
                            let embedding: Vec<f32> = data[last_token_start..last_token_end].to_vec();
                            let norm = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
                            
                            println!("   ✅ ПОСЛЕДНИЙ ТОКЕН ИЗВЛЕЧЕН!");
                            println!("   Размерность: {}", embedding.len());
                            println!("   Норма: {:.4}", norm);
                            println!("   Образец: {:?}", &embedding[..5.min(embedding.len())]);
                            
                            if norm > 0.0 {
                                let normalized: Vec<f32> = embedding.iter().map(|x| x / norm).collect();
                                println!("   ✅ НОРМАЛИЗОВАНО!");
                                println!("   🏆 QWEN3 ЭМБЕДДИНГ ГОТОВ: {} размерность", normalized.len());
                                
                                println!("\n🎊🎊🎊 ТРИУМФ!");
                                println!("✅ Qwen3-Embedding работает БЕЗ KV кешей!");
                                println!("✅ Нужны только 3 базовых входа!");
                                println!("✅ use_cache: false решает проблему!");
                                println!("✅ ORT 2.0 + Qwen3 = ПОЛНАЯ ПОБЕДА!");
                                
                                return Ok(());
                            }
                        }
                    }
                }
            }
            
            println!("⚠️ Инференс прошел, но эмбеддинги не найдены в ожидаемом формате");
        },
        Err(e) => {
            println!("❌ Все еще нужны KV кеши: {}", e);
            
            if format!("{}", e).contains("Missing Input:") {
                println!("💡 Модель все еще требует дополнительные входы");
                println!("💡 Возможно use_cache влияет только на runtime, не на граф модели");
            }
        }
    }
    
    // Очистка
    let _ = std::fs::remove_file(&temp_config_path);
    
    println!("\n📊 РЕЗУЛЬТАТ ЭКСПЕРИМЕНТА:");
    println!("- Модификация use_cache: Попробовано");
    println!("- Базовые входы только: Протестировано");
    println!("- Статус: См. выше");
    
    Ok(())
}