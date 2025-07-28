---
license: apache-2.0
base_model:
- Qwen/Qwen3-Reranker-0.6B
library_name: transformers
pipeline_tag: text-generation
---
# Qwen3-Reranker-0.6B-ONNX

**How to use**
```python
from transformers import AutoTokenizer
import onnxruntime as ort
import numpy as np
import torch
from typing import List


class Qwen3RerankerONNX:
    def __init__(
        self,
        model_path: str = "zhiqing/Qwen3-Reranker-0.6B-ONNX/model.onnx",
        tokenizer_dir: str = "zhiqing/Qwen3-Reranker-0.6B-ONNX",
        providers: List[str] = ("CUDAExecutionProvider", "CPUExecutionProvider"),
        max_length: int = 2048,
    ):
        self.tokenizer = AutoTokenizer.from_pretrained(tokenizer_dir, padding_side="left")
        self.prefix = "<|im_start|>system\nJudge whether the Document meets the requirements based on the Query and the Instruct provided. Note that the answer can only be "yes" or "no".'<|im_end|>\n<|im_start|>user\n"
        self.suffix = "<|im_end|>\n<|im_start|>assistant\n<think>\n\n</think>\n\n"
        self.prefix_tokens = self.tokenizer.encode(self.prefix, add_special_tokens=False)
        self.suffix_tokens = self.tokenizer.encode(self.suffix, add_special_tokens=False)
        self.default_instruction = (
            "Given a web search query, retrieve relevant passages that answer the query"
        )
        self.max_length = max_length
        self.token_false_id = self.tokenizer.convert_tokens_to_ids("no")
        self.token_true_id = self.tokenizer.convert_tokens_to_ids("yes")
        self.session = ort.InferenceSession(model_path, providers=list(providers))
        self.output_name = "logits"

    def _format_instruction(self, instruction: str, query: str, doc: str) -> str:
        inst = instruction if instruction is not None else self.default_instruction
        return f"<Instruct>: {inst}\n<Query>: {query}\n<Document>: {doc}"

    def _tokenize(self, pairs: List[str]):
        encoded = self.tokenizer(
            [self.prefix + s + self.suffix for s in pairs],
            padding=True,
            truncation="longest_first",
            max_length=self.max_length - len(self.prefix_tokens) - len(self.suffix_tokens),
            add_special_tokens=False,
            return_tensors="np",
        )
        input_ids = encoded["input_ids"].astype(np.int64)
        attention_mask = encoded["attention_mask"].astype(np.int64)
        seq_len = input_ids.shape[1]
        position_ids = (
            np.arange(seq_len, dtype=np.int64)[None, :].repeat(input_ids.shape[0], axis=0)
        )
        return input_ids, attention_mask, position_ids

    def infer(
        self,
        queries: List[str],
        documents: List[str],
        instruction: str = None,
    ):
        if len(queries) == 1 and len(documents) > 1:
            queries = [queries[0]] * len(documents)
        elif len(queries) != len(documents):
            raise ValueError("The number of queries must be 1 or equal to the number of documents.")
        pairs = [
            self._format_instruction(instruction, q, d) for q, d in zip(queries, documents)
        ]
        input_ids, attention_mask, position_ids = self._tokenize(pairs)
        ort_inputs = {
            "input_ids": input_ids,
            "attention_mask": attention_mask,
            "position_ids": position_ids,
        }
        logits_np = self.session.run([self.output_name], ort_inputs)[0]
        last_token_logits = torch.from_numpy(logits_np[:, -1, :]).float()
        false_logits = last_token_logits[:, self.token_false_id]
        true_logits = last_token_logits[:, self.token_true_id]
        probs = torch.softmax(torch.stack([false_logits, true_logits], dim=1), dim=1)
        scores_no = probs[:, 0].tolist()
        scores_yes = probs[:, 1].tolist()
        return scores_yes, scores_no
```

<p align="center">
    <img src="https://qianwen-res.oss-accelerate-overseas.aliyuncs.com/logo_qwen3.png" width="400"/>
<p>

## Highlights

The Qwen3 Embedding model series is the latest proprietary model of the Qwen family, specifically designed for text embedding and ranking tasks. Building upon the dense foundational models of the Qwen3 series, it provides a comprehensive range of text embeddings and reranking models in various sizes (0.6B, 4B, and 8B). This series inherits the exceptional multilingual capabilities, long-text understanding, and reasoning skills of its foundational model. The Qwen3 Embedding series represents significant advancements in multiple text embedding and ranking tasks, including text retrieval, code retrieval, text classification, text clustering, and bitext mining.

**Exceptional Versatility**: The embedding model has achieved state-of-the-art performance across a wide range of downstream application evaluations. The 8B size embedding model ranks No.1 in the MTEB multilingual leaderboard (as of June 5, 2025, score 70.58), while the reranking model excels in various text retrieval scenarios.

**Comprehensive Flexibility**: The Qwen3 Embedding series offers a full spectrum of sizes (from 0.6B to 8B) for both embedding and reranking models, catering to diverse use cases that prioritize efficiency and effectiveness. Developers can seamlessly combine these two modules. Additionally, the embedding model allows for flexible vector definitions across all dimensions, and both embedding and reranking models support user-defined instructions to enhance performance for specific tasks, languages, or scenarios.

**Multilingual Capability**: The Qwen3 Embedding series offer support for over 100 languages, thanks to the multilingual capabilites of Qwen3 models. This includes various programming languages, and provides robust multilingual, cross-lingual, and code retrieval capabilities.

## Model Overview

**Qwen3-Reranker-0.6B** has the following features:

- Model Type: Text Reranking
- Supported Languages: 100+ Languages
- Number of Paramaters: 0.6B
- Context Length: 32k

For more details, including benchmark evaluation, hardware requirements, and inference performance, please refer to our [blog](https://qwenlm.github.io/blog/qwen3-embedding/), [GitHub](https://github.com/QwenLM/Qwen3-Embedding).

## Qwen3 Embedding Series Model list

| Model Type       | Models               | Size | Layers | Sequence Length | Embedding Dimension | MRL Support | Instruction Aware |
|------------------|----------------------|------|--------|-----------------|---------------------|-------------|----------------|
| Text Embedding   | [Qwen3-Embedding-0.6B](https://huggingface.co/Qwen/Qwen3-Embedding-0.6B) | 0.6B | 28     | 32K             | 1024                | Yes         | Yes            |
| Text Embedding   | [Qwen3-Embedding-4B](https://huggingface.co/Qwen/Qwen3-Embedding-4B)   | 4B   | 36     | 32K             | 2560                | Yes         | Yes            |
| Text Embedding   | [Qwen3-Embedding-8B](https://huggingface.co/Qwen/Qwen3-Embedding-8B)   | 8B   | 36     | 32K             | 4096                | Yes         | Yes            |
| Text Reranking   | [Qwen3-Reranker-0.6B](https://huggingface.co/Qwen/Qwen3-Reranker-0.6B) | 0.6B | 28     | 32K             | -                   | -           | Yes            |
| Text Reranking   | [Qwen3-Reranker-4B](https://huggingface.co/Qwen/Qwen3-Reranker-4B)   | 4B   | 36     | 32K             | -                   | -           | Yes            |
| Text Reranking   | [Qwen3-Reranker-8B](https://huggingface.co/Qwen/Qwen3-Reranker-8B)   | 8B   | 36     | 32K             | -                   | -           | Yes            |

> **Note**:
> - `MRL Support` indicates whether the embedding model supports custom dimensions for the final embedding. 
> - `Instruction Aware` notes whether the embedding or reranking model supports customizing the input instruction according to different tasks.
> - Our evaluation indicates that, for most downstream tasks, using instructions (instruct) typically yields an improvement of 1% to 5% compared to not using them. Therefore, we recommend that developers create tailored instructions specific to their tasks and scenarios. In multilingual contexts, we also advise users to write their instructions in English, as most instructions utilized during the model training process were originally written in English.


## Usage

With Transformers versions earlier than 4.51.0, you may encounter the following error:
```
KeyError: 'qwen3'
```


📌 **Tip**: We recommend that developers customize the `instruct` according to their specific scenarios, tasks, and languages. Our tests have shown that in most retrieval scenarios, not using an `instruct` on the query side can lead to a drop in retrieval performance by approximately 1% to 5%.

## Evaluation

| Model                              | Param  | MTEB-R  | CMTEB-R | MMTEB-R | MLDR   | MTEB-Code | FollowIR |
|------------------------------------|--------|---------|---------|---------|--------|-----------|----------|
| **Qwen3-Embedding-0.6B**               | 0.6B   | 61.82   | 71.02   | 64.64   | 50.26  | 75.41     | 5.09     |
| Jina-multilingual-reranker-v2-base | 0.3B   | 58.22   | 63.37   | 63.73   | 39.66  | 58.98     | -0.68    |
| gte-multilingual-reranker-base                      | 0.3B   | 59.51   | 74.08   | 59.44   | 66.33  | 54.18     | -1.64    |
| BGE-reranker-v2-m3                 | 0.6B   | 57.03   | 72.16   | 58.36   | 59.51  | 41.38     | -0.01    |
| **Qwen3-Reranker-0.6B**                | 0.6B   | 65.80   | 71.31   | 66.36   | 67.28  | 73.42     | 5.41     |
| **Qwen3-Reranker-4B**                  | 1.7B   | **69.76** | 75.94   | 72.74   | 69.97  | 81.20     | **14.84** |
| **Qwen3-Reranker-8B**                  | 8B     | 69.02   | **77.45** | **72.94** | **70.19** | **81.22** | 8.05     |

> **Note**:  
> - Evaluation results for reranking models. We use the retrieval subsets of MTEB(eng, v2), MTEB(cmn, v1), MMTEB and MTEB (Code), which are MTEB-R, CMTEB-R, MMTEB-R and MTEB-Code.
> - All scores are our runs based on the top-100 candidates retrieved by dense embedding model [Qwen3-Embedding-0.6B](https://huggingface.co/Qwen/Qwen3-Embedding-0.6B).

## Citation
If you find our work helpful, feel free to give us a cite.

```
@misc{qwen3-embedding,
    title  = {Qwen3-Embedding},
    url    = {https://qwenlm.github.io/blog/qwen3/},
    author = {Qwen Team},
    month  = {May},
    year   = {2025}
}
```