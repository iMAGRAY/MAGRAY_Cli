#![cfg(all(not(feature = "minimal")))]

// Keep only lightweight smoke and unified api tests in cpu default; others require feature flags

#[cfg(test)]
mod selected {
    // Always run critical smoke tests
    include!("./smoke_memory.rs");
}

// Heavier suites gated to persistence/vector/gpu features
#[cfg(all(not(feature = "minimal"), feature = "persistence"))]
mod persistence_suites {
    include!("./test_vector_store.rs");
    include!("./test_cache_migration.rs");
}

#[cfg(all(not(feature = "minimal"), feature = "rayon"))]
mod vector_suites {
    include!("./test_batch_optimized.rs");
}

#[cfg(all(not(feature = "minimal"), feature = "gpu-acceleration"))]
mod gpu_suites {
    include!("./test_gpu_batch_processor.rs");
}