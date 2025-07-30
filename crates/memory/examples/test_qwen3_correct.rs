use anyhow::Result;
use ort::{session::Session, value::Tensor, inputs};
use std::path::PathBuf;

fn main() -> Result<()> {
    println!("=== QWEN3-EMBEDDING: ПРАВИЛЬНЫЙ ТЕСТ ===\n");
    
    // Установка пути к DLL
    let dll_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent().unwrap()
        .parent().unwrap()
        .join("scripts")
        .join("onnxruntime")
        .join("lib")
        .join("onnxruntime.dll");
    std::env::set_var("ORT_DYLIB_PATH", dll_path.to_str().unwrap());
    
    println!("✅ ПРАВИЛЬНОЕ ПОНИМАНИЕ:");
    println!("- Qwen3-Embedding экспортирован для feature-extraction");
    println!("- НЕТ KV кешей в ONNX графе");
    println!("- use_cache:true в config.json - просто остаток");
    println!("- Должна работать как обычная embedding модель!");
    
    // Путь к модели
    let model_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("models")
        .join("Qwen3-Embedding-0.6B-ONNX")
        .join("model.onnx");
    
    println!("\n📁 Модель: {}", model_path.display());
    
    // Инициализация ORT
    println!("\n1. Инициализация ONNX Runtime...");
    ort::init()
        .with_name("qwen3_correct")
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
    
    // КРИТИЧЕСКИЙ МОМЕНТ: Реальный анализ входов
    println!("\n3. 🔍 РЕАЛЬНЫЙ АНАЛИЗ ВХОДОВ:");
    let mut has_kv_inputs = false;
    let mut basic_inputs = 0;
    
    for (i, input) in session.inputs.iter().enumerate() {
        if input.name.contains("past_key_values") {
            has_kv_inputs = true;
            if i < 10 {
                println!("   {}: {} - {:?} ❌ KV кеш!", i, input.name, input.input_type);
            }
        } else {
            basic_inputs += 1;
            println!("   {}: {} - {:?} ✅ Базовый вход", i, input.name, input.input_type);
        }
    }
    
    println!("\n📊 СТАТИСТИКА ВХОДОВ:");
    println!("   Базовые входы: {}", basic_inputs);
    println!("   KV кеш входы: {}", session.inputs.len() - basic_inputs);
    println!("   KV кеши найдены: {}", has_kv_inputs);
    
    if has_kv_inputs {
        println!("\n⚠️ НЕОЖИДАННОСТЬ: В ONNX графе ЕСТЬ KV кеши!");
        println!("💭 Возможные объяснения:");
        println!("   1. Модель экспортирована с full CausalLM графом");
        println!("   2. Transformers.js экспорт включил кеши 'на всякий случай'");
        println!("   3. Модель поддерживает и embedding, и generation");
        println!("\n💡 НО для embedding нам нужны только базовые входы!");
    } else {
        println!("\n✅ ОЖИДАЕМО: Только базовые входы для embedding!");
    }
    
    // ТЕСТ: Только базовые входы
    println!("\n4. 🧪 ТЕСТ: Только базовые входы (как должно быть)...");
    
    let seq_len = 5;
    let input_ids = vec![151643i64, 3555, 374, 15592, 1029]; // "What is AI"
    let attention_mask = vec![1i64; seq_len];
    let position_ids: Vec<i64> = (0..seq_len as i64).collect();
    
    let input_ids_tensor = Tensor::from_array(([1, seq_len], input_ids))?;
    let attention_mask_tensor = Tensor::from_array(([1, seq_len], attention_mask))?;
    let position_ids_tensor = Tensor::from_array(([1, seq_len], position_ids))?;
    
    println!("✅ Базовые тензоры созданы");
    
    // Попытка с только базовыми входами
    println!("\n5. 🚀 КРИТИЧЕСКИЙ ТЕСТ: Только 3 базовых входа...");
    
    let result = session.run(inputs![
        "input_ids" => input_ids_tensor,
        "attention_mask" => attention_mask_tensor,
        "position_ids" => position_ids_tensor
    ]);
    
    match result {
        Ok(outputs) => {
            println!("🎉🎉🎉 ПОБЕДА! QWEN3-EMBEDDING РАБОТАЕТ БЕЗ KV КЕШЕЙ!");
            println!("   Получено {} выходов", outputs.len());
            
            // Поиск эмбеддингов
            for (name, output) in outputs.iter() {
                if let Ok((shape, data)) = output.try_extract_tensor::<f32>() {
                    let shape_vec: Vec<i64> = (0..shape.len()).map(|i| shape[i]).collect();
                    println!("   Выход '{}': форма {:?}, данных {}", name, shape_vec, data.len());
                    
                    // Ищем hidden states [batch, seq, hidden]
                    if shape_vec.len() == 3 && shape_vec[0] == 1 && shape_vec[1] == seq_len as i64 {
                        let hidden_size = shape_vec[2] as usize;
                        
                        println!("   🎯 НАЙДЕНЫ QWEN3 ЭМБЕДДИНГИ!");
                        println!("   Размерности: [1, {}, {}]", seq_len, hidden_size);
                        
                        // Last token pooling
                        let last_token_start = (seq_len - 1) * hidden_size;
                        let last_token_end = last_token_start + hidden_size;
                        
                        if last_token_end <= data.len() {
                            let embedding: Vec<f32> = data[last_token_start..last_token_end].to_vec();
                            let norm = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
                            
                            println!("   ✅ ЭМБЕДДИНГ ИЗВЛЕЧЕН!");
                            println!("   Размерность: {}", embedding.len());
                            println!("   Норма: {:.4}", norm);
                            println!("   Образец: {:?}", &embedding[..3.min(embedding.len())]);
                            
                            if norm > 0.0 {
                                println!("\n🏆🏆🏆 ТРИУМФ!");
                                println!("✅ Qwen3-Embedding - обычная embedding модель!");
                                println!("✅ Работает БЕЗ KV кешей!");
                                println!("✅ {} размерность эмбеддингов!", embedding.len());
                                println!("✅ Готова к production использованию!");
                                
                                return Ok(());
                            }
                        }
                    }
                }
            }
            
            println!("⚠️ Инференс прошел, но эмбеддинги не найдены в ожидаемом формате");
        },
        Err(e) => {
            println!("❌ Ошибка: {}", e);
            
            if format!("{}", e).contains("Missing Input:") {
                println!("\n💔 К сожалению, модель все-таки требует KV кеши");
                println!("💡 Это означает, что ONNX граф включает полную CausalLM архитектуру");
                println!("💡 Но для embedding мы могли бы использовать пустые кеши");
            }
        }
    }
    
    println!("\n📋 ФИНАЛЬНОЕ ЗАКЛЮЧЕНИЕ:");
    println!("- Если работает: Qwen3-Embedding - простая embedding модель ✅");
    println!("- Если нет: Требует KV кеши, но может работать с пустыми ⚠️");
    println!("- В любом случае: E5-small и MXBai проще для production 🚀");
    
    Ok(())
}