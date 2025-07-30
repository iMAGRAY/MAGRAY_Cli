use anyhow::Result;
use ort::{session::Session, value::Tensor, inputs};
use std::path::PathBuf;

fn main() -> Result<()> {
    println!("=== MXBAI RERANK BASE V2: АНАЛИЗ АРХИТЕКТУРЫ ===\n");
    
    // Установка пути к DLL
    let dll_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent().unwrap()
        .parent().unwrap()
        .join("scripts")
        .join("onnxruntime")
        .join("lib")
        .join("onnxruntime.dll");
    std::env::set_var("ORT_DYLIB_PATH", dll_path.to_str().unwrap());
    
    // Путь к модели MXBai
    let model_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("models")
        .join("mxbai_rerank_base_v2")
        .join("model.onnx");
    
    println!("🤔 ВАЖНЫЙ ВОПРОС: Пользователь сказал 'mxbai не генеративная'");
    println!("📋 НО config.json показывает: Qwen2ForCausalLM");
    println!("🧪 ПРОВЕРИМ: Может ли CausalLM работать как reranker?");
    println!("📁 Модель: {}", model_path.display());
    println!("✅ Модель существует: {}", model_path.exists());
    
    if !model_path.exists() {
        return Err(anyhow::anyhow!("Файл модели не найден"));
    }
    
    // Инициализация ORT
    println!("\n1. Инициализация ONNX Runtime...");
    ort::init()
        .with_name("mxbai_rerank")
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
    
    // КРИТИЧЕСКИЙ АНАЛИЗ: Количество входов покажет истину
    println!("\n3. 🔍 ДЕТАЛЬНЫЙ АНАЛИЗ ВХОДОВ:");
    for (i, input) in session.inputs.iter().enumerate() {
        println!("   {}: {} - {:?}", i, input.name, input.input_type);
    }
    
    // Анализ выходов модели
    println!("\n4. 🔍 АНАЛИЗ ВЫХОДОВ:");
    for (i, output) in session.outputs.iter().enumerate() {
        println!("   {}: {} - {:?}", i, output.name, output.output_type);
    }
    
    // ГИПОТЕЗА: Если входов мало (3-4), то возможно reranker упрощен
    let num_inputs = session.inputs.len();
    println!("\n5. 🧠 АНАЛИЗ СЛОЖНОСТИ:");
    
    if num_inputs <= 4 {
        println!("   ✅ Входов мало ({}): Возможно упрощенная архитектура!", num_inputs);
        println!("   💡 Может работать без полных KV кешей");
        
        // Пробуем базовые входы для reranker
        println!("\n6. 🧪 ТЕСТ: Базовые входы для reranker...");
        
        let seq_len = 6;
        let input_ids = vec![151643i64, 3555, 374, 15592, 1029, 151645]; // "What is AI?" + tokens
        let attention_mask = vec![1i64; seq_len];
        let position_ids: Vec<i64> = (0..seq_len as i64).collect();
        
        let input_ids_tensor = Tensor::from_array(([1, seq_len], input_ids))?;
        let attention_mask_tensor = Tensor::from_array(([1, seq_len], attention_mask))?;
        let position_ids_tensor = Tensor::from_array(([1, seq_len], position_ids))?;
        
        println!("✅ Создали тензоры для reranker теста");
        
        // Попытка inference с минимальными входами
        let result = session.run(inputs![
            "input_ids" => input_ids_tensor,
            "attention_mask" => attention_mask_tensor,
            "position_ids" => position_ids_tensor
        ]);
        
        match result {
            Ok(outputs) => {
                println!("🎉 НЕВЕРОЯТНО! MXBAI РАБОТАЕТ С 3 ВХОДАМИ!");
                println!("   Получено {} выходов", outputs.len());
                
                // Ищем reranking scores
                for (name, output) in outputs.iter() {
                    if let Ok((shape, data)) = output.try_extract_tensor::<f32>() {
                        let shape_vec: Vec<i64> = (0..shape.len()).map(|i| shape[i]).collect();
                        println!("   Выход '{}': форма {:?}, данных {}", name, shape_vec, data.len());
                        
                        // Для reranker ищем скаляр или простой вектор
                        if shape_vec.len() <= 2 && data.len() > 0 {
                            println!("   🎯 ВОЗМОЖНЫЕ RERANK SCORES!");
                            println!("   Данные: {:?}", &data[..data.len().min(10)]);
                        }
                    }
                }
                
                println!("\n✅ ОТКРЫТИЕ: MXBai CausalLM может работать как простой reranker!");
                println!("✅ Не требует полных KV кешей для reranking задач!");
                
            },
            Err(e) => {
                println!("❌ Все еще нужны дополнительные входы: {}", e);
                
                if format!("{}", e).contains("Missing Input:") {
                    println!("💡 Модель требует больше входов чем базовые 3");
                    println!("💡 Возможно все-таки полная CausalLM архитектура");
                }
            }
        }
        
    } else {
        println!("   ❌ Входов много ({}): Полная CausalLM архитектура", num_inputs);
        println!("   💡 Вероятно нужны KV кеши как у Qwen3");
    }
    
    // ФИНАЛЬНОЕ ЗАКЛЮЧЕНИЕ
    println!("\n7. 🏆 ЗАКЛЮЧЕНИЕ О MXBAI:");
    println!("- Config: Qwen2ForCausalLM (генеративная архитектура)");
    println!("- Входов: {}", num_inputs);
    
    if num_inputs <= 4 {
        println!("- Статус: Возможно упрощенная для reranking");
        println!("- Прогноз: Может работать без полных KV кешей");
    } else {
        println!("- Статус: Полная генеративная модель");
        println!("- Прогноз: Потребует все KV кеши как Qwen3");
    }
    
    println!("\n🤔 ПАРАДОКС РЕШЕН?");
    println!("Пользователь мог иметь в виду:");
    println!("- MXBai используется для reranking (задача не генеративная)");
    println!("- Но архитектура все еще CausalLM (генеративная)");
    println!("- Возможно fine-tuned для reranking с упрощенными требованиями");
    
    Ok(())
}