//! Test file with various mocks, stubs and test doubles

use mockall::*;
use mockall::predicate::*;

/// Mock for memory service
#[automock]
pub trait MemoryService {
    fn store(&mut self, data: Vec<u8>) -> Result<(), String>;
    fn retrieve(&self, id: &str) -> Result<Vec<u8>, String>;
}

/// Stub implementation for database
pub struct StubDatabase {
    data: Vec<String>,
}

/// Fake implementation for testing
pub struct FakeMemoryService {
    storage: std::collections::HashMap<String, Vec<u8>>,
}

/// Dummy object for testing
pub struct DummyLogger;

/// Spy implementation to track calls
pub struct SpyService {
    call_count: usize,
    last_args: Option<String>,
}

impl MemoryService for FakeMemoryService {
    fn store(&mut self, data: Vec<u8>) -> Result<(), String> {
        self.storage.insert("test".to_string(), data);
        Ok(())
    }
    
    fn retrieve(&self, id: &str) -> Result<Vec<u8>, String> {
        self.storage.get(id).cloned().ok_or("Not found".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_mock_memory_service() {
        let mut mock = MockMemoryService::new();
        mock.expect_store()
            .times(1)
            .returning(|_| Ok(()));
        
        assert!(mock.store(vec![1, 2, 3]).is_ok());
    }
    
    #[test]
    fn test_stub_database() {
        let stub = StubDatabase { data: vec![] };
        // Test with stub
    }
    
    #[test]
    fn test_fake_implementation() {
        let mut fake = FakeMemoryService {
            storage: Default::default(),
        };
        fake.store(vec![1, 2, 3]).unwrap();
    }
}