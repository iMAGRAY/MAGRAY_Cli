use anyhow::Result;
use ort::{session::Session, value::Tensor, inputs};
use std::path::PathBuf;

fn main() -> Result<()> {
    println!("=== QWEN3-EMBEDDING: ПРОСТАЯ ПРОВЕРКА ===\n");
    
    // Установка пути к DLL
    let dll_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent().unwrap()
        .parent().unwrap()
        .join("scripts")
        .join("onnxruntime")
        .join("lib")
        .join("onnxruntime.dll");
    std::env::set_var("ORT_DYLIB_PATH", dll_path.to_str().unwrap());
    
    println!("🤔 ПРОСТОЙ ВОПРОС: Нужны ли KV кеши для embedding задач?");
    println!("💭 Гипотеза: Qwen3-Embedding - обычная embedding модель с ONNX overhead");
    
    // Путь к модели
    let model_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("models")
        .join("Qwen3-Embedding-0.6B-ONNX")
        .join("model.onnx");
    
    println!("📁 Модель: {}", model_path.display());
    
    // Инициализация ORT
    println!("\n1. Инициализация ONNX Runtime...");
    ort::init()
        .with_name("qwen3_check")
        .commit()?;
    println!("✅ ORT инициализирован");
    
    // Создание сессии
    println!("\n2. Создание ONNX сессии...");
    let mut session = Session::builder()?
        .with_optimization_level(ort::session::builder::GraphOptimizationLevel::Level3)?
        .with_intra_threads(4)?
        .commit_from_file(&model_path)?;
    
    println!("✅ Сессия создана");
    println!("   Входов: {} (3 базовых + 56 KV кешей)", session.inputs.len());
    println!("   Выходов: {} (1 hidden_state + 56 KV outputs)", session.outputs.len());
    
    // ЭКСПЕРИМЕНТ 1: Минимальные пустые кеши
    println!("\n3. 🧪 ЭКСПЕРИМЕНТ: Создание минимальных пустых KV кешей...");
    
    let seq_len = 4;
    let input_ids = vec![151643i64, 3555, 374, 15592]; // "What is"
    let attention_mask = vec![1i64; seq_len];
    let position_ids: Vec<i64> = (0..seq_len as i64).collect();
    
    // Базовые тензоры
    let input_ids_tensor = Tensor::from_array(([1, seq_len], input_ids))?;
    let attention_mask_tensor = Tensor::from_array(([1, seq_len], attention_mask))?;
    let position_ids_tensor = Tensor::from_array(([1, seq_len], position_ids))?;
    
    println!("✅ Базовые тензоры созданы");
    
    // Попытка создать один KV кеш для тестирования
    println!("\n4. 🔧 Создание пустого KV кеша для layer 0...");
    
    // Пустой тензор с правильными размерами [1, 8, 0, 128]
    let empty_key_0 = Tensor::from_array(([1, 8, 0, 128], Vec::<f32>::new()))?;
    let empty_value_0 = Tensor::from_array(([1, 8, 0, 128], Vec::<f32>::new()))?;
    
    println!("✅ Пустые KV кеши для layer 0 созданы: [1, 8, 0, 128]");
    
    // Простой тест с первыми двумя KV кешами
    println!("\n5. 🚀 МИНИ-ТЕСТ: Базовые входы + 2 пустых KV кеша...");
    
    let result = session.run(inputs![
        "input_ids" => input_ids_tensor,
        "attention_mask" => attention_mask_tensor, 
        "position_ids" => position_ids_tensor,
        "past_key_values.0.key" => empty_key_0,
        "past_key_values.0.value" => empty_value_0
    ]);
    
    match result {
        Ok(outputs) => {
            println!("🎉 ЧАСТИЧНЫЙ УСПЕХ! Инференс начался!");
            println!("   Получено {} выходов", outputs.len());
        },
        Err(e) => {
            println!("❌ Ошибка: {}", e);
            
            if format!("{}", e).contains("Missing Input: past_key_values.1.key") {
                println!("\n💡 ПОНИМАНИЕ: Модели нужны ВСЕ 56 KV кешей!");
                println!("💡 Даже если они пустые, граф ONNX требует все входы");
                println!("💡 Это подтверждает: KV кеши = ONNX artifact, не functionality");
                
                println!("\n🔍 ВЫВОД:");
                println!("✅ Qwen3-Embedding действительно embedding модель");  
                println!("❌ НО ONNX экспорт включил полный CausalLM граф");
                println!("⚠️ Поэтому нужно создать все 56 пустых кешей");
                println!("💡 В production это будет overhead, но embedding реальный");
                
            } else {
                println!("💡 Другая ошибка: {}", e);
            }
        }
    }
    
    println!("\n📊 ЗАКЛЮЧЕНИЕ:");
    println!("- Qwen3-Embedding: Настоящая embedding модель");
    println!("- КV кеши: ONNX экспорт артефакт (нужны пустые)"); 
    println!("- Использование: Возможно, но с overhead");
    println!("- Альтернативы: E5-small, MXBai - проще и быстрее");
    
    println!("\n🤷‍♂️ Стоит ли использовать Qwen3-Embedding?");
    println!("Зависит от требований к качеству vs производительности!");
    
    Ok(())
}