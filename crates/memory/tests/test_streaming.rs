use memory::streaming::*;
use memory::types::*;
use tokio::time::{timeout, Duration};
use futures::StreamExt;

#[tokio::test]
async fn test_memory_stream_creation() {
    let stream = MemoryStream::new(Layer::Interact, 100);
    
    assert_eq!(stream.layer(), &Layer::Interact);
    assert_eq!(stream.buffer_size(), 100);
    assert!(stream.is_empty());
    assert_eq!(stream.pending_count(), 0);
}

#[tokio::test]
async fn test_memory_stream_add_record() {
    let mut stream = MemoryStream::new(Layer::Interact, 10);
    
    let record = MemoryRecord {
        id: "stream_test".to_string(),
        content: "streaming test content".to_string(),
        embedding: vec![0.1, 0.2, 0.3],
        metadata: MemoryMetadata {
            layer: Layer::Interact,
            timestamp: chrono::Utc::now(),
            tags: vec!["streaming".to_string()],
            source: Some("test".to_string()),
            importance: 0.7,
            access_count: 0,
            last_accessed: chrono::Utc::now(),
        },
    };
    
    let result = stream.add_record(record.clone()).await;
    assert!(result.is_ok());
    
    assert!(!stream.is_empty());
    assert_eq!(stream.pending_count(), 1);
}

#[tokio::test]
async fn test_memory_stream_flush() {
    let mut stream = MemoryStream::new(Layer::Insights, 5);
    
    // Add multiple records
    for i in 0..3 {
        let record = MemoryRecord {
            id: format!("flush_test_{}", i),
            content: format!("content {}", i),
            embedding: vec![i as f32 * 0.1],
            metadata: MemoryMetadata {
                layer: Layer::Insights,
                timestamp: chrono::Utc::now(),
                tags: vec![format!("tag_{}", i)],
                source: None,
                importance: 0.5,
                access_count: 0,
                last_accessed: chrono::Utc::now(),
            },
        };
        stream.add_record(record).await.unwrap();
    }
    
    assert_eq!(stream.pending_count(), 3);
    
    // Flush the stream
    let flushed_records = stream.flush().await;
    assert!(flushed_records.is_ok());
    
    let records = flushed_records.unwrap();
    assert_eq!(records.len(), 3);
    assert_eq!(stream.pending_count(), 0);
    assert!(stream.is_empty());
}

#[tokio::test]
async fn test_memory_stream_auto_flush() {
    let mut stream = MemoryStream::new(Layer::Assets, 2); // Small buffer
    
    // Add records up to buffer limit
    stream.add_record(create_test_record("auto1", Layer::Assets)).await.unwrap();
    stream.add_record(create_test_record("auto2", Layer::Assets)).await.unwrap();
    
    assert_eq!(stream.pending_count(), 2);
    
    // Adding one more should trigger auto-flush
    let result = stream.add_record(create_test_record("auto3", Layer::Assets)).await;
    assert!(result.is_ok());
    
    // Buffer should be cleared after auto-flush
    assert!(stream.pending_count() <= 1); // Only the latest record should remain
}

#[tokio::test]
async fn test_streaming_processor_creation() {
    let processor = StreamingProcessor::new(100);
    
    assert_eq!(processor.buffer_size(), 100);
    assert_eq!(processor.active_streams(), 0);
    assert!(processor.get_stats().is_empty());
}

#[tokio::test]
async fn test_streaming_processor_add_stream() {
    let mut processor = StreamingProcessor::new(50);
    
    let stream_id = processor.add_stream(Layer::Interact).await;
    assert!(!stream_id.is_empty());
    
    assert_eq!(processor.active_streams(), 1);
    
    let stats = processor.get_stats();
    assert_eq!(stats.len(), 1);
    assert!(stats.contains_key(&stream_id));
}

#[tokio::test]
async fn test_streaming_processor_remove_stream() {
    let mut processor = StreamingProcessor::new(50);
    
    let stream_id = processor.add_stream(Layer::Insights).await;
    assert_eq!(processor.active_streams(), 1);
    
    let removed = processor.remove_stream(&stream_id).await;
    assert!(removed.is_ok());
    
    assert_eq!(processor.active_streams(), 0);
}

#[tokio::test]
async fn test_streaming_processor_process_record() {
    let mut processor = StreamingProcessor::new(100);
    
    let stream_id = processor.add_stream(Layer::Assets).await;
    let record = create_test_record("processor_test", Layer::Assets);
    
    let result = processor.process_record(&stream_id, record).await;
    assert!(result.is_ok());
    
    let stats = processor.get_stats();
    let stream_stats = stats.get(&stream_id).unwrap();
    assert_eq!(stream_stats.pending_records, 1);
}

#[tokio::test]
async fn test_streaming_processor_flush_stream() {
    let mut processor = StreamingProcessor::new(100);
    
    let stream_id = processor.add_stream(Layer::Interact).await;
    
    // Add multiple records
    for i in 0..5 {
        let record = create_test_record(&format!("flush_test_{}", i), Layer::Interact);
        processor.process_record(&stream_id, record).await.unwrap();
    }
    
    let result = processor.flush_stream(&stream_id).await;
    assert!(result.is_ok());
    
    let flushed_records = result.unwrap();
    assert_eq!(flushed_records.len(), 5);
    
    // Stream should be empty after flush
    let stats = processor.get_stats();
    let stream_stats = stats.get(&stream_id).unwrap();
    assert_eq!(stream_stats.pending_records, 0);
}

#[tokio::test]
async fn test_streaming_processor_flush_all() {
    let mut processor = StreamingProcessor::new(100);
    
    // Create multiple streams
    let stream1 = processor.add_stream(Layer::Interact).await;
    let stream2 = processor.add_stream(Layer::Insights).await;
    let stream3 = processor.add_stream(Layer::Assets).await;
    
    // Add records to each stream
    processor.process_record(&stream1, create_test_record("s1_r1", Layer::Interact)).await.unwrap();
    processor.process_record(&stream1, create_test_record("s1_r2", Layer::Interact)).await.unwrap();
    
    processor.process_record(&stream2, create_test_record("s2_r1", Layer::Insights)).await.unwrap();
    
    processor.process_record(&stream3, create_test_record("s3_r1", Layer::Assets)).await.unwrap();
    processor.process_record(&stream3, create_test_record("s3_r2", Layer::Assets)).await.unwrap();
    processor.process_record(&stream3, create_test_record("s3_r3", Layer::Assets)).await.unwrap();
    
    let result = processor.flush_all().await;
    assert!(result.is_ok());
    
    let all_records = result.unwrap();
    let total_records: usize = all_records.values().map(|v| v.len()).sum();
    assert_eq!(total_records, 6); // 2 + 1 + 3
    
    // All streams should be empty
    for stats in processor.get_stats().values() {
        assert_eq!(stats.pending_records, 0);
    }
}

#[tokio::test]
async fn test_real_time_memory_processor() {
    let mut processor = RealTimeMemoryProcessor::new(50, Duration::from_millis(100));
    
    assert!(!processor.is_running());
    
    let start_result = processor.start().await;
    assert!(start_result.is_ok());
    assert!(processor.is_running());
    
    let stop_result = processor.stop().await;
    assert!(stop_result.is_ok());
    assert!(!processor.is_running());
}

#[tokio::test]
async fn test_real_time_processor_with_records() {
    let mut processor = RealTimeMemoryProcessor::new(100, Duration::from_millis(50));
    
    processor.start().await.unwrap();
    
    // Add some records
    for i in 0..3 {
        let record = create_test_record(&format!("realtime_{}", i), Layer::Interact);
        let result = processor.queue_record(record).await;
        assert!(result.is_ok());
    }
    
    // Let processor run for a bit
    tokio::time::sleep(Duration::from_millis(200)).await;
    
    processor.stop().await.unwrap();
    
    let stats = processor.get_processing_stats();
    assert!(stats.total_processed >= 3);
}

#[tokio::test]
async fn test_streaming_with_backpressure() {
    let mut stream = MemoryStream::new(Layer::Insights, 2); // Very small buffer
    
    // Fill the buffer
    stream.add_record(create_test_record("bp1", Layer::Insights)).await.unwrap();
    stream.add_record(create_test_record("bp2", Layer::Insights)).await.unwrap();
    
    // Adding more should handle backpressure gracefully
    let start_time = std::time::Instant::now();
    stream.add_record(create_test_record("bp3", Layer::Insights)).await.unwrap();
    let elapsed = start_time.elapsed();
    
    // Should handle backpressure without blocking too long
    assert!(elapsed < Duration::from_secs(1));
}

#[tokio::test]
async fn test_streaming_error_handling() {
    let mut stream = MemoryStream::new(Layer::Assets, 10);
    
    // Test adding record with wrong layer
    let wrong_layer_record = MemoryRecord {
        id: "wrong_layer".to_string(),
        content: "content".to_string(),
        embedding: vec![0.1],
        metadata: MemoryMetadata {
            layer: Layer::Interact, // Wrong layer
            timestamp: chrono::Utc::now(),
            tags: vec![],
            source: None,
            importance: 0.5,
            access_count: 0,
            last_accessed: chrono::Utc::now(),
        },
    };
    
    let result = stream.add_record(wrong_layer_record).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_streaming_stats() {
    let mut processor = StreamingProcessor::new(100);
    
    let stream_id = processor.add_stream(Layer::Interact).await;
    
    // Add some records
    for i in 0..5 {
        let record = create_test_record(&format!("stats_{}", i), Layer::Interact);
        processor.process_record(&stream_id, record).await.unwrap();
    }
    
    let stats = processor.get_stats();
    let stream_stats = stats.get(&stream_id).unwrap();
    
    assert_eq!(stream_stats.stream_id, stream_id);
    assert_eq!(stream_stats.layer, Layer::Interact);
    assert_eq!(stream_stats.pending_records, 5);
    assert!(stream_stats.created_at <= chrono::Utc::now());
    assert!(stream_stats.last_activity >= stream_stats.created_at);
}

#[tokio::test]
async fn test_streaming_concurrent_access() {
    use std::sync::Arc;
    use tokio::sync::Mutex;
    
    let processor = Arc::new(Mutex::new(StreamingProcessor::new(200)));
    let mut handles = vec![];
    
    // Create multiple streams concurrently
    for i in 0..5 {
        let processor_clone = Arc::clone(&processor);
        let handle = tokio::spawn(async move {
            let mut proc = processor_clone.lock().await;
            let stream_id = proc.add_stream(Layer::Interact).await;
            
            // Add records to the stream
            for j in 0..3 {
                let record = create_test_record(&format!("concurrent_{}_{}", i, j), Layer::Interact);
                proc.process_record(&stream_id, record).await.unwrap();
            }
            
            stream_id
        });
        handles.push(handle);
    }
    
    let mut stream_ids = vec![];
    for handle in handles {
        let stream_id = handle.await.unwrap();
        stream_ids.push(stream_id);
    }
    
    // Verify all streams were created
    let final_processor = processor.lock().await;
    assert_eq!(final_processor.active_streams(), 5);
    
    // Verify all records were added
    let stats = final_processor.get_stats();
    let total_pending: usize = stats.values().map(|s| s.pending_records).sum();
    assert_eq!(total_pending, 15); // 5 streams * 3 records each
}

#[tokio::test] 
async fn test_streaming_timeout_handling() {
    let mut stream = MemoryStream::new(Layer::Assets, 100);
    
    // Test that operations don't hang indefinitely
    let record = create_test_record("timeout_test", Layer::Assets);
    
    let result = timeout(
        Duration::from_secs(1),
        stream.add_record(record)
    ).await;
    
    assert!(result.is_ok());
    assert!(result.unwrap().is_ok());
}

#[tokio::test]
async fn test_streaming_memory_usage() {
    let mut stream = MemoryStream::new(Layer::Insights, 1000);
    
    let initial_usage = stream.estimated_memory_usage();
    assert_eq!(initial_usage, 0);
    
    // Add some records
    for i in 0..10 {
        let record = create_test_record(&format!("memory_test_{}", i), Layer::Insights);
        stream.add_record(record).await.unwrap();
    }
    
    let usage_after_adding = stream.estimated_memory_usage();
    assert!(usage_after_adding > initial_usage);
    
    // Flush and check memory usage decreases
    stream.flush().await.unwrap();
    let usage_after_flush = stream.estimated_memory_usage();
    assert!(usage_after_flush < usage_after_adding);
}

#[tokio::test]
async fn test_stream_iterator() {
    let mut stream = MemoryStream::new(Layer::Interact, 100);
    
    // Add test records
    for i in 0..5 {
        let record = create_test_record(&format!("iter_test_{}", i), Layer::Interact);
        stream.add_record(record).await.unwrap();
    }
    
    let mut record_stream = stream.iter();
    let mut count = 0;
    
    while let Some(record) = record_stream.next().await {
        assert!(record.is_ok());
        count += 1;
        
        if count >= 5 {
            break;
        }
    }
    
    assert_eq!(count, 5);
}

// Helper function to create test records
fn create_test_record(id: &str, layer: Layer) -> MemoryRecord {
    MemoryRecord {
        id: id.to_string(),
        content: format!("Test content for {}", id),
        embedding: vec![0.1, 0.2, 0.3],
        metadata: MemoryMetadata {
            layer,
            timestamp: chrono::Utc::now(),
            tags: vec!["test".to_string()],
            source: Some("unit_test".to_string()),
            importance: 0.5,
            access_count: 0,
            last_accessed: chrono::Utc::now(),
        },
    }
}