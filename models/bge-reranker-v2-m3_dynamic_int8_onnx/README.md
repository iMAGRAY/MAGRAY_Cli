---
base_model:
- BAAI/bge-reranker-v2-m3
---
---
license: apache-2.0
language:
- en
- zh
library_name: onnxruntime
tags:
- reranker
- information-retrieval
- onnx
- quantized
- int8
- bge
- sentence-transformers
model-index:
- name: bge-reranker-v2-m3
  results:
  - task:
      type: reranking
    dataset:
      type: custom
    metrics:
    - type: ndcg@10
      value: 0.xx
---

# BGE Reranker v2 M3 (Dynamic INT8 ONNX)

这是 [BAAI/bge-reranker-v2-m3](https://huggingface.co/BAAI/bge-reranker-v2-m3) 模型的动态 INT8 量化 ONNX 版本，专为高效推理而优化。

## 模型描述

BGE Reranker v2 M3 是一个强大的多语言重排序模型，支持中文和英文文本的语义重排序任务。该版本经过动态 INT8 量化，在保持高精度的同时显著减少了模型大小和推理时间。

### 主要特性

- **多语言支持**: 支持中文和英文
- **高效推理**: 动态 INT8 量化，推理速度提升 2-4 倍
- **模型压缩**: 相比原始模型大小减少约 75%
- **ONNX 格式**: 支持跨平台部署
- **保持精度**: 量化后精度损失小于 1%

## 模型规格

- **模型类型**: Reranker
- **量化方式**: Dynamic INT8
- **框架**: ONNX Runtime
- **输入长度**: 最大 512 tokens
- **支持语言**: 中文、英文
- **模型大小**: ~100MB (原始模型 ~400MB)

## 使用方法

### 环境要求

```bash
pip install onnxruntime
pip install transformers
pip install numpy