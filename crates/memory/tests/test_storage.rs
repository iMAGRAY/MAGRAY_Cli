use memory::storage::*;
use memory::types::*;
use std::sync::Arc;
use tempfile::TempDir;

#[test]
fn test_layer_storage_creation() {
    let temp_dir = TempDir::new().unwrap();
    let storage = LayerStorage::new(temp_dir.path(), Layer::Interact);
    
    assert!(storage.is_ok());
    
    let storage = storage.unwrap();
    assert_eq!(storage.layer(), &Layer::Interact);
    assert_eq!(storage.record_count(), 0);
    assert!(storage.is_empty());
}

#[test]
fn test_layer_storage_add_record() {
    let temp_dir = TempDir::new().unwrap();
    let mut storage = LayerStorage::new(temp_dir.path(), Layer::Interact).unwrap();
    
    let record = MemoryRecord {
        id: "test_id".to_string(),
        content: "test content".to_string(),
        embedding: vec![0.1, 0.2, 0.3],
        metadata: MemoryMetadata {
            layer: Layer::Interact,
            timestamp: chrono::Utc::now(),
            tags: vec!["test".to_string()],
            source: Some("test_source".to_string()),
            importance: 0.5,
            access_count: 0,
            last_accessed: chrono::Utc::now(),
        },
    };
    
    let result = storage.add_record(record.clone());
    assert!(result.is_ok());
    
    assert_eq!(storage.record_count(), 1);
    assert!(!storage.is_empty());
    
    let retrieved = storage.get_record(&record.id);
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap().id, record.id);
}

#[test]
fn test_layer_storage_get_nonexistent_record() {
    let temp_dir = TempDir::new().unwrap();
    let storage = LayerStorage::new(temp_dir.path(), Layer::Interact).unwrap();
    
    let result = storage.get_record("nonexistent_id");
    assert!(result.is_none());
}

#[test]
fn test_layer_storage_remove_record() {
    let temp_dir = TempDir::new().unwrap();
    let mut storage = LayerStorage::new(temp_dir.path(), Layer::Interact).unwrap();
    
    let record = MemoryRecord {
        id: "test_id".to_string(),
        content: "test content".to_string(),
        embedding: vec![0.1, 0.2, 0.3],
        metadata: MemoryMetadata {
            layer: Layer::Interact,
            timestamp: chrono::Utc::now(),
            tags: vec!["test".to_string()],
            source: Some("test_source".to_string()),
            importance: 0.5,
            access_count: 0,
            last_accessed: chrono::Utc::now(),
        },
    };
    
    storage.add_record(record.clone()).unwrap();
    assert_eq!(storage.record_count(), 1);
    
    let removed = storage.remove_record(&record.id);
    assert!(removed.is_some());
    assert_eq!(removed.unwrap().id, record.id);
    
    assert_eq!(storage.record_count(), 0);
    assert!(storage.is_empty());
}

#[test]
fn test_layer_storage_list_records() {
    let temp_dir = TempDir::new().unwrap();
    let mut storage = LayerStorage::new(temp_dir.path(), Layer::Interact).unwrap();
    
    // Add multiple records
    for i in 0..5 {
        let record = MemoryRecord {
            id: format!("test_id_{}", i),
            content: format!("test content {}", i),
            embedding: vec![i as f32 * 0.1, i as f32 * 0.2],
            metadata: MemoryMetadata {
                layer: Layer::Interact,
                timestamp: chrono::Utc::now(),
                tags: vec![format!("tag_{}", i)],
                source: Some(format!("source_{}", i)),
                importance: i as f32 * 0.2,
                access_count: i,
                last_accessed: chrono::Utc::now(),
            },
        };
        storage.add_record(record).unwrap();
    }
    
    let records = storage.list_records(10);
    assert_eq!(records.len(), 5);
    
    let limited_records = storage.list_records(3);
    assert_eq!(limited_records.len(), 3);
}

#[test]
fn test_layer_storage_search_by_tags() {
    let temp_dir = TempDir::new().unwrap();
    let mut storage = LayerStorage::new(temp_dir.path(), Layer::Interact).unwrap();
    
    let record1 = MemoryRecord {
        id: "record1".to_string(),
        content: "content1".to_string(),
        embedding: vec![0.1, 0.2],
        metadata: MemoryMetadata {
            layer: Layer::Interact,
            timestamp: chrono::Utc::now(),
            tags: vec!["important".to_string(), "work".to_string()],
            source: None,
            importance: 0.8,
            access_count: 0,
            last_accessed: chrono::Utc::now(),
        },
    };
    
    let record2 = MemoryRecord {
        id: "record2".to_string(),
        content: "content2".to_string(),
        embedding: vec![0.3, 0.4],
        metadata: MemoryMetadata {
            layer: Layer::Interact,
            timestamp: chrono::Utc::now(),
            tags: vec!["personal".to_string(), "hobby".to_string()],
            source: None,
            importance: 0.3,
            access_count: 0,
            last_accessed: chrono::Utc::now(),
        },
    };
    
    storage.add_record(record1).unwrap();
    storage.add_record(record2).unwrap();
    
    let important_records = storage.search_by_tag("important");
    assert_eq!(important_records.len(), 1);
    assert_eq!(important_records[0].id, "record1");
    
    let work_records = storage.search_by_tag("work");
    assert_eq!(work_records.len(), 1);
    
    let nonexistent_records = storage.search_by_tag("nonexistent");
    assert_eq!(nonexistent_records.len(), 0);
}

#[test]
fn test_layer_storage_update_record() {
    let temp_dir = TempDir::new().unwrap();
    let mut storage = LayerStorage::new(temp_dir.path(), Layer::Interact).unwrap();
    
    let mut record = MemoryRecord {
        id: "test_id".to_string(),
        content: "original content".to_string(),
        embedding: vec![0.1, 0.2],
        metadata: MemoryMetadata {
            layer: Layer::Interact,
            timestamp: chrono::Utc::now(),
            tags: vec!["original".to_string()],
            source: None,
            importance: 0.5,
            access_count: 0,
            last_accessed: chrono::Utc::now(),
        },
    };
    
    storage.add_record(record.clone()).unwrap();
    
    // Update the record
    record.content = "updated content".to_string();
    record.metadata.tags = vec!["updated".to_string()];
    record.metadata.importance = 0.8;
    
    let result = storage.update_record(record.clone());
    assert!(result.is_ok());
    
    let retrieved = storage.get_record(&record.id).unwrap();
    assert_eq!(retrieved.content, "updated content");
    assert_eq!(retrieved.metadata.tags, vec!["updated".to_string()]);
    assert_eq!(retrieved.metadata.importance, 0.8);
}

#[test]
fn test_layer_storage_clear() {
    let temp_dir = TempDir::new().unwrap();
    let mut storage = LayerStorage::new(temp_dir.path(), Layer::Interact).unwrap();
    
    // Add some records
    for i in 0..3 {
        let record = MemoryRecord {
            id: format!("id_{}", i),
            content: format!("content_{}", i),
            embedding: vec![i as f32],
            metadata: MemoryMetadata {
                layer: Layer::Interact,
                timestamp: chrono::Utc::now(),
                tags: vec![],
                source: None,
                importance: 0.5,
                access_count: 0,
                last_accessed: chrono::Utc::now(),
            },
        };
        storage.add_record(record).unwrap();
    }
    
    assert_eq!(storage.record_count(), 3);
    
    storage.clear();
    
    assert_eq!(storage.record_count(), 0);
    assert!(storage.is_empty());
}

#[test]
fn test_layer_storage_persistence() {
    let temp_dir = TempDir::new().unwrap();
    let record_id = "persistent_record".to_string();
    
    // Create storage and add record
    {
        let mut storage = LayerStorage::new(temp_dir.path(), Layer::Insights).unwrap();
        
        let record = MemoryRecord {
            id: record_id.clone(),
            content: "persistent content".to_string(),
            embedding: vec![0.5, 0.6, 0.7],
            metadata: MemoryMetadata {
                layer: Layer::Insights,
                timestamp: chrono::Utc::now(),
                tags: vec!["persistent".to_string()],
                source: Some("test".to_string()),
                importance: 0.9,
                access_count: 5,
                last_accessed: chrono::Utc::now(),
            },
        };
        
        storage.add_record(record).unwrap();
        assert_eq!(storage.record_count(), 1);
    } // Storage dropped here
    
    // Create new storage instance pointing to same directory
    {
        let storage = LayerStorage::new(temp_dir.path(), Layer::Insights).unwrap();
        
        // Should load existing data
        assert_eq!(storage.record_count(), 1);
        
        let retrieved = storage.get_record(&record_id);
        assert!(retrieved.is_some());
        
        let record = retrieved.unwrap();
        assert_eq!(record.content, "persistent content");
        assert_eq!(record.metadata.importance, 0.9);
        assert_eq!(record.metadata.access_count, 5);
    }
}

#[test]
fn test_layer_storage_concurrent_access() {
    use std::thread;
    use std::sync::Arc;
    
    let temp_dir = TempDir::new().unwrap();
    let storage = Arc::new(std::sync::Mutex::new(
        LayerStorage::new(temp_dir.path(), Layer::Assets).unwrap()
    ));
    
    let mut handles = vec![];
    
    // Spawn multiple threads to add records concurrently
    for i in 0..5 {
        let storage_clone = Arc::clone(&storage);
        let handle = thread::spawn(move || {
            let record = MemoryRecord {
                id: format!("concurrent_record_{}", i),
                content: format!("content_{}", i),
                embedding: vec![i as f32 * 0.1],
                metadata: MemoryMetadata {
                    layer: Layer::Assets,
                    timestamp: chrono::Utc::now(),
                    tags: vec![format!("thread_{}", i)],
                    source: None,
                    importance: 0.5,
                    access_count: 0,
                    last_accessed: chrono::Utc::now(),
                },
            };
            
            let mut storage_guard = storage_clone.lock().unwrap();
            storage_guard.add_record(record).unwrap();
        });
        handles.push(handle);
    }
    
    // Wait for all threads
    for handle in handles {
        handle.join().unwrap();
    }
    
    // Verify all records were added
    let final_storage = storage.lock().unwrap();
    assert_eq!(final_storage.record_count(), 5);
}

#[test]
fn test_memory_record_serialization() {
    let record = MemoryRecord {
        id: "serialization_test".to_string(),
        content: "test content for serialization".to_string(),
        embedding: vec![0.1, 0.2, 0.3, 0.4],
        metadata: MemoryMetadata {
            layer: Layer::Interact,
            timestamp: chrono::Utc::now(),
            tags: vec!["serialization".to_string(), "test".to_string()],
            source: Some("unit_test".to_string()),
            importance: 0.75,
            access_count: 10,
            last_accessed: chrono::Utc::now(),
        },
    };
    
    // Test JSON serialization
    let json = serde_json::to_string(&record).unwrap();
    assert!(json.contains("serialization_test"));
    assert!(json.contains("test content for serialization"));
    
    // Test deserialization
    let deserialized: MemoryRecord = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.id, record.id);
    assert_eq!(deserialized.content, record.content);
    assert_eq!(deserialized.embedding, record.embedding);
    assert_eq!(deserialized.metadata.importance, record.metadata.importance);
}

#[test]
fn test_layer_storage_stats() {
    let temp_dir = TempDir::new().unwrap();
    let mut storage = LayerStorage::new(temp_dir.path(), Layer::Insights).unwrap();
    
    let stats = storage.get_stats();
    assert_eq!(stats.total_records, 0);
    assert_eq!(stats.total_size_bytes, 0);
    assert_eq!(stats.layer, Layer::Insights);
    
    // Add some records
    for i in 0..3 {
        let record = MemoryRecord {
            id: format!("stats_record_{}", i),
            content: format!("content for stats test {}", i),
            embedding: vec![i as f32; 10], // 10 dimensions
            metadata: MemoryMetadata {
                layer: Layer::Insights,
                timestamp: chrono::Utc::now(),
                tags: vec![format!("stats_tag_{}", i)],
                source: None,
                importance: 0.5,
                access_count: i,
                last_accessed: chrono::Utc::now(),
            },
        };
        storage.add_record(record).unwrap();
    }
    
    let updated_stats = storage.get_stats();
    assert_eq!(updated_stats.total_records, 3);
    assert!(updated_stats.total_size_bytes > 0);
    assert!(updated_stats.average_importance > 0.0);
    assert_eq!(updated_stats.layer, Layer::Insights);
}

#[test]
fn test_layer_storage_error_handling() {
    let temp_dir = TempDir::new().unwrap();
    let mut storage = LayerStorage::new(temp_dir.path(), Layer::Assets).unwrap();
    
    // Test adding record with wrong layer
    let wrong_layer_record = MemoryRecord {
        id: "wrong_layer".to_string(),
        content: "content".to_string(),
        embedding: vec![0.1],
        metadata: MemoryMetadata {
            layer: Layer::Interact, // Different from storage layer
            timestamp: chrono::Utc::now(),
            tags: vec![],
            source: None,
            importance: 0.5,
            access_count: 0,
            last_accessed: chrono::Utc::now(),
        },
    };
    
    let result = storage.add_record(wrong_layer_record);
    assert!(result.is_err());
    
    // Test duplicate ID
    let record1 = MemoryRecord {
        id: "duplicate_id".to_string(),
        content: "first content".to_string(),
        embedding: vec![0.1],
        metadata: MemoryMetadata {
            layer: Layer::Assets,
            timestamp: chrono::Utc::now(),
            tags: vec![],
            source: None,
            importance: 0.5,
            access_count: 0,
            last_accessed: chrono::Utc::now(),
        },
    };
    
    let record2 = MemoryRecord {
        id: "duplicate_id".to_string(),
        content: "second content".to_string(),
        embedding: vec![0.2],
        metadata: MemoryMetadata {
            layer: Layer::Assets,
            timestamp: chrono::Utc::now(),
            tags: vec![],
            source: None,
            importance: 0.7,
            access_count: 0,
            last_accessed: chrono::Utc::now(),
        },
    };
    
    storage.add_record(record1).unwrap();
    let duplicate_result = storage.add_record(record2);
    assert!(duplicate_result.is_err());
}

#[test]
fn test_memory_metadata_defaults() {
    let metadata = MemoryMetadata::default();
    
    assert_eq!(metadata.layer, Layer::Interact);
    assert_eq!(metadata.importance, 0.5);
    assert_eq!(metadata.access_count, 0);
    assert!(metadata.tags.is_empty());
    assert!(metadata.source.is_none());
}

#[test]
fn test_layer_enum_ordering() {
    assert!(Layer::Assets > Layer::Insights);
    assert!(Layer::Insights > Layer::Interact);
    assert!(Layer::Assets > Layer::Interact);
    
    // Test equality
    assert_eq!(Layer::Interact, Layer::Interact);
    assert_eq!(Layer::Insights, Layer::Insights);
    assert_eq!(Layer::Assets, Layer::Assets);
}

#[test]
fn test_layer_storage_backup_and_restore() {
    let temp_dir = TempDir::new().unwrap();
    let backup_dir = TempDir::new().unwrap();
    
    let mut storage = LayerStorage::new(temp_dir.path(), Layer::Insights).unwrap();
    
    // Add test data
    let record = MemoryRecord {
        id: "backup_test".to_string(),
        content: "content to backup".to_string(),
        embedding: vec![0.1, 0.2, 0.3],
        metadata: MemoryMetadata {
            layer: Layer::Insights,
            timestamp: chrono::Utc::now(),
            tags: vec!["backup".to_string()],
            source: Some("test".to_string()),
            importance: 0.8,
            access_count: 3,
            last_accessed: chrono::Utc::now(),
        },
    };
    
    storage.add_record(record.clone()).unwrap();
    
    // Create backup
    let backup_result = storage.create_backup(backup_dir.path());
    assert!(backup_result.is_ok());
    
    // Clear original storage
    storage.clear();
    assert_eq!(storage.record_count(), 0);
    
    // Restore from backup
    let restore_result = storage.restore_from_backup(backup_dir.path());
    assert!(restore_result.is_ok());
    
    // Verify restoration
    assert_eq!(storage.record_count(), 1);
    let restored_record = storage.get_record(&record.id);
    assert!(restored_record.is_some());
    assert_eq!(restored_record.unwrap().content, record.content);
}