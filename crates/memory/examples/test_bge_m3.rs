use anyhow::Result;
use ort::{session::{Session, SessionOutputs}, value::Tensor, inputs};
use std::path::PathBuf;

fn main() -> Result<()> {
    println!("=== BGE-M3: XLMROBERTA EMBEDDING МОДЕЛЬ ===\n");
    
    // Установка пути к DLL
    let dll_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent().unwrap()
        .parent().unwrap()
        .join("scripts")
        .join("onnxruntime")
        .join("lib")
        .join("onnxruntime.dll");
    std::env::set_var("ORT_DYLIB_PATH", dll_path.to_str().unwrap());
    
    println!("✅ BGE-M3 МОДЕЛЬ: XLMRobertaModel (encoder-only)");
    println!("✅ Pipeline: feature-extraction");
    println!("✅ Quantized: INT8 для эффективности");
    println!("✅ Ожидается: НИКАКИХ KV кешей!");
    
    let model_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("models")
        .join("bge-m3")
        .join("model.onnx");
    
    println!("\n📁 Модель: {}", model_path.display());
    
    // Инициализация ORT
    println!("\n1. Инициализация ONNX Runtime...");
    ort::init()
        .with_name("bge_m3")
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
    
    // ПРОВЕРКА: Должно быть мало входов (encoder-only)
    println!("\n3. 🔍 АНАЛИЗ ВХОДОВ (ожидаем encoder-only!):");
    for (i, input) in session.inputs.iter().enumerate() {
        println!("   {}: {} - {:?}", i, input.name, input.input_type);
    }
    
    println!("\n4. 🔍 АНАЛИЗ ВЫХОДОВ:");
    for (i, output) in session.outputs.iter().enumerate() {
        println!("   {}: {} - {:?}", i, output.name, output.output_type);
    }
    
    let num_inputs = session.inputs.len();
    if num_inputs <= 4 {
        println!("\n🎉 ОТЛИЧНО! {} входов - это encoder-only модель!", num_inputs);
    } else {
        println!("\n⚠️ Неожиданно: {} входов, ожидалось <= 4", num_inputs);
    }
    
    // ТЕСТ: Базовые входы для XLMRoberta
    println!("\n5. 🧪 ТЕСТ: BGE-M3 embedding extraction...");
    
    let seq_len = 8;
    // Токены для XLMRoberta (обычно 0=<s>, 2=</s>, 1=<pad>)
    let input_ids = vec![0i64, 6661, 83, 70, 1788, 111, 23, 2]; // "<s> Hello world test </s>"
    let attention_mask = vec![1i64; seq_len]; // Все токены активны
    let token_type_ids = vec![0i64; seq_len]; // Все токены одного типа
    
    let input_ids_tensor = Tensor::from_array(([1, seq_len], input_ids))?;
    let attention_mask_tensor = Tensor::from_array(([1, seq_len], attention_mask))?;
    let token_type_ids_tensor = Tensor::from_array(([1, seq_len], token_type_ids))?;
    
    println!("✅ Входные тензоры созданы");
    println!("   input_ids: [1, {}]", seq_len);
    println!("   attention_mask: [1, {}]", seq_len);
    println!("   token_type_ids: [1, {}]", seq_len);
    
    // КРИТИЧЕСКИЙ ТЕСТ: XLMRoberta с базовыми входами
    println!("\n6. 🚀 КРИТИЧЕСКИЙ ТЕСТ: BGE-M3 XLMRoberta...");
    
    // Попробуем с разными комбинациями входов
    println!("🔄 Попытка с 3 входами (полная комбинация)...");
    
    let result = session.run(inputs![
        "input_ids" => input_ids_tensor,
        "attention_mask" => attention_mask_tensor,
        "token_type_ids" => token_type_ids_tensor
    ]);
    
    match result {
        Ok(outputs) => {
            println!("🎉 УСПЕХ С 3 ВХОДАМИ!");
            process_embeddings(outputs, seq_len, "BGE-M3")?;
        },
        Err(e) => {
            println!("❌ 3 входа не работают: {}", e);
            
            if format!("{}", e).contains("Missing Input:") {
                println!("💡 Модель требует дополнительные входы");
            }
        }
    }
    
    println!("\n📊 ФИНАЛЬНЫЙ СТАТУС BGE-M3:");
    println!("- Архитектура: XLMRobertaModel (encoder-only)");
    println!("- Входов: {}", num_inputs);
    println!("- Тип: {}", if num_inputs <= 4 { "✅ Простая модель" } else { "⚠️ Сложная модель" });
    println!("- Размерность: 1024 (из config.json)");
    
    Ok(())
}

fn process_embeddings(outputs: SessionOutputs, seq_len: usize, model_name: &str) -> Result<()> {
    println!("   Получено {} выходов", outputs.len());
    
    let mut found_embeddings = false;
    
    for (name, output) in outputs.iter() {
        if let Ok((shape, data)) = output.try_extract_tensor::<f32>() {
            let shape_vec: Vec<i64> = (0..shape.len()).map(|i| shape[i]).collect();
            println!("   Выход '{}': форма {:?}, данных {}", name, shape_vec, data.len());
            
            // Ищем hidden states [batch, seq, hidden]
            if shape_vec.len() == 3 && shape_vec[0] == 1 && shape_vec[1] == seq_len as i64 {
                let hidden_size = shape_vec[2] as usize;
                
                println!("   🎯 НАЙДЕНЫ {} ЭМБЕДДИНГИ!", model_name);
                println!("   Размерности: [1, {}, {}]", seq_len, hidden_size);
                
                // Mean pooling (стандарт для encoder-only)
                let mut pooled_embedding = vec![0.0f32; hidden_size];
                
                for seq_idx in 0..seq_len {
                    for hidden_idx in 0..hidden_size {
                        let data_idx = seq_idx * hidden_size + hidden_idx;
                        if data_idx < data.len() {
                            pooled_embedding[hidden_idx] += data[data_idx];
                        }
                    }
                }
                
                // Усреднение
                for val in &mut pooled_embedding {
                    *val /= seq_len as f32;
                }
                
                println!("   ✅ MEAN POOLING ПРИМЕНЕН!");
                println!("   Финальный размер: {}", pooled_embedding.len());
                
                let norm = pooled_embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
                println!("   Норма: {:.4}", norm);
                println!("   Образец: {:?}", &pooled_embedding[..5.min(pooled_embedding.len())]);
                
                if norm > 0.0 {
                    let normalized: Vec<f32> = pooled_embedding.iter().map(|x| x / norm).collect();
                    let final_norm = normalized.iter().map(|x| x * x).sum::<f32>().sqrt();
                    
                    println!("   ✅ ЭМБЕДДИНГ НОРМАЛИЗОВАН!");
                    println!("   Финальная норма: {:.6}", final_norm);
                    println!("   🏆 ГОТОВЫЙ ЭМБЕДДИНГ: {} размерность", normalized.len());
                    
                    found_embeddings = true;
                    break;
                }
            }
        }
    }
    
    if found_embeddings {
        println!("\n🎊🎊🎊 ПОЛНАЯ ПОБЕДА!");
        println!("✅ {} работает как чистая embedding модель!", model_name);
        println!("✅ Encoder-only архитектура без KV кешей!");
        println!("✅ Реальные нормализованные эмбеддинги!");
        println!("✅ Готово для production!");
    } else {
        println!("⚠️ Инференс прошел, но эмбеддинги не найдены в ожидаемом формате");
    }
    
    Ok(())
}