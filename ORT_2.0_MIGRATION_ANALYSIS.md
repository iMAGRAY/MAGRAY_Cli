# ORT 2.0 Migration Analysis for MAGRAY CLI

## Executive Summary

MAGRAY CLI currently uses ORT (ONNX Runtime) 1.16 for its memory system's semantic layer. Migrating to ORT 2.0.0-rc.10 would require significant code changes due to breaking API changes. This document provides a comprehensive analysis of the migration requirements and potential issues.

## Current State

### Version Information
- **Current ORT Version**: 1.16.3
- **Target ORT Version**: 2.0.0-rc.10
- **ONNX Runtime Version**: 1.22 (in ORT 2.0)

### Current Usage
The memory crate uses ORT for:
1. Text embeddings via Qwen3-Embedding-0.6B-ONNX model
2. Document reranking via Qwen3-Reranker-0.6B-ONNX model
3. Semantic search capabilities in the M4 memory layer

### Architecture
- `ort_backend.rs`: Abstraction layer for ORT versions
- `onnx_models.rs`: Original implementation (for ORT 2.0, unused)
- `onnx_models_v1.rs`: Current implementation for ORT 1.16
- `onnx_models_simplified.rs`: Mock implementation for development

## Breaking Changes in ORT 2.0

### 1. Session Builder API Changes
**ORT 1.16:**
```rust
let environment = Environment::builder()
    .with_name("qwen3_embedding")
    .build()?;

let session = SessionBuilder::new(&environment)?
    .with_optimization_level(GraphOptimizationLevel::Level3)?
    .with_intra_threads(4)?
    .with_model_from_file(&model_file)?;
```

**ORT 2.0:**
```rust
let session = Session::builder()?
    .with_optimization_level(GraphOptimizationLevel::Level3)?
    .with_intra_threads(4)?
    .commit_from_file(&model_file)?;
```

### 2. Input System Changes
**ORT 1.16:**
```rust
let outputs = session.run(vec![
    Value::from_array(([1, target_length], input_ids_i64))?,
    Value::from_array(([1, target_length], attention_mask_i64))?
])?;
```

**ORT 2.0:**
```rust
let outputs = session.run(ort::inputs![
    "input_ids" => ndarray::arr2(&[input_ids_i64]),
    "attention_mask" => ndarray::arr2(&[attention_mask_i64])
]?)?;
```

### 3. Tensor Extraction Changes
**ORT 1.16:**
```rust
let output_tensor = outputs[0].try_extract::<f32>()?.view();
```

**ORT 2.0:**
```rust
let output_tensor = outputs[0].extract_tensor::<f32>().view();
// No try_ prefix, infallible extraction for known types
```

### 4. Value Creation API
**ORT 1.16:**
```rust
Value::from_array(([batch_size, seq_len], data))?
```

**ORT 2.0:**
```rust
// Direct ndarray usage with inputs! macro
ort::inputs![ndarray::arr2(&data)]?
```

## Windows-Specific Issues

### Runtime Library Mismatch
The project previously encountered a linker error:
```
LNK2038: mismatch detected for 'RuntimeLibrary': 
value 'MT_StaticRelease' doesn't match value 'MD_DynamicRelease'
```

**Solution Applied**: Added `load-dynamic` feature to ORT in Cargo.toml:
```toml
ort = { version = "1.16", features = ["download-binaries", "load-dynamic"] }
```

This issue may resurface with ORT 2.0 and require similar handling.

## Dependencies Analysis

### Current Dependencies
- `ort = "1.16"` - Stable, well-tested
- `tokenizers = "0.21"` - Compatible with both ORT versions
- `ndarray = "0.15"` - May need update for ORT 2.0's improved ndarray integration

### ORT 2.0 Dependencies
- Requires ONNX Runtime 1.22
- Better ndarray integration
- Improved CUDA support
- New allocator API for device memory

## Migration Path

### Phase 1: Update Abstraction Layer
1. Extend `OrtBackend` trait to support both versions
2. Create `OrtV2Backend` implementation
3. Update feature flags for version selection

### Phase 2: Code Changes
1. Update session creation code
2. Replace all input construction with `inputs!` macro
3. Update tensor extraction to use new infallible API
4. Test with both CPU and GPU execution providers

### Phase 3: Testing
1. Unit tests for both backends
2. Performance comparison
3. Memory usage analysis
4. Windows-specific testing

## Risk Assessment

### High Risk
1. **API Instability**: 2.0.0-rc.10 is a release candidate, not stable
2. **Production Impact**: Breaking changes could affect inference reliability
3. **Windows Compatibility**: Previous linker issues may resurface

### Medium Risk
1. **Performance Changes**: New API may have different performance characteristics
2. **Memory Management**: New allocator API could affect memory usage patterns
3. **Feature Parity**: Some v1.16 features might not have direct equivalents

### Low Risk
1. **Model Compatibility**: ONNX models should work with both versions
2. **Tokenizer Integration**: External dependency, unaffected by ORT version

## Recommendations

### Short Term (Current Approach)
1. **Stay on ORT 1.16**: It's stable, tested, and working
2. **Maintain Abstraction**: Keep `ort_backend.rs` for future flexibility
3. **Monitor ORT 2.0**: Wait for stable release before migration

### Long Term
1. **Plan Migration**: When ORT 2.0 becomes stable
2. **Parallel Support**: Support both versions via feature flags
3. **Gradual Rollout**: Test extensively before full migration

### Implementation Strategy
```rust
// Cargo.toml
[features]
default = ["ort-v1"]
ort-v1 = ["ort1"]
ort-v2 = ["ort2"]

[dependencies]
ort1 = { package = "ort", version = "1.16", optional = true }
ort2 = { package = "ort", version = "2.0", optional = true }
```

## Conclusion

While ORT 2.0 offers improved APIs and better performance, the current implementation using ORT 1.16 is stable and functional. The breaking changes are significant enough that migration should be postponed until:

1. ORT 2.0 reaches stable release
2. There's a clear performance or feature benefit
3. Adequate testing resources are available
4. The abstraction layer can support both versions simultaneously

The current architecture with `ort_backend.rs` provides a good foundation for future migration when the time is right.