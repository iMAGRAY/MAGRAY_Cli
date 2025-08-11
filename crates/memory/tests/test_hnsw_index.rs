#![cfg(all(feature = "extended-tests", feature = "hnsw-index"))]

use memory::hnsw_index::{HnswConfig, VectorIndex};

#[test]
fn hnsw_add_and_search_orders_by_distance() {
    let mut cfg = HnswConfig::small_dataset();
    cfg.dimension = 8;
    cfg.max_elements = 1000;
    cfg.ef_search = 24;
    let index = VectorIndex::new(cfg).expect("config ok");

    // two clusters around unit vectors
    let a = vec![1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0];
    let b = vec![0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0];

    index.add("A".into(), a.clone()).unwrap();
    index.add("B".into(), b.clone()).unwrap();

    // query near A
    let q = vec![0.9, 0.1, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0];
    let res = index.search(&q, 2).expect("search ok");
    assert_eq!(res.len(), 2);
    let ids: Vec<String> = res.into_iter().map(|(id, _d)| id).collect();
    assert_eq!(ids[0], "A");
}

#[test]
fn hnsw_capacity_validation() {
    let mut cfg = HnswConfig::small_dataset();
    cfg.dimension = 4;
    cfg.max_elements = 1;
    let index = VectorIndex::new(cfg).unwrap();
    index.add("one".into(), vec![1.0, 0.0, 0.0, 0.0]).unwrap();
    let err = index
        .add("two".into(), vec![0.0, 1.0, 0.0, 0.0])
        .unwrap_err();
    assert!(err.to_string().contains("capacity"));
}

#[test]
fn hnsw_parallel_search_basic() {
    let mut cfg = HnswConfig::small_dataset();
    cfg.dimension = 4;
    cfg.max_elements = 100;
    cfg.use_parallel = true;
    let index = VectorIndex::new(cfg).unwrap();

    for i in 0..10 {
        let mut v = vec![0.0, 0.0, 0.0, 0.0];
        v[i % 4] = 1.0;
        index.add(format!("id{}", i), v).unwrap();
    }

    let queries = vec![
        vec![1.0, 0.0, 0.0, 0.0],
        vec![0.0, 1.0, 0.0, 0.0],
        vec![0.0, 0.0, 1.0, 0.0],
    ];
    let out = index.parallel_search(&queries, 1).unwrap();
    assert_eq!(out.len(), 3);
    for (i, r) in out.iter().enumerate() {
        assert_eq!(r.len(), 1);
        let id = &r[0].0; // (id, distance)
        assert!(id.contains(&format!("{}", i))); // nearest one-hot should match position
    }
}
