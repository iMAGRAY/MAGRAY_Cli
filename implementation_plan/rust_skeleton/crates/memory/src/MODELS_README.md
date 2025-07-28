# ONNX Models Setup

This directory contains ONNX models for embedding and reranking, but the actual model files are too large for GitHub.

## Required Model Files

You need to download these files manually:

### Qwen3-Embedding-0.6B-ONNX/
- `model_fp16.onnx` (small, ~2MB)
- `model_fp16.onnx_data` (large, ~1.1GB) 

### Qwen3-Reranker-0.6B-ONNX/
- `model.onnx` (large, ~600MB)

## Download Instructions

1. **Option 1: HuggingFace Hub**
   ```bash
   # Install huggingface-hub
   pip install huggingface-hub
   
   # Download embedding model
   huggingface-cli download Alibaba-NLP/gte-Qwen2-1.5B-instruct --local-dir Qwen3-Embedding-0.6B-ONNX/
   
   # Download reranker model  
   huggingface-cli download Alibaba-NLP/gte-reranker-base --local-dir Qwen3-Reranker-0.6B-ONNX/
   ```

2. **Option 2: Manual Download**
   - Go to https://huggingface.co/Alibaba-NLP/gte-Qwen2-1.5B-instruct
   - Download the ONNX files to `Qwen3-Embedding-0.6B-ONNX/`
   - Go to https://huggingface.co/Alibaba-NLP/gte-reranker-base  
   - Download the ONNX files to `Qwen3-Reranker-0.6B-ONNX/`

3. **Option 3: Use Alternative Models**
   - You can use smaller models or different providers
   - Update the paths in `onnx_models.rs` accordingly

## Verification

After downloading, your directory structure should look like:
```
src/
├── Qwen3-Embedding-0.6B-ONNX/
│   ├── model_fp16.onnx
│   ├── model_fp16.onnx_data
│   ├── config.json
│   └── ... (other config files)
├── Qwen3-Reranker-0.6B-ONNX/
│   ├── model.onnx
│   ├── config.json
│   └── ... (other config files)
```

The Rust code will automatically detect and load these models when available.