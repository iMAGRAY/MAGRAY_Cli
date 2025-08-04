use memory::{
    Transaction, TransactionManager, TransactionGuard, TransactionOp, TransactionStatus, RollbackAction,
    Record, Layer
};
use uuid::Uuid;
use std::sync::Arc;
use std::thread;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

/// Комплексные unit тесты для transaction системы
/// Тестирует: transactions, operations, rollback, manager, concurrency, RAII guard

/// Тест создания и базовых операций транзакции
#[test]
fn test_transaction_creation_and_basic_ops() {
    println!("🧪 Тестируем создание и базовые операции транзакции");
    
    let mut tx = Transaction::new();
    
    // Изначально транзакция должна быть активной
    assert_eq!(tx.status(), TransactionStatus::Active);
    
    // Создаем тестовые записи
    let record1 = Record {
        id: Uuid::new_v4(),
        text: "test record 1".to_string(),
        embedding: vec![0.1; 768],
        layer: Layer::Interact,
        ..Default::default()
    };
    
    let record2 = Record {
        id: Uuid::new_v4(),
        text: "test record 2".to_string(),
        embedding: vec![0.2; 768],
        layer: Layer::Insights,
        ..Default::default()
    };
    
    // Тест вставки
    assert!(tx.insert(record1.clone()).is_ok());
    assert_eq!(tx.operations_count(), 1);
    assert_eq!(tx.rollback_actions_count(), 1);
    
    // Тест обновления
    let new_record = Record {
        id: record1.id,
        text: "updated record".to_string(),
        embedding: vec![0.3; 768],
        layer: Layer::Interact,
        ..Default::default()
    };
    
    assert!(tx.update(Layer::Interact, record1.id, new_record).is_ok());
    assert_eq!(tx.operations_count(), 2);
    assert_eq!(tx.rollback_actions_count(), 1); // Update не добавляет rollback action
    
    // Тест удаления
    assert!(tx.delete(Layer::Interact, record1.id).is_ok());
    assert_eq!(tx.operations_count(), 3);
    
    // Тест пакетной вставки
    let batch_records = vec![record1.clone(), record2.clone()];
    assert!(tx.batch_insert(batch_records.clone()).is_ok());
    assert_eq!(tx.operations_count(), 4);
    assert_eq!(tx.rollback_actions_count(), 3); // 1 + 2 from batch
    
    println!("  ✅ Транзакция создана: {} операций, {} rollback действий", 
             tx.operations_count(), tx.rollback_actions_count());
    
    println!("✅ Создание и базовые операции работают корректно");
}

/// Тест жизненного цикла транзакции
#[test]
fn test_transaction_lifecycle() {
    println!("🧪 Тестируем жизненный цикл транзакции");
    
    let mut tx = Transaction::new();
    let record = Record {
        id: Uuid::new_v4(),
        text: "lifecycle test".to_string(),
        embedding: vec![0.1; 768],
        layer: Layer::Assets,
        ..Default::default()
    };
    
    // Active state - операции разрешены
    assert_eq!(tx.status(), TransactionStatus::Active);
    assert!(tx.insert(record.clone()).is_ok());
    
    // Получаем операции и коммитим
    let ops = tx.take_operations().unwrap();
    assert_eq!(ops.len(), 1);
    assert_eq!(tx.status(), TransactionStatus::Committed);
    assert_eq!(tx.operations_count(), 0); // Операции были перемещены
    
    // После коммита операции запрещены
    let result = tx.insert(record.clone());
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("not active"));
    
    println!("  ✅ Committed state: операции запрещены");
    
    // Тест abort
    let mut tx2 = Transaction::new();
    tx2.insert(record.clone()).unwrap();
    assert_eq!(tx2.operations_count(), 1);
    assert_eq!(tx2.rollback_actions_count(), 1);
    
    tx2.abort();
    assert_eq!(tx2.status(), TransactionStatus::Aborted);
    assert_eq!(tx2.operations_count(), 0);
    assert_eq!(tx2.rollback_actions_count(), 0);
    
    // После abort операции запрещены
    let result = tx2.insert(record);
    assert!(result.is_err());
    
    println!("  ✅ Aborted state: операции очищены и запрещены");
    
    println!("✅ Жизненный цикл транзакции работает корректно");
}

/// Тест различных типов операций в транзакции
#[test]
fn test_transaction_operation_types() {
    println!("🧪 Тестируем различные типы операций");
    
    let mut tx = Transaction::new();
    
    let record1 = Record {
        id: Uuid::new_v4(),
        text: "insert test".to_string(),
        embedding: vec![0.1; 768],
        layer: Layer::Interact,
        ..Default::default()
    };
    
    let record2 = Record {
        id: Uuid::new_v4(),
        text: "update test".to_string(),
        embedding: vec![0.2; 768],
        layer: Layer::Insights,
        ..Default::default()
    };
    
    // Insert operation
    tx.insert(record1.clone()).unwrap();
    
    // Update operation
    tx.update(Layer::Interact, record1.id, record2.clone()).unwrap();
    
    // Delete operation
    tx.delete(Layer::Insights, record2.id).unwrap();
    
    // Batch insert operation
    let batch = vec![
        Record {
            id: Uuid::new_v4(),
            text: "batch 1".to_string(),
            embedding: vec![0.3; 768],
            layer: Layer::Assets,
            ..Default::default()
        },
        Record {
            id: Uuid::new_v4(),
            text: "batch 2".to_string(),
            embedding: vec![0.4; 768],
            layer: Layer::Assets,
            ..Default::default()
        },
    ];
    
    tx.batch_insert(batch.clone()).unwrap();
    
    // Проверяем операции
    let ops = tx.take_operations().unwrap();
    assert_eq!(ops.len(), 4);
    
    // Проверяем типы операций
    match &ops[0] {
        TransactionOp::Insert { record } => {
            assert_eq!(record.text, "insert test");
            assert_eq!(record.layer, Layer::Interact);
        }
        _ => panic!("Expected Insert operation"),
    }
    
    match &ops[1] {
        TransactionOp::Update { layer, id, record } => {
            assert_eq!(*layer, Layer::Interact);
            assert_eq!(*id, record1.id);
            assert_eq!(record.text, "update test");
        }
        _ => panic!("Expected Update operation"),
    }
    
    match &ops[2] {
        TransactionOp::Delete { layer, id } => {
            assert_eq!(*layer, Layer::Insights);
            assert_eq!(*id, record2.id);
        }
        _ => panic!("Expected Delete operation"),
    }
    
    match &ops[3] {
        TransactionOp::BatchInsert { records } => {
            assert_eq!(records.len(), 2);
            assert_eq!(records[0].text, "batch 1");
            assert_eq!(records[1].text, "batch 2");
        }
        _ => panic!("Expected BatchInsert operation"),
    }
    
    println!("  ✅ Все типы операций: Insert, Update, Delete, BatchInsert");
    println!("✅ Типы операций работают корректно");
}

/// Тест rollback actions и восстановления
#[test]
fn test_rollback_actions() {
    println!("🧪 Тестируем rollback actions");
    
    let mut tx = Transaction::new();
    
    let record1 = Record {
        id: Uuid::new_v4(),
        text: "rollback test 1".to_string(),
        embedding: vec![0.1; 768],
        layer: Layer::Interact,
        ..Default::default()
    };
    
    let record2 = Record {
        id: Uuid::new_v4(),
        text: "rollback test 2".to_string(),
        embedding: vec![0.2; 768],
        layer: Layer::Insights,
        ..Default::default()
    };
    
    // Insert создает DeleteInserted rollback action
    tx.insert(record1.clone()).unwrap();
    let rollback_actions = tx.rollback_actions();
    assert_eq!(rollback_actions.len(), 1);
    
    match &rollback_actions[0] {
        RollbackAction::DeleteInserted { layer, id } => {
            assert_eq!(*layer, Layer::Interact);
            assert_eq!(*id, record1.id);
        }
        _ => panic!("Expected DeleteInserted rollback action"),
    }
    
    // Batch insert создает множественные rollback actions
    let batch = vec![record1.clone(), record2.clone()];
    tx.batch_insert(batch).unwrap();
    
    let rollback_actions = tx.rollback_actions();
    assert_eq!(rollback_actions.len(), 3); // 1 + 2 from batch
    
    // Ручное добавление rollback action
    let custom_rollback = RollbackAction::RestoreDeleted { record: record2.clone() };
    tx.add_rollback_action(custom_rollback);
    
    let rollback_actions = tx.rollback_actions();
    assert_eq!(rollback_actions.len(), 4);
    
    match &rollback_actions[3] {
        RollbackAction::RestoreDeleted { record } => {
            assert_eq!(record.text, "rollback test 2");
            assert_eq!(record.layer, Layer::Insights);
        }
        _ => panic!("Expected RestoreDeleted rollback action"),
    }
    
    println!("  ✅ Rollback actions: {} действий для восстановления", rollback_actions.len());
    println!("✅ Rollback actions работают корректно");
}

/// Тест transaction manager - создание и управление транзакциями
#[test]
fn test_transaction_manager_basic() {
    println!("🧪 Тестируем basic функциональность transaction manager");
    
    let manager = TransactionManager::new();
    assert_eq!(manager.active_count(), 0);
    
    // Начинаем транзакцию
    let tx_id = manager.begin().unwrap();
    assert_eq!(manager.active_count(), 1);
    
    // Выполняем операции в транзакции
    let record = Record {
        id: Uuid::new_v4(),
        text: "manager test".to_string(),
        embedding: vec![0.1; 768],
        layer: Layer::Interact,
        ..Default::default()
    };
    
    manager.execute(tx_id, |tx| {
        tx.insert(record.clone())?;
        tx.update(Layer::Interact, record.id, record.clone())?;
        tx.delete(Layer::Interact, record.id)?;
        Ok(())
    }).unwrap();
    
    // Подготавливаем коммит
    let ops = manager.prepare_commit(tx_id).unwrap();
    assert_eq!(ops.len(), 3);
    assert_eq!(manager.active_count(), 0); // Транзакция удалена после prepare_commit
    
    println!("  ✅ Транзакция выполнена: {} операций", ops.len());
    
    // Тест rollback
    let tx_id2 = manager.begin().unwrap();
    assert_eq!(manager.active_count(), 1);
    
    manager.execute(tx_id2, |tx| {
        tx.insert(record)
    }).unwrap();
    
    manager.rollback(tx_id2).unwrap();
    assert_eq!(manager.active_count(), 0);
    
    println!("  ✅ Rollback выполнен успешно");
    
    println!("✅ Transaction manager работает корректно");
}

/// Тест transaction manager - множественные транзакции
#[test]
fn test_transaction_manager_multiple() {
    println!("🧪 Тестируем множественные транзакции");
    
    let manager = TransactionManager::new();
    
    // Создаем несколько транзакций
    let tx_ids: Vec<Uuid> = (0..5).map(|_| manager.begin().unwrap()).collect();
    assert_eq!(manager.active_count(), 5);
    
    // Добавляем операции в каждую транзакцию
    for (i, &tx_id) in tx_ids.iter().enumerate() {
        manager.execute(tx_id, |tx| {
            let record = Record {
                id: Uuid::new_v4(),
                text: format!("multi test {}", i),
                embedding: vec![0.1 * i as f32; 768],
                layer: match i % 3 {
                    0 => Layer::Interact,
                    1 => Layer::Insights,
                    _ => Layer::Assets,
                },
                ..Default::default()
            };
            
            tx.insert(record)?;
            if i % 2 == 0 {
                tx.delete(Layer::Interact, Uuid::new_v4())?;
            }
            Ok(())
        }).unwrap();
    }
    
    // Коммитим некоторые транзакции
    for i in 0..3 {
        let ops = manager.prepare_commit(tx_ids[i]).unwrap();
        assert!(ops.len() >= 1);
        println!("  ✅ Транзакция {}: {} операций", i, ops.len());
    }
    
    assert_eq!(manager.active_count(), 2); // Остались 2 транзакции
    
    // Откатываем оставшиеся
    for i in 3..5 {
        manager.rollback(tx_ids[i]).unwrap();
    }
    
    assert_eq!(manager.active_count(), 0);
    
    println!("✅ Множественные транзакции работают корректно");
}

/// Тест transaction manager - error handling
#[test]
fn test_transaction_manager_errors() {
    println!("🧪 Тестируем error handling в transaction manager");
    
    let manager = TransactionManager::new();
    
    // Тест операции с несуществующей транзакцией
    let fake_id = Uuid::new_v4();
    let result = manager.execute(fake_id, |_tx| Ok(()));
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("not found"));
    
    // Тест коммита несуществующей транзакции
    let result = manager.prepare_commit(fake_id);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("not found"));
    
    // Тест rollback несуществующей транзакции (не должен быть ошибкой)
    let result = manager.rollback(fake_id);
    assert!(result.is_ok());
    
    // Тест повторного коммита той же транзакции
    let tx_id = manager.begin().unwrap();
    manager.execute(tx_id, |tx| {
        let record = Record {
            id: Uuid::new_v4(),
            text: "error test".to_string(),
            embedding: vec![0.1; 768],
            layer: Layer::Interact,
            ..Default::default()
        };
        tx.insert(record)
    }).unwrap();
    
    let _ops = manager.prepare_commit(tx_id).unwrap();
    
    // Повторный коммит должен быть ошибкой
    let result = manager.prepare_commit(tx_id);
    assert!(result.is_err());
    
    println!("  ✅ Все error cases обработаны корректно");
    println!("✅ Error handling работает корректно");
}

/// Тест TransactionGuard - RAII автоматический rollback
#[test]
fn test_transaction_guard_raii() {
    println!("🧪 Тестируем TransactionGuard RAII");
    
    let manager = TransactionManager::new();
    
    // Тест автоматического rollback при выходе из scope
    {
        let guard = TransactionGuard::new(&manager).unwrap();
        let tx_id = guard.tx_id();
        
        assert_eq!(manager.active_count(), 1);
        
        // Выполняем операции через manager
        manager.execute(tx_id, |tx| {
            let record = Record {
                id: Uuid::new_v4(),
                text: "guard test".to_string(),
                embedding: vec![0.1; 768],
                layer: Layer::Interact,
                ..Default::default()
            };
            tx.insert(record)
        }).unwrap();
        
        // Guard выйдет из scope без explicit commit
    }
    
    // После выхода из scope транзакция должна быть откачена
    assert_eq!(manager.active_count(), 0);
    println!("  ✅ Автоматический rollback при выходе из scope");
    
    // Тест явного commit
    {
        let guard = TransactionGuard::new(&manager).unwrap();
        let tx_id = guard.tx_id();
        
        manager.execute(tx_id, |tx| {
            let record = Record {
                id: Uuid::new_v4(),
                text: "guard commit test".to_string(),
                embedding: vec![0.1; 768],
                layer: Layer::Insights,
                ..Default::default()
            };
            tx.insert(record)
        }).unwrap();
        
        let ops = guard.commit().unwrap();
        assert_eq!(ops.len(), 1);
    }
    
    // После явного commit rollback не должен произойти
    assert_eq!(manager.active_count(), 0);
    println!("  ✅ Явный commit предотвращает автоматический rollback");
    
    println!("✅ TransactionGuard RAII работает корректно");
}

/// Тест concurrency - множественные потоки
#[test]
fn test_transaction_manager_concurrency() {
    println!("🧪 Тестируем concurrency transaction manager");
    
    let manager = Arc::new(TransactionManager::new());
    let success_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let error_occurred = Arc::new(AtomicBool::new(false));
    
    // Запускаем множественные потоки для создания транзакций
    let mut handles = vec![];
    
    for i in 0..10 {
        let manager_clone = Arc::clone(&manager);
        let success_count_clone = Arc::clone(&success_count);
        let error_occurred_clone = Arc::clone(&error_occurred);
        
        let handle = thread::spawn(move || {
            match manager_clone.begin() {
                Ok(tx_id) => {
                    // Выполняем операции
                    let execute_result = manager_clone.execute(tx_id, |tx| {
                        let record = Record {
                            id: Uuid::new_v4(),
                            text: format!("concurrent test {}", i),
                            embedding: vec![0.1 * i as f32; 768],
                            layer: Layer::Interact,
                            ..Default::default()
                        };
                        tx.insert(record)?;
                        
                        // Добавляем небольшую задержку для увеличения вероятности race conditions
                        thread::sleep(Duration::from_millis(1));
                        Ok(())
                    });
                    
                    if execute_result.is_ok() {
                        // Попытка коммита
                        match manager_clone.prepare_commit(tx_id) {
                            Ok(_ops) => {
                                success_count_clone.fetch_add(1, Ordering::SeqCst);
                            }
                            Err(_) => {
                                error_occurred_clone.store(true, Ordering::SeqCst);
                            }
                        }
                    } else {
                        error_occurred_clone.store(true, Ordering::SeqCst);
                    }
                }
                Err(_) => {
                    error_occurred_clone.store(true, Ordering::SeqCst);
                }
            }
        });
        
        handles.push(handle);
    }
    
    // Ждем завершения всех потоков
    for handle in handles {
        handle.join().unwrap();
    }
    
    let final_success_count = success_count.load(Ordering::SeqCst);
    let had_errors = error_occurred.load(Ordering::SeqCst);
    
    println!("  📊 Успешных транзакций: {}/10", final_success_count);
    println!("  📊 Были ошибки: {}", had_errors);
    
    // В concurrent окружении должны быть успешные транзакции
    assert!(final_success_count > 0);
    
    // После завершения всех операций не должно быть активных транзакций
    assert_eq!(manager.active_count(), 0);
    
    println!("✅ Concurrency transaction manager работает корректно");
}

/// Тест cleanup stale transactions
#[test]
fn test_cleanup_stale_transactions() {
    println!("🧪 Тестируем cleanup stale transactions");
    
    let manager = TransactionManager::new();
    
    // Создаем несколько транзакций
    let _tx_id1 = manager.begin().unwrap();
    let _tx_id2 = manager.begin().unwrap();
    let _tx_id3 = manager.begin().unwrap();
    
    assert_eq!(manager.active_count(), 3);
    
    // Вызываем cleanup (в текущей реализации это только logging)
    manager.cleanup_stale_transactions(60);
    
    // Транзакции пока остаются (реальная логика cleanup не реализована)
    assert_eq!(manager.active_count(), 3);
    
    println!("  ⚠️ Cleanup пока только логирует (реализация будет добавлена позже)");
    
    println!("✅ Cleanup stale transactions протестирован");
}

/// Тест edge cases и граничных условий
#[test]
fn test_transaction_edge_cases() {
    println!("🧪 Тестируем edge cases транзакций");
    
    // Тест пустой транзакции
    let mut empty_tx = Transaction::new();
    let ops = empty_tx.take_operations().unwrap();
    assert_eq!(ops.len(), 0);
    assert_eq!(empty_tx.status(), TransactionStatus::Committed);
    
    // Тест транзакции с нулевым embedding
    let mut tx = Transaction::new();
    let record_zero_embedding = Record {
        id: Uuid::new_v4(),
        text: "zero embedding".to_string(),
        embedding: vec![],
        layer: Layer::Assets,
        ..Default::default()
    };
    
    assert!(tx.insert(record_zero_embedding).is_ok());
    let ops = tx.take_operations().unwrap();
    assert_eq!(ops.len(), 1);
    
    // Тест транзакции с очень большим embedding
    let mut tx2 = Transaction::new();
    let record_large_embedding = Record {
        id: Uuid::new_v4(),
        text: "large embedding".to_string(),
        embedding: vec![0.1; 10000], // Очень большой embedding
        layer: Layer::Insights,
        ..Default::default()
    };
    
    assert!(tx2.insert(record_large_embedding).is_ok());
    let ops = tx2.take_operations().unwrap();
    assert_eq!(ops.len(), 1);
    
    // Тест пустого batch insert
    let mut tx3 = Transaction::new();
    assert!(tx3.batch_insert(vec![]).is_ok());
    let ops = tx3.take_operations().unwrap();
    assert_eq!(ops.len(), 1);
    
    // Тест множественных abort
    let mut tx4 = Transaction::new();
    let record = Record {
        id: Uuid::new_v4(),
        text: "abort test".to_string(),
        embedding: vec![0.1; 768],
        layer: Layer::Interact,
        ..Default::default()
    };
    
    tx4.insert(record).unwrap();
    assert_eq!(tx4.operations_count(), 1);
    
    tx4.abort();
    assert_eq!(tx4.operations_count(), 0);
    
    // Повторный abort не должен вызывать проблем
    tx4.abort();
    assert_eq!(tx4.status(), TransactionStatus::Aborted);
    
    println!("  ✅ Edge cases: пустая транзакция, нулевой/большой embedding, множественный abort");
    
    println!("✅ Edge cases обработаны корректно");
}

/// Performance test для transaction операций
#[test]
fn test_transaction_performance() {
    println!("🧪 Тестируем производительность транзакций");
    
    let manager = TransactionManager::new();
    let start_time = std::time::Instant::now();
    
    // Множественные транзакции с операциями
    for i in 0..1000 {
        let tx_id = manager.begin().unwrap();
        
        manager.execute(tx_id, |tx| {
            let record = Record {
                id: Uuid::new_v4(),
                text: format!("perf test {}", i),
                embedding: vec![0.1; 768],
                layer: Layer::Interact,
                ..Default::default()
            };
            
            tx.insert(record)?;
            if i % 10 == 0 {
                tx.delete(Layer::Interact, Uuid::new_v4())?;
            }
            Ok(())
        }).unwrap();
        
        let _ops = manager.prepare_commit(tx_id).unwrap();
    }
    
    let elapsed = start_time.elapsed();
    println!("  📊 1000 транзакций выполнено за {:?}", elapsed);
    
    // Должно быть достаточно быстро (< 100ms для 1000 транзакций)
    assert!(elapsed.as_millis() < 100);
    
    assert_eq!(manager.active_count(), 0);
    
    println!("✅ Производительность транзакций отличная");
}

/// Integration test всей transaction системы
#[test]
fn test_transaction_system_integration() {
    println!("🧪 Integration test transaction системы");
    
    let manager = TransactionManager::new();
    
    // Сценарий 1: Успешная транзакция с множественными операциями
    {
        let guard = TransactionGuard::new(&manager).unwrap();
        let tx_id = guard.tx_id();
        
        manager.execute(tx_id, |tx| {
            let record1 = Record {
                id: Uuid::new_v4(),
                text: "integration test 1".to_string(),
                embedding: vec![0.1; 768],
                layer: Layer::Interact,
                ..Default::default()
            };
            
            let record2 = Record {
                id: Uuid::new_v4(),
                text: "integration test 2".to_string(),
                embedding: vec![0.2; 768],
                layer: Layer::Insights,
                ..Default::default()
            };
            
            tx.insert(record1.clone())?;
            tx.update(Layer::Interact, record1.id, record2.clone())?;
            tx.delete(Layer::Insights, record2.id)?;
            tx.batch_insert(vec![record1, record2])?;
            
            Ok(())
        }).unwrap();
        
        let ops = guard.commit().unwrap();
        assert_eq!(ops.len(), 4);
        
        println!("  ✅ Сценарий 1: успешная комплексная транзакция");
    }
    
    // Сценарий 2: Транзакция с автоматическим rollback
    {
        let guard = TransactionGuard::new(&manager).unwrap();
        let tx_id = guard.tx_id();
        
        manager.execute(tx_id, |tx| {
            let record = Record {
                id: Uuid::new_v4(),
                text: "rollback test".to_string(),
                embedding: vec![0.3; 768],
                layer: Layer::Assets,
                ..Default::default()
            };
            
            tx.insert(record)?;
            Ok(())
        }).unwrap();
        
        // Guard выходит из scope без commit - автоматический rollback
    }
    
    // Сценарий 3: Множественные concurrent транзакции
    let manager_arc = Arc::new(manager);
    let handles: Vec<_> = (0..5).map(|i| {
        let manager_clone = Arc::clone(&manager_arc);
        thread::spawn(move || {
            let guard = TransactionGuard::new(&*manager_clone).unwrap();
            let tx_id = guard.tx_id();
            
            manager_clone.execute(tx_id, |tx| {
                let record = Record {
                    id: Uuid::new_v4(),
                    text: format!("concurrent integration {}", i),
                    embedding: vec![0.1 * i as f32; 768],
                    layer: Layer::Interact,
                    ..Default::default()
                };
                
                tx.insert(record)
            }).unwrap();
            
            guard.commit().unwrap()
        })
    }).collect();
    
    for handle in handles {
        let ops = handle.join().unwrap();
        assert_eq!(ops.len(), 1);
    }
    
    println!("  ✅ Сценарий 2: автоматический rollback");
    println!("  ✅ Сценарий 3: concurrent транзакции");
    
    assert_eq!(manager_arc.active_count(), 0);
    
    println!("✅ Integration test transaction системы успешен");
}

/// Quick smoke test для всех основных функций
#[test]
fn test_transaction_smoke() {
    // Test transaction creation
    let mut tx = Transaction::new();
    assert_eq!(tx.status(), TransactionStatus::Active);
    
    // Test basic operations
    let record = Record {
        id: Uuid::new_v4(),
        text: "smoke test".to_string(),
        embedding: vec![0.1; 768],
        layer: Layer::Interact,
        ..Default::default()
    };
    
    tx.insert(record.clone()).unwrap();
    tx.update(Layer::Interact, record.id, record.clone()).unwrap();
    tx.delete(Layer::Interact, record.id).unwrap();
    tx.batch_insert(vec![record]).unwrap();
    
    // Test commit
    let ops = tx.take_operations().unwrap();
    assert_eq!(ops.len(), 4);
    
    // Test manager
    let manager = TransactionManager::new();
    let tx_id = manager.begin().unwrap();
    let _ops = manager.prepare_commit(tx_id).unwrap();
    
    // Test guard
    let guard = TransactionGuard::new(&manager).unwrap();
    let _ops = guard.commit().unwrap();
    
    println!("✅ Все функции transaction работают");
}