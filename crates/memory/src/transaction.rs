use anyhow::{anyhow, Result};
use std::sync::Arc;
use std::collections::HashMap;
use uuid::Uuid;
use tracing::{debug, info, warn};
use parking_lot::Mutex;

use crate::types::{Layer, Record};

/// Операция в транзакции
#[derive(Debug, Clone)]
pub enum TransactionOp {
    Insert { record: Record },
    Update { layer: Layer, id: Uuid, record: Record },
    Delete { layer: Layer, id: Uuid },
    BatchInsert { records: Vec<Record> },
}

/// Статус транзакции
#[derive(Debug, Clone, PartialEq)]
pub enum TransactionStatus {
    Active,
    Committed,
    Aborted,
}

/// Транзакция для атомарных операций
pub struct Transaction {
    id: Uuid,
    operations: Vec<TransactionOp>,
    status: TransactionStatus,
    rollback_actions: Vec<RollbackAction>,
}

/// Действие для отката
#[derive(Debug, Clone)]
#[allow(dead_code)] // Может быть использовано в будущем
pub enum RollbackAction {
    #[allow(dead_code)]
    DeleteInserted { layer: Layer, id: Uuid },
    #[allow(dead_code)]
    RestoreDeleted { record: Record },
    #[allow(dead_code)]
    RestoreUpdated { record: Record },
}

impl Transaction {
    pub fn new() -> Self {
        Self {
            id: Uuid::new_v4(),
            operations: Vec::new(),
            status: TransactionStatus::Active,
            rollback_actions: Vec::new(),
        }
    }

    /// Добавить операцию вставки
    pub fn insert(&mut self, record: Record) -> Result<()> {
        if self.status != TransactionStatus::Active {
            return Err(anyhow!("Transaction is not active"));
        }
        
        self.operations.push(TransactionOp::Insert { record: record.clone() });
        self.rollback_actions.push(RollbackAction::DeleteInserted {
            layer: record.layer,
            id: record.id,
        });
        
        Ok(())
    }

    /// Добавить операцию обновления
    pub fn update(&mut self, layer: Layer, id: Uuid, new_record: Record) -> Result<()> {
        if self.status != TransactionStatus::Active {
            return Err(anyhow!("Transaction is not active"));
        }
        
        self.operations.push(TransactionOp::Update { 
            layer, 
            id, 
            record: new_record 
        });
        
        Ok(())
    }

    /// Добавить операцию удаления
    pub fn delete(&mut self, layer: Layer, id: Uuid) -> Result<()> {
        if self.status != TransactionStatus::Active {
            return Err(anyhow!("Transaction is not active"));
        }
        
        self.operations.push(TransactionOp::Delete { layer, id });
        
        Ok(())
    }

    /// Добавить пакетную вставку
    pub fn batch_insert(&mut self, records: Vec<Record>) -> Result<()> {
        if self.status != TransactionStatus::Active {
            return Err(anyhow!("Transaction is not active"));
        }
        
        for record in &records {
            self.rollback_actions.push(RollbackAction::DeleteInserted {
                layer: record.layer,
                id: record.id,
            });
        }
        
        self.operations.push(TransactionOp::BatchInsert { records });
        
        Ok(())
    }

    /// Получить операции для выполнения и зафиксировать транзакцию
    pub fn take_operations(&mut self) -> Result<Vec<TransactionOp>> {
        if self.status != TransactionStatus::Active {
            return Err(anyhow!("Transaction is not active"));
        }
        
        self.status = TransactionStatus::Committed;
        Ok(std::mem::take(&mut self.operations))
    }

    /// Отметить транзакцию как прерванную
    pub fn abort(&mut self) {
        self.status = TransactionStatus::Aborted;
        self.operations.clear();
        self.rollback_actions.clear();
    }

    /// Получить действия для отката
    pub fn rollback_actions(&self) -> &[RollbackAction] {
        &self.rollback_actions
    }

    /// Добавить действие для отката
    pub fn add_rollback_action(&mut self, action: RollbackAction) {
        self.rollback_actions.push(action);
    }
}

/// Менеджер транзакций
pub struct TransactionManager {
    active_transactions: Arc<Mutex<HashMap<Uuid, Transaction>>>,
}

impl TransactionManager {
    pub fn new() -> Self {
        Self {
            active_transactions: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Начать новую транзакцию
    pub fn begin(&self) -> Result<Uuid> {
        let transaction = Transaction::new();
        let id = transaction.id;
        
        let mut transactions = self.active_transactions.lock();
        transactions.insert(id, transaction);
        
        debug!("Started transaction {}", id);
        Ok(id)
    }

    /// Выполнить операцию в транзакции
    pub fn execute<F>(&self, tx_id: Uuid, operation: F) -> Result<()>
    where
        F: FnOnce(&mut Transaction) -> Result<()>,
    {
        let mut transactions = self.active_transactions.lock();
        let transaction = transactions.get_mut(&tx_id)
            .ok_or_else(|| anyhow!("Transaction {} not found", tx_id))?;
        
        operation(transaction)
    }

    /// Подготовить транзакцию к коммиту
    pub fn prepare_commit(&self, tx_id: Uuid) -> Result<Vec<TransactionOp>> {
        let mut transactions = self.active_transactions.lock();
        let mut transaction = transactions.remove(&tx_id)
            .ok_or_else(|| anyhow!("Transaction {} not found", tx_id))?;
        
        transaction.take_operations()
    }

    /// Откатить транзакцию
    pub fn rollback(&self, tx_id: Uuid) -> Result<()> {
        let mut transactions = self.active_transactions.lock();
        
        if let Some(mut transaction) = transactions.remove(&tx_id) {
            transaction.abort();
            info!("Rolled back transaction {}", tx_id);
        }
        
        Ok(())
    }

    /// Очистить старые неактивные транзакции
    pub fn cleanup_stale_transactions(&self, _timeout_secs: u64) {
        let transactions = self.active_transactions.lock();
        let stale_count = transactions.len();
        
        // В реальной реализации здесь была бы проверка по времени
        // Пока просто логируем
        if stale_count > 0 {
            warn!("Found {} potentially stale transactions", stale_count);
        }
    }

    /// Получить количество активных транзакций
    pub fn active_count(&self) -> usize {
        self.active_transactions.lock().len()
    }
}

/// RAII guard для автоматического отката при ошибке
pub struct TransactionGuard<'a> {
    manager: &'a TransactionManager,
    tx_id: Uuid,
    committed: bool,
}

impl<'a> TransactionGuard<'a> {
    pub fn new(manager: &'a TransactionManager) -> Result<Self> {
        let tx_id = manager.begin()?;
        Ok(Self {
            manager,
            tx_id,
            committed: false,
        })
    }

    pub fn tx_id(&self) -> Uuid {
        self.tx_id
    }

    pub fn commit(mut self) -> Result<Vec<TransactionOp>> {
        let ops = self.manager.prepare_commit(self.tx_id)?;
        self.committed = true;
        Ok(ops)
    }
}

impl<'a> Drop for TransactionGuard<'a> {
    fn drop(&mut self) {
        if !self.committed {
            // Автоматический откат при выходе из scope без коммита
            let _ = self.manager.rollback(self.tx_id);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Layer;

    #[test]
    fn test_transaction_basic() {
        let mut tx = Transaction::new();
        
        let record = Record {
            id: Uuid::new_v4(),
            text: "test".to_string(),
            embedding: vec![0.1; 1024],
            layer: Layer::Interact,
            ..Default::default()
        };
        
        tx.insert(record.clone()).unwrap();
        assert_eq!(tx.operations.len(), 1);
        assert_eq!(tx.rollback_actions.len(), 1);
        
        let ops = tx.take_operations().unwrap();
        assert_eq!(ops.len(), 1);
        assert_eq!(tx.status, TransactionStatus::Committed);
    }

    #[test]
    fn test_transaction_manager() {
        let manager = TransactionManager::new();
        
        // Begin transaction
        let tx_id = manager.begin().unwrap();
        assert_eq!(manager.active_count(), 1);
        
        // Execute operation
        manager.execute(tx_id, |tx| {
            let record = Record {
                id: Uuid::new_v4(),
                text: "test".to_string(),
                embedding: vec![0.1; 1024],
                layer: Layer::Interact,
                ..Default::default()
            };
            tx.insert(record)
        }).unwrap();
        
        // Commit
        let ops = manager.prepare_commit(tx_id).unwrap();
        assert_eq!(ops.len(), 1);
        assert_eq!(manager.active_count(), 0);
    }

    #[test]
    fn test_transaction_guard() {
        let manager = TransactionManager::new();
        
        // Test auto-rollback
        {
            let _guard = TransactionGuard::new(&manager).unwrap();
            assert_eq!(manager.active_count(), 1);
            // Guard dropped without commit
        }
        assert_eq!(manager.active_count(), 0);
        
        // Test commit
        {
            let guard = TransactionGuard::new(&manager).unwrap();
            let _ops = guard.commit().unwrap();
        }
        assert_eq!(manager.active_count(), 0);
    }
}