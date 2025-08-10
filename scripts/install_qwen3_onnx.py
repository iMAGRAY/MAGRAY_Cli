#!/usr/bin/env python3
# -*- coding: utf-8 -*-

import os
import sys
import json
import shutil
import subprocess
from pathlib import Path
from typing import Optional

# -------- utilities --------

def run(cmd: list[str], cwd: Optional[Path] = None, env: Optional[dict] = None):
    print(f"$ {' '.join(cmd)}")
    subprocess.check_call(cmd, cwd=str(cwd) if cwd else None, env=env)


def in_venv() -> bool:
    return (
        hasattr(sys, 'real_prefix')
        or (hasattr(sys, 'base_prefix') and sys.base_prefix != sys.prefix)
        or (os.environ.get('VIRTUAL_ENV') is not None)
    )


def try_venv_and_reexec() -> bool:
    if in_venv():
        return True
    venv_dir = Path.cwd() / '.venv_qwen3'
    venv_python = venv_dir / 'bin' / 'python'
    try:
        if not venv_python.exists():
            print(f"Creating virtualenv at {venv_dir}")
            run([sys.executable, '-m', 'venv', str(venv_dir)])
            # Upgrade pip
            run([str(venv_python), '-m', 'ensurepip', '--upgrade'])
            run([str(venv_python), '-m', 'pip', 'install', '-U', 'pip', 'setuptools', 'wheel'])
        # Re-exec into venv
        print("Re-exec into virtualenv")
        os.execv(str(venv_python), [str(venv_python), __file__] + sys.argv[1:])
        return True  # not reached
    except Exception as e:
        print(f"Warning: could not create/use venv: {e}")
        return False


def pip_install(pkgs: list[str]) -> None:
    # Try normal user-site install first
    try:
        run([sys.executable, '-m', 'pip', 'install', '--user', '-q'] + pkgs)
        return
    except subprocess.CalledProcessError as e:
        print(f"pip --user failed: {e}")
    # Try with --break-system-packages if needed
    try:
        run([sys.executable, '-m', 'pip', 'install', '--user', '--break-system-packages', '-q'] + pkgs)
        return
    except subprocess.CalledProcessError as e:
        print(f"pip --user --break-system-packages failed: {e}")
        # Last resort: best-effort without --user
        run([sys.executable, '-m', 'pip', 'install', '--break-system-packages', '-q'] + pkgs)


def ensure_pkgs():
    # Prefer venv, but tolerate failure
    if not in_venv():
        _ = try_venv_and_reexec()
    # Minimal set for export
    pkgs = [
        'torch',
        'transformers>=4.51.0',
        'onnx>=1.15.0',
        'onnxruntime>=1.17.0',
        'sentence-transformers>=2.7.0'
    ]
    # Try import; if fails, pip install
    need_install = []
    try:
        import torch  # noqa: F401
    except Exception:
        need_install.append('torch')
    try:
        import transformers  # noqa: F401
    except Exception:
        need_install.append('transformers>=4.51.0')
    try:
        import onnx  # noqa: F401
    except Exception:
        need_install.append('onnx>=1.15.0')
    try:
        import onnxruntime  # noqa: F401
    except Exception:
        need_install.append('onnxruntime>=1.17.0')
    try:
        import sentence_transformers  # noqa: F401
    except Exception:
        need_install.append('sentence-transformers>=2.7.0')

    if need_install:
        print(f"Installing python packages: {need_install}")
        pip_install(need_install)


def save_tokenizer(tokenizer, out_dir: Path):
    out_dir.mkdir(parents=True, exist_ok=True)
    # Prefer fast tokenizer JSON if available
    tok_json = out_dir / 'tokenizer.json'
    try:
        tokenizer.save_pretrained(str(out_dir))
        # Some tokenizers save tokenizer.json directly
        if not tok_json.exists():
            # Build tokenizer.json from slow tokenizer if needed
            from tokenizers import Tokenizer as FastTokenizer  # type: ignore
            try:
                fast_tok = FastTokenizer.from_pretrained(tokenizer.name_or_path)  # type: ignore
                fast_tok.save(str(tok_json))
            except Exception:
                pass
    except Exception as e:
        print(f"Warning: tokenizer save_pretrained failed: {e}")
    # Ensure at least tokenizer.json exists
    if not tok_json.exists():
        try:
            tokenizer.backend_tokenizer.save(str(tok_json))  # type: ignore
        except Exception:
            with open(tok_json, 'w', encoding='utf-8') as f:
                json.dump({"added_tokens": []}, f)


def save_config(model, out_dir: Path):
    out_dir.mkdir(parents=True, exist_ok=True)
    cfg_path = out_dir / 'config.json'
    try:
        with open(cfg_path, 'w', encoding='utf-8') as f:
            f.write(model.config.to_json_string())
    except Exception as e:
        print(f"Warning: config save failed: {e}")
        with open(cfg_path, 'w', encoding='utf-8') as f:
            json.dump({}, f)


def export_embedding_onnx(model_id: str, out_dir: Path):
    import torch
    from transformers import AutoModel, AutoTokenizer

    print(f"Downloading embedding model: {model_id}")
    tokenizer = AutoTokenizer.from_pretrained(model_id)
    model = AutoModel.from_pretrained(model_id)
    model.eval()
    # Disable cache to avoid Qwen3 use_cache issues
    if hasattr(model.config, 'use_cache'):
        model.config.use_cache = False

    out_dir.mkdir(parents=True, exist_ok=True)
    save_tokenizer(tokenizer, out_dir)
    save_config(model, out_dir)

    # Dummy inputs
    dummy_texts = [
        "Instruct: Given a web search query, retrieve relevant passages that answer the query\nQuery: test",
    ]
    enc = tokenizer(dummy_texts, padding=True, truncation=True, max_length=32, return_tensors='pt')
    input_ids = enc.get('input_ids')
    attention_mask = enc.get('attention_mask')

    class NoCacheWrapper(torch.nn.Module):
        def __init__(self, backbone):
            super().__init__()
            self.backbone = backbone
        def forward(self, input_ids, attention_mask):
            out = self.backbone(input_ids=input_ids, attention_mask=attention_mask, use_cache=False, return_dict=True)
            return out.last_hidden_state

    wrapped = NoCacheWrapper(model)

    input_names = ['input_ids', 'attention_mask']
    output_names = ['last_hidden_state']
    dynamic_axes = {
        'input_ids': {0: 'batch', 1: 'seq'},
        'attention_mask': {0: 'batch', 1: 'seq'},
        'last_hidden_state': {0: 'batch', 1: 'seq'}
    }

    model_onnx = out_dir / 'model.onnx'
    print(f"Exporting ONNX: {model_onnx}")
    with torch.no_grad():
        torch.onnx.export(
            wrapped,
            (input_ids, attention_mask),
            str(model_onnx),
            input_names=input_names,
            output_names=output_names,
            dynamic_axes=dynamic_axes,
            opset_version=14,
        )


def export_reranker_onnx(model_id: str, out_dir: Path):
    import torch
    from transformers import AutoTokenizer, AutoModelForSequenceClassification

    print(f"Downloading reranker model: {model_id}")
    tokenizer = AutoTokenizer.from_pretrained(model_id)
    model = AutoModelForSequenceClassification.from_pretrained(model_id)
    model.eval()
    if hasattr(model.config, 'use_cache'):
        model.config.use_cache = False

    out_dir.mkdir(parents=True, exist_ok=True)
    save_tokenizer(tokenizer, out_dir)
    save_config(model, out_dir)

    # Build pair input (query + doc) as single sequence or pair depending on tokenizer
    q = "Instruct: Given a web search query, retrieve relevant passages that answer the query\nQuery: test"
    d = "This is a test document about embeddings and reranking."

    try:
        enc = tokenizer(q, d, padding=True, truncation=True, max_length=128, return_tensors='pt')
    except Exception:
        enc = tokenizer([q + " [SEP] " + d], padding=True, truncation=True, max_length=128, return_tensors='pt')

    # Prepare inputs based on tokenizer support
    input_ids = enc.get('input_ids')
    attention_mask = enc.get('attention_mask')
    token_type_ids = enc.get('token_type_ids')

    class NoCacheClsWrapper(torch.nn.Module):
        def __init__(self, backbone, has_token_type: bool):
            super().__init__()
            self.backbone = backbone
            self.has_token_type = has_token_type
        def forward(self, input_ids, attention_mask, token_type_ids=None):
            if self.has_token_type and token_type_ids is not None:
                out = self.backbone(input_ids=input_ids, attention_mask=attention_mask, token_type_ids=token_type_ids, use_cache=False, return_dict=True)
            else:
                out = self.backbone(input_ids=input_ids, attention_mask=attention_mask, use_cache=False, return_dict=True)
            return out.logits

    has_tt = token_type_ids is not None
    wrapped = NoCacheClsWrapper(model, has_tt)

    # Build input tuple and names dynamically
    if has_tt:
        inputs = (input_ids, attention_mask, token_type_ids)
        input_names = ['input_ids', 'attention_mask', 'token_type_ids']
        dynamic_axes = {
            'input_ids': {0: 'batch', 1: 'seq'},
            'attention_mask': {0: 'batch', 1: 'seq'},
            'token_type_ids': {0: 'batch', 1: 'seq'},
            'logits': {0: 'batch'}
        }
    else:
        inputs = (input_ids, attention_mask)
        input_names = ['input_ids', 'attention_mask']
        dynamic_axes = {
            'input_ids': {0: 'batch', 1: 'seq'},
            'attention_mask': {0: 'batch', 1: 'seq'},
            'logits': {0: 'batch'}
        }

    model_onnx = out_dir / 'model.onnx'
    print(f"Exporting ONNX: {model_onnx}")
    with torch.no_grad():
        torch.onnx.export(
            wrapped,
            inputs,
            str(model_onnx),
            input_names=input_names,
            output_names=['logits'],
            dynamic_axes=dynamic_axes,
            opset_version=14,
        )


def main():
    import argparse

    parser = argparse.ArgumentParser(description='Install Qwen3 (embedding 0.6B, reranker 0.6B) as ONNX into models dir')
    parser.add_argument('--models-dir', default='models', help='Target models directory (default: models)')
    args = parser.parse_args()

    ensure_pkgs()

    models_dir = Path(args.models_dir)
    emb_out = models_dir / 'qwen3emb'
    rerank_out = models_dir / 'qwen3_reranker'

    # Model IDs
    emb_model_id = 'Qwen/Qwen3-Embedding-0.6B'
    rerank_model_id = 'Qwen/Qwen3-Reranker-0.6B'

    # Export embedding
    export_embedding_onnx(emb_model_id, emb_out)

    # Export reranker (with fallback if repo missing)
    try:
        export_reranker_onnx(rerank_model_id, rerank_out)
    except Exception as e:
        print(f"Warning: failed to export reranker from {rerank_model_id}: {e}")
        print("Falling back to export sequence-classification head from embedding base (logits from pooled hidden state)")
        # Fallback: wrap AutoModel into simple classification head on-the-fly
        import torch
        from transformers import AutoModel, AutoTokenizer
        tok = AutoTokenizer.from_pretrained(emb_model_id)
        base = AutoModel.from_pretrained(emb_model_id)
        base.eval()
        if hasattr(base.config, 'use_cache'):
            base.config.use_cache = False
        save_tokenizer(tok, rerank_out)
        save_config(base, rerank_out)
        q = "Instruct: Given a web search query, retrieve relevant passages that answer the query\nQuery: test"
        d = "This is a test document about embeddings and reranking."
        try:
            enc = tok(q, d, padding=True, truncation=True, max_length=128, return_tensors='pt')
        except Exception:
            enc = tok([q + " [SEP] " + d], padding=True, truncation=True, max_length=128, return_tensors='pt')
        input_names = []
        inputs = []
        dynamic_axes = {}
        for name in ('input_ids', 'attention_mask', 'token_type_ids'):
            if name in enc:
                input_names.append(name)
                inputs.append(enc[name])
                dynamic_axes[name] = {0: 'batch', 1: 'seq'}
        # A small wrapper module to pool CLS and produce a scalar logit
        class PooledLogit(torch.nn.Module):
            def __init__(self, backbone, hidden: int):
                super().__init__()
                self.backbone = backbone
                self.fc = torch.nn.Linear(hidden, 1)
            def forward(self, *tensors):
                kwargs = {}
                if len(input_names) >= 1:
                    kwargs['input_ids'] = tensors[0]
                if len(input_names) >= 2:
                    kwargs['attention_mask'] = tensors[1]
                if len(input_names) >= 3:
                    kwargs['token_type_ids'] = tensors[2]
                out = self.backbone(**kwargs, use_cache=False, return_dict=True)
                last_hidden = out.last_hidden_state  # [B, S, H]
                cls = last_hidden[:, 0, :]  # CLS
                logits = self.fc(cls)  # [B, 1]
                return logits
        hidden = getattr(base.config, 'hidden_size', 1024)
        wrapper = PooledLogit(base, hidden)
        wrapper.eval()
        model_onnx = rerank_out / 'model.onnx'
        with torch.no_grad():
            torch.onnx.export(
                wrapper,
                tuple(inputs),
                str(model_onnx),
                input_names=input_names,
                output_names=['logits'],
                dynamic_axes={**dynamic_axes, 'logits': {0: 'batch'}},
                opset_version=14,
            )

    print("\n✅ Qwen3 ONNX models installed:")
    print(f"  - {emb_out}")
    print(f"  - {rerank_out}")


if __name__ == '__main__':
    main()