use memory::{
    Transaction, TransactionManager, TransactionGuard, 
    Record, Layer
};
use memory::transaction::{TransactionOp, TransactionStatus, RollbackAction};
use uuid::Uuid;
use std::sync::Arc;
use chrono::Utc;

/// Базовый smoke test для transaction API
#[test]
fn test_transaction_smoke() {
    let mut tx = Transaction::new();
    assert_eq!(tx.status(), &TransactionStatus::Active);
    
    let record = Record {
        id: Uuid::new_v4(),
        text: "smoke test".to_string(),
        embedding: vec![0.1; 768],
        layer: Layer::Interact,
        kind: "test".to_string(),
        tags: vec![],
        project: "test".to_string(),
        session: "test".to_string(),
        ts: Utc::now(),
        score: 0.0,
        access_count: 0,
        last_access: Utc::now(),
    };
    
    // Test basic operations
    tx.insert(record.clone()).unwrap();
    assert_eq!(tx.operations_count(), 1);
    assert_eq!(tx.rollback_actions_count(), 1);
    
    tx.update(Layer::Interact, record.id, record.clone()).unwrap();
    assert_eq!(tx.operations_count(), 2);
    
    tx.delete(Layer::Interact, record.id).unwrap();
    assert_eq!(tx.operations_count(), 3);
    
    tx.batch_insert(vec![record]).unwrap();
    assert_eq!(tx.operations_count(), 4);
    assert_eq!(tx.rollback_actions_count(), 2); // 1 from insert + 1 from batch
    
    // Test commit
    let ops = tx.take_operations().unwrap();
    assert_eq!(ops.len(), 4);
    assert_eq!(tx.status(), &TransactionStatus::Committed);
    
    println!("✅ Transaction smoke test passed");
}

/// Test transaction manager basic functionality
#[test]
fn test_transaction_manager_basic() {
    let manager = TransactionManager::new();
    assert_eq!(manager.active_count(), 0);
    
    // Begin transaction
    let tx_id = manager.begin().unwrap();
    assert_eq!(manager.active_count(), 1);
    
    // Execute operation
    let record = Record {
        id: Uuid::new_v4(),
        text: "manager test".to_string(),
        embedding: vec![0.1; 768],
        layer: Layer::Interact,
        kind: "test".to_string(),
        tags: vec![],
        project: "test".to_string(),
        session: "test".to_string(),
        ts: Utc::now(),
        score: 0.0,
        access_count: 0,
        last_access: Utc::now(),
    };
    
    manager.execute(tx_id, |tx| {
        tx.insert(record.clone())?;
        tx.update(Layer::Interact, record.id, record.clone())?;
        tx.delete(Layer::Interact, record.id)?;
        Ok(())
    }).unwrap();
    
    // Prepare commit
    let ops = manager.prepare_commit(tx_id).unwrap();
    assert_eq!(ops.len(), 3);
    assert_eq!(manager.active_count(), 0); // Transaction removed after prepare_commit
    
    println!("✅ Transaction manager basic test passed");
}

/// Test transaction guard RAII
#[test] 
fn test_transaction_guard_raii() {
    let manager = TransactionManager::new();
    
    // Test auto-rollback on drop
    {
        let guard = TransactionGuard::new(&manager).unwrap();
        let tx_id = guard.tx_id();
        
        assert_eq!(manager.active_count(), 1);
        
        manager.execute(tx_id, |tx| {
            let record = Record {
                id: Uuid::new_v4(),
                text: "guard test".to_string(),
                embedding: vec![0.1; 768],
                layer: Layer::Interact,
                kind: "test".to_string(),
                tags: vec![],
                project: "test".to_string(),
                session: "test".to_string(),
                ts: Utc::now(),
                score: 0.0,
                access_count: 0,
                last_access: Utc::now(),
            };
            tx.insert(record)
        }).unwrap();
        
        // Guard will drop without explicit commit
    }
    
    // After scope exit, transaction should be rolled back
    assert_eq!(manager.active_count(), 0);
    
    // Test explicit commit
    {
        let guard = TransactionGuard::new(&manager).unwrap();
        let tx_id = guard.tx_id();
        
        manager.execute(tx_id, |tx| {
            let record = Record {
                id: Uuid::new_v4(),
                text: "guard commit test".to_string(),
                embedding: vec![0.1; 768],
                layer: Layer::Insights,
                kind: "test".to_string(),
                tags: vec![],
                project: "test".to_string(),
                session: "test".to_string(),
                ts: Utc::now(),
                score: 0.0,
                access_count: 0,
                last_access: Utc::now(),
            };
            tx.insert(record)
        }).unwrap();
        
        let ops = guard.commit().unwrap();
        assert_eq!(ops.len(), 1);
    }
    
    // After explicit commit, no rollback should occur
    assert_eq!(manager.active_count(), 0);
    println!("✅ Transaction guard RAII test passed");
}

/// Test transaction lifecycle
#[test]
fn test_transaction_lifecycle() {
    let mut tx = Transaction::new();
    let record = Record {
        id: Uuid::new_v4(),
        text: "lifecycle test".to_string(),
        embedding: vec![0.1; 768],
        layer: Layer::Assets,
        kind: "test".to_string(),
        tags: vec![],
        project: "test".to_string(),
        session: "test".to_string(),
        ts: Utc::now(),
        score: 0.0,
        access_count: 0,
        last_access: Utc::now(),
    };
    
    // Active state - operations allowed
    assert_eq!(tx.status(), &TransactionStatus::Active);
    assert!(tx.insert(record.clone()).is_ok());
    
    // Get operations and commit
    let ops = tx.take_operations().unwrap();
    assert_eq!(ops.len(), 1);
    assert_eq!(tx.status(), &TransactionStatus::Committed);
    assert_eq!(tx.operations_count(), 0); // Operations were moved
    
    // After commit, operations are forbidden
    let result = tx.insert(record.clone());
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("not active"));
    
    // Test abort
    let mut tx2 = Transaction::new();
    tx2.insert(record.clone()).unwrap();
    assert_eq!(tx2.operations_count(), 1);
    assert_eq!(tx2.rollback_actions_count(), 1);
    
    tx2.abort();
    assert_eq!(tx2.status(), &TransactionStatus::Aborted);
    assert_eq!(tx2.operations_count(), 0);
    assert_eq!(tx2.rollback_actions_count(), 0);
    
    // After abort, operations are forbidden
    let result = tx2.insert(record);
    assert!(result.is_err());
    
    println!("✅ Transaction lifecycle test passed");
}

/// Test rollback actions
#[test]
fn test_rollback_actions() {
    let mut tx = Transaction::new();
    
    let record1 = Record {
        id: Uuid::new_v4(),
        text: "rollback test 1".to_string(),
        embedding: vec![0.1; 768],
        layer: Layer::Interact,
        kind: "test".to_string(),
        tags: vec![],
        project: "test".to_string(),
        session: "test".to_string(),
        ts: Utc::now(),
        score: 0.0,
        access_count: 0,
        last_access: Utc::now(),
    };
    
    let record2 = Record {
        id: Uuid::new_v4(),
        text: "rollback test 2".to_string(),
        embedding: vec![0.2; 768],
        layer: Layer::Insights,
        kind: "test".to_string(),
        tags: vec![],
        project: "test".to_string(),
        session: "test".to_string(),
        ts: Utc::now(),
        score: 0.0,
        access_count: 0,
        last_access: Utc::now(),
    };
    
    // Insert creates DeleteInserted rollback action
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
    
    // Batch insert creates multiple rollback actions
    let batch = vec![record1.clone(), record2.clone()];
    tx.batch_insert(batch).unwrap();
    
    let rollback_actions = tx.rollback_actions();
    assert_eq!(rollback_actions.len(), 3); // 1 + 2 from batch
    
    // Manual rollback action addition
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
    
    println!("✅ Rollback actions test passed");
}

/// Test concurrent operations
#[test]
fn test_concurrent_operations() {
    let manager = Arc::new(TransactionManager::new());
    let success_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    
    // Run multiple threads creating transactions
    let mut handles = vec![];
    
    for i in 0..5 {
        let manager_clone = Arc::clone(&manager);
        let success_count_clone = Arc::clone(&success_count);
        
        let handle = std::thread::spawn(move || {
            match manager_clone.begin() {
                Ok(tx_id) => {
                    // Execute operations
                    let execute_result = manager_clone.execute(tx_id, |tx| {
                        let record = Record {
                            id: Uuid::new_v4(),
                            text: format!("concurrent test {}", i),
                            embedding: vec![0.1 * i as f32; 768],
                            layer: Layer::Interact,
                            kind: "test".to_string(),
                            tags: vec![],
                            project: "test".to_string(),
                            session: "test".to_string(),
                            ts: Utc::now(),
                            score: 0.0,
                            access_count: 0,
                            last_access: Utc::now(),
                        };
                        tx.insert(record)?;
                        Ok(())
                    });
                    
                    if execute_result.is_ok() {
                        // Attempt commit
                        match manager_clone.prepare_commit(tx_id) {
                            Ok(_ops) => {
                                success_count_clone.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                            }
                            Err(_) => {} // Error in commit
                        }
                    }
                }
                Err(_) => {} // Error in begin
            }
        });
        
        handles.push(handle);
    }
    
    // Wait for all threads to complete
    for handle in handles {
        handle.join().unwrap();
    }
    
    let final_success_count = success_count.load(std::sync::atomic::Ordering::SeqCst);
    
    // Should have successful transactions in concurrent environment
    assert!(final_success_count > 0);
    
    // After all operations complete, should have no active transactions
    assert_eq!(manager.active_count(), 0);
    
    println!("✅ Concurrent operations test passed: {}/5 transactions succeeded", final_success_count);
}