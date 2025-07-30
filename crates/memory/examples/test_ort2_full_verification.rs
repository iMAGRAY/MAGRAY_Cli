use anyhow::Result;
use ort::{inputs, session::Session};
use std::path::PathBuf;

fn main() -> Result<()> {
    println!("=== ПОЛНАЯ ВЕРИФИКАЦИЯ ORT 2.0 ===\n");
    
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
    
    println!("Модель: {}", model_path.display());
    println!("Модель существует: {}", model_path.exists());
    
    if !model_path.exists() {
        println!("\n❌ ФАЙЛ МОДЕЛИ НЕ НАЙДЕН!");
        return Err(anyhow::anyhow!("Модель не найдена"));
    }
    
    // 1. Инициализация ONNX Runtime
    println!("\n1. Инициализация ONNX Runtime...");
    match ort::init()
        .with_name("verification_test")
        .commit() {
        Ok(_) => println!("✅ ORT инициализирован"),
        Err(e) => {
            println!("❌ ОШИБКА инициализации ORT: {}", e);
            return Err(e.into());
        }
    }
    
    // 2. Создание сессии
    println!("\n2. Создание ONNX сессии...");
    let session = match Session::builder() {
        Ok(builder) => {
            match builder
                .with_optimization_level(ort::session::builder::GraphOptimizationLevel::Level3) {
                Ok(builder) => {
                    match builder.commit_from_file(&model_path) {
                        Ok(session) => {
                            println!("✅ Сессия создана");
                            session
                        },
                        Err(e) => {
                            println!("❌ ОШИБКА создания сессии: {}", e);
                            return Err(e.into());
                        }
                    }
                },
                Err(e) => {
                    println!("❌ ОШИБКА конфигурации: {}", e);
                    return Err(e.into());
                }
            }
        },
        Err(e) => {
            println!("❌ ОШИБКА создания builder: {}", e);
            return Err(e.into());
        }
    };
    
    // 3. Проверка входов модели
    println!("\n3. Анализ входов модели:");
    println!("Количество входов: {}", session.inputs.len());
    
    let mut required_inputs = Vec::new();
    for (i, input) in session.inputs.iter().enumerate() {
        println!("  {}: {} (тип: {:?})", i, input.name, input.input_type);
        
        // Определяем обязательные входы
        if input.name == "input_ids" || 
           input.name == "attention_mask" || 
           input.name == "position_ids" {
            required_inputs.push(input.name.clone());
        }
    }
    
    println!("Обязательные входы: {:?}", required_inputs);
    
    // 4. Создание тестовых тензоров
    println!("\n4. Создание тестовых тензоров...");
    
    let seq_len = 4;
    
    // input_ids: [101, 7592, 2088, 102] - типичные BERT токены
    let input_ids = vec![101i64, 7592, 2088, 102];
    let attention_mask = vec![1i64, 1, 1, 1];
    let position_ids = vec![0i64, 1, 2, 3]; // Позиционные индексы
    
    println!("Создание input_ids тензора...");
    let input_ids_tensor = match ort::value::Tensor::from_array(([1, seq_len], input_ids)) {
        Ok(tensor) => {
            println!("✅ input_ids тензор создан");
            tensor
        },
        Err(e) => {
            println!("❌ ОШИБКА создания input_ids: {}", e);
            return Err(e.into());
        }
    };
    
    println!("Создание attention_mask тензора...");
    let attention_mask_tensor = match ort::value::Tensor::from_array(([1, seq_len], attention_mask)) {
        Ok(tensor) => {
            println!("✅ attention_mask тензор создан");
            tensor
        },
        Err(e) => {
            println!("❌ ОШИБКА создания attention_mask: {}", e);
            return Err(e.into());
        }
    };
    
    println!("Создание position_ids тензора...");
    let position_ids_tensor = match ort::value::Tensor::from_array(([1, seq_len], position_ids)) {
        Ok(tensor) => {
            println!("✅ position_ids тензор создан");
            tensor
        },
        Err(e) => {
            println!("❌ ОШИБКА создания position_ids: {}", e);
            return Err(e.into());
        }
    };
    
    // 5. КРИТИЧЕСКИЙ ТЕСТ: Запуск инференса
    println!("\n5. 🔥 КРИТИЧЕСКИЙ ТЕСТ: Запуск ONNX инференса...");
    
    let session = std::sync::Mutex::new(session);
    let mut session_guard = session.lock().unwrap();
    
    println!("Подготовка inputs! макроса...");
    let inputs_vec = inputs![
        "input_ids" => input_ids_tensor,
        "attention_mask" => attention_mask_tensor,
        "position_ids" => position_ids_tensor
    ];
    
    println!("Вызов session.run()...");
    let outputs = match session_guard.run(inputs_vec) {
        Ok(outputs) => {
            println!("🎉 УСПЕХ! Инференс выполнен!");
            outputs
        },
        Err(e) => {
            println!("❌ КРИТИЧЕСКАЯ ОШИБКА инференса: {}", e);
            println!("❌ ORT 2.0 НЕ РАБОТАЕТ ПОЛНОСТЬЮ");
            return Err(e.into());
        }
    };
    
    // 6. Анализ выходов
    println!("\n6. 🎯 АНАЛИЗ ВЫХОДНЫХ ДАННЫХ:");
    println!("Количество выходов: {}", outputs.len());
    
    let mut embedding_found = false;
    
    for (name, output) in outputs.iter() {
        println!("\nВыход '{}': {:?}", name, output.dtype());
        
        match output.try_extract_tensor::<f32>() {
            Ok((shape, data)) => {
                let shape_vec: Vec<i64> = (0..shape.len()).map(|i| shape[i]).collect();
                println!("   Форма: {:?}", shape_vec);
                println!("   Размер данных: {}", data.len());
                
                if data.len() > 0 {
                    println!("   Первые 5 значений: {:?}", &data[..5.min(data.len())]);
                    
                    // Проверяем, похоже ли на эмбеддинги
                    if shape_vec.len() == 3 && shape_vec[2] > 500 { // [batch, seq, hidden]
                        println!("   🎯 НАЙДЕНЫ ЭМБЕДДИНГИ! Размерность: {}", shape_vec[2]);
                        embedding_found = true;
                    }
                    
                    let min = data.iter().fold(f32::INFINITY, |a, &b| a.min(b));
                    let max = data.iter().fold(f32::NEG_INFINITY, |a, &b| a.max(b));
                    let mean = data.iter().sum::<f32>() / data.len() as f32;
                    println!("   Статистика: min={:.4}, max={:.4}, mean={:.4}", min, max, mean);
                }
            },
            Err(e) => {
                println!("   Не удалось извлечь как f32: {}", e);
            }
        }
    }
    
    // 7. ФИНАЛЬНАЯ ПРОВЕРКА
    println!("\n7. 🏁 ФИНАЛЬНАЯ ПРОВЕРКА:");
    
    if embedding_found {
        println!("✅ ✅ ✅ ORT 2.0 ПОЛНОСТЬЮ РАБОТАЕТ!");
        println!("✅ ✅ ✅ РЕАЛЬНЫЕ ЭМБЕДДИНГИ ГЕНЕРИРУЮТСЯ!");
        println!("✅ ✅ ✅ ИНТЕГРАЦИЯ УСПЕШНА!");
    } else {
        println!("⚠️  Инференс работает, но эмбеддинги не найдены");
        println!("⚠️  Возможно, нужна дополнительная обработка выходов");
    }
    
    println!("\n🎉 ВЕРИФИКАЦИЯ ЗАВЕРШЕНА!");
    
    Ok(())
}