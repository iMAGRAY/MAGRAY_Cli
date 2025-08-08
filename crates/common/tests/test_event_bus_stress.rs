#![cfg(feature = "extended-tests")]

use common::event_bus::{EventBus, Topic};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

#[tokio::test(flavor = "multi_thread", worker_threads = 8)]
async fn event_bus_fanout_stress_no_loss() {
    let subscribers: usize = 1000;
    let messages: usize = 20;
    let bus: EventBus<u32> = EventBus::new(4096, std::time::Duration::from_millis(250));

    // Prepare counters per subscriber
    let counters: Vec<Arc<AtomicUsize>> = (0..subscribers)
        .map(|_| Arc::new(AtomicUsize::new(0)))
        .collect();

    // Spawn subscribers
    let mut handles = Vec::with_capacity(subscribers);
    for i in 0..subscribers {
        let mut rx = bus.subscribe(Topic("stress.topic")).await;
        let counter = counters[i].clone();
        handles.push(tokio::spawn(async move {
            while let Ok(_evt) = rx.recv().await {
                counter.fetch_add(1, Ordering::Relaxed);
                if counter.load(Ordering::Relaxed) >= messages {
                    break;
                }
            }
        }));
    }

    // Publish messages
    for n in 0..messages as u32 {
        bus.publish(Topic("stress.topic"), n).await;
    }

    // Wait until all subscribers have received all messages or timeout
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    loop {
        let done = counters
            .iter()
            .all(|c| c.load(Ordering::Relaxed) >= messages);
        if done { break; }
        if std::time::Instant::now() > deadline { break; }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }

    // Join tasks
    for h in handles { let _ = h.await; }

    // Verify no loss
    for (idx, c) in counters.iter().enumerate() {
        let v = c.load(Ordering::Relaxed);
        assert_eq!(v, messages, "subscriber {} missed {} msgs", idx, messages - v);
    }
}