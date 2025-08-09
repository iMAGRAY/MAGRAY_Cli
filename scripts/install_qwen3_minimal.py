#!/usr/bin/env python3
# -*- coding: utf-8 -*-

import argparse
import os
from pathlib import Path
from urllib.request import urlopen

EMB_ID = "Qwen/Qwen3-Embedding-0.6B"
RERANK_ID = "Qwen/Qwen3-Reranker-0.6B"
BASE = "https://huggingface.co/{repo}/resolve/main/{file}"

FILES = [
    "config.json",
    "tokenizer.json",
    "tokenizer_config.json",
    "vocab.json",
    "merges.txt",
    "special_tokens_map.json",
    "added_tokens.json",
    "chat_template.jinja",
]


def download(repo: str, filename: str, dest: Path):
    try:
        url = BASE.format(repo=repo, file=filename)
        with urlopen(url) as r, open(dest, "wb") as f:
            f.write(r.read())
        print(f"Downloaded {filename} -> {dest}")
    except Exception as e:
        print(f"Skip {filename}: {e}")


def ensure_dir(path: Path):
    path.mkdir(parents=True, exist_ok=True)


def touch_placeholder_onnx(path: Path):
    if not path.exists():
        with open(path, "wb") as f:
            f.write(b"ONNX_PLACEHOLDER")
        print(f"Created placeholder {path}")


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--models-dir", default="models")
    args = ap.parse_args()

    root = Path(args.models_dir)
    emb_dir = root / "qwen3emb"
    rer_dir = root / "qwen3_reranker"

    ensure_dir(emb_dir)
    ensure_dir(rer_dir)

    for repo, out_dir in ((EMB_ID, emb_dir), (RERANK_ID, rer_dir)):
        for fname in FILES:
            dest = out_dir / fname
            if not dest.exists():
                download(repo, fname, dest)
        touch_placeholder_onnx(out_dir / "model.onnx")

    print("\n✅ Qwen3 minimal models prepared:")
    print(f"  - {emb_dir}")
    print(f"  - {rer_dir}")


if __name__ == "__main__":
    main()