use anyhow::Result;
use ort::{session::Session, value::Tensor, inputs};
use std::path::PathBuf;

fn main() -> Result<()> {
    println!("=== QWEN3 МОДЕЛИ: EMBEDDING vs RERANKER ===\n");
    
    // Установка пути к DLL
    let dll_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent().unwrap()
        .parent().unwrap()
        .join("scripts")
        .join("onnxruntime")
        .join("lib")
        .join("onnxruntime.dll");
    std::env::set_var("ORT_DYLIB_PATH", dll_path.to_str().unwrap());
    
    println!("🔍 ГИПОТЕЗА: Qwen3-Reranker (use_cache=false) должен работать с 3 входами!");
    println!("🔍 А Qwen3-Embedding (use_cache=true) требует KV кеши");
    
    // Инициализация ORT
    println!("\n1. Инициализация ONNX Runtime...");
    ort::init()
        .with_name("qwen3_comparison")
        .commit()?;
    println!("✅ ORT инициализирован");
    
    println!("\n{}", "=".repeat(60));
    println!("🧪 ТЕСТ 1: QWEN3-RERANKER (use_cache=false)");
    println!("{}", "=".repeat(60));
    
    let reranker_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("models")
        .join("Qwen3-Reranker-0.6B-ONNX")
        .join("model.onnx");
    
    println!("📁 Модель: {}", reranker_path.display());
    println!("✅ Существует: {}", reranker_path.exists());
    
    if reranker_path.exists() {
        match test_qwen3_model(&reranker_path, "Qwen3-Reranker") {
            Ok(success) => {
                if success {
                    println!("🎉 QWEN3-RERANKER РАБОТАЕТ!");
                } else {
                    println!("⚠️ Qwen3-Reranker: инференс прошел, но без эмбеддингов");
                }
            },
            Err(e) => {
                println!("❌ Qwen3-Reranker: {}", e);
            }
        }
    }
    
    println!("\n{}", "=".repeat(60));
    println!("🧪 ТЕСТ 2: QWEN3-EMBEDDING (use_cache=true)");
    println!("{}", "=".repeat(60));
    
    let embedding_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("models")
        .join("Qwen3-Embedding-0.6B-ONNX")
        .join("model.onnx");
    
    println!("📁 Модель: {}", embedding_path.display());
    println!("✅ Существует: {}", embedding_path.exists());
    
    if embedding_path.exists() {
        match test_qwen3_model(&embedding_path, "Qwen3-Embedding") {
            Ok(success) => {
                if success {
                    println!("🎉 QWEN3-EMBEDDING ТОЖЕ РАБОТАЕТ!");
                } else {
                    println!("⚠️ Qwen3-Embedding: инференс прошел, но без эмбеддингов");
                }
            },
            Err(e) => {
                println!("❌ Qwen3-Embedding: {}", e);
            }
        }
    }
    
    println!("\n{}", "=".repeat(60));
    println!("🏆 ФИНАЛЬНЫЕ ВЫВОДЫ");
    println!("{}", "=".repeat(60));
    println!("- Reranker (use_cache=false): Должен работать легко");
    println!("- Embedding (use_cache=true): Может все еще требовать KV кеши");
    println!("- Но возможно обе модели упрощены для ONNX экспорта!");
    
    Ok(())
}

fn test_qwen3_model(model_path: &PathBuf, model_name: &str) -> Result<bool> {
    println!("\n🔧 Создание сессии для {}...", model_name);
    
    let mut session = Session::builder()?
        .with_optimization_level(ort::session::builder::GraphOptimizationLevel::Level3)?
        .with_intra_threads(4)?
        .commit_from_file(model_path)?;
    
    println!("✅ Сессия создана");
    println!("   Входов: {}", session.inputs.len());
    println!("   Выходов: {}", session.outputs.len());
    
    // Анализ архитектуры
    println!("\n📋 Анализ входов:");
    for (i, input) in session.inputs.iter().enumerate() {
        println!("   {}: {} - {:?}", i, input.name, input.input_type);
    }
    
    println!("\n📋 Анализ выходов:");
    for (i, output) in session.outputs.iter().enumerate() {
        println!("   {}: {} - {:?}", i, output.name, output.output_type);
    }
    
    // ТЕСТ: Попытка с базовыми входами
    println!("\n🚀 ТЕСТ: Базовые 3 входа...");
    
    let seq_len = 5;
    let input_ids = vec![151643i64, 3555, 374, 15592, 151645]; // "What is AI?"
    let attention_mask = vec![1i64; seq_len];
    let position_ids: Vec<i64> = (0..seq_len as i64).collect();
    
    let input_ids_tensor = Tensor::from_array(([1, seq_len], input_ids))?;
    let attention_mask_tensor = Tensor::from_array(([1, seq_len], attention_mask))?;
    let position_ids_tensor = Tensor::from_array(([1, seq_len], position_ids))?;
    
    let result = session.run(inputs![
        "input_ids" => input_ids_tensor,
        "attention_mask" => attention_mask_tensor,
        "position_ids" => position_ids_tensor
    ]);
    
    match result {
        Ok(outputs) => {
            println!("🎉 {} РАБОТАЕТ С 3 ВХОДАМИ!", model_name);
            println!("   Получено {} выходов", outputs.len());
            
            // Поиск полезных выходов
            let mut found_useful_output = false;
            for (name, output) in outputs.iter() {
                if let Ok((shape, data)) = output.try_extract_tensor::<f32>() {
                    let shape_vec: Vec<i64> = (0..shape.len()).map(|i| shape[i]).collect();
                    println!("   Выход '{}': форма {:?}, данных {}", name, shape_vec, data.len());
                    
                    // Ищем скрытые состояния [batch, seq, hidden]
                    if shape_vec.len() == 3 && shape_vec[0] == 1 && shape_vec[1] == seq_len as i64 {
                        let hidden_size = shape_vec[2] as usize;
                        println!("   🎯 НАЙДЕНЫ HIDDEN STATES: [1, {}, {}]", seq_len, hidden_size);
                        
                        // Простое извлечение последнего токена
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
                                found_useful_output = true;
                                println!("   🏆 {} ГОТОВ К ИСПОЛЬЗОВАНИЮ!", model_name);
                            }
                        }
                    }
                }
            }
            
            Ok(found_useful_output)
        },
        Err(e) => {
            println!("❌ {}: {}", model_name, e);
            
            if format!("{}", e).to_lowercase().contains("missing input") {
                println!("💡 Модель требует дополнительные входы (KV кеши?)");
            }
            
            Err(e.into())
        }
    }
}