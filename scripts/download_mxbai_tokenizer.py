#!/usr/bin/env python3
"""
Скрипт для скачивания токенизатора MXBai reranker модели
"""

import os
import sys
import json
import requests
from pathlib import Path

# MXBai reranker model на Hugging Face
MODEL_ID = "mixedbread-ai/mxbai-rerank-base-v1"
BASE_URL = f"https://huggingface.co/{MODEL_ID}/resolve/main"

# Файлы которые нужно скачать
FILES_TO_DOWNLOAD = [
    "tokenizer.json",
    "tokenizer_config.json", 
    "vocab.txt",
    "special_tokens_map.json",
    "merges.txt"  # если это BPE токенизатор
]

def download_file(url: str, dest_path: Path) -> bool:
    """Скачивает файл с прогресс-баром"""
    try:
        print(f"📥 Скачивание {dest_path.name}...")
        
        response = requests.get(url, stream=True)
        response.raise_for_status()
        
        # Создаем директорию если не существует
        dest_path.parent.mkdir(parents=True, exist_ok=True)
        
        total_size = int(response.headers.get('content-length', 0))
        
        with open(dest_path, 'wb') as f:
            downloaded = 0
            for chunk in response.iter_content(chunk_size=8192):
                if chunk:
                    f.write(chunk)
                    downloaded += len(chunk)
                    
                    if total_size > 0:
                        percent = (downloaded / total_size) * 100
                        print(f"  Progress: {percent:.1f}%", end='\r')
        
        print(f"  ✅ {dest_path.name} скачан ({downloaded:,} bytes)")
        return True
        
    except Exception as e:
        print(f"  ❌ Ошибка скачивания {dest_path.name}: {e}")
        return False

def main():
    print("🤖 MXBai Tokenizer Downloader")
    print("=" * 50)
    
    # Путь к модели
    model_dir = Path(__file__).parent.parent / "crates" / "memory" / "models" / "mxbai_rerank_base_v2"
    
    if not model_dir.exists():
        print(f"❌ Директория модели не найдена: {model_dir}")
        return 1
    
    print(f"📁 Директория модели: {model_dir}")
    
    successful_downloads = 0
    
    for filename in FILES_TO_DOWNLOAD:
        url = f"{BASE_URL}/{filename}"
        dest_path = model_dir / filename
        
        # Пропускаем если файл уже существует
        if dest_path.exists():
            print(f"⏭️ {filename} уже существует, пропускаем")
            successful_downloads += 1
            continue
        
        if download_file(url, dest_path):
            successful_downloads += 1
    
    print(f"\n📊 Результат: {successful_downloads}/{len(FILES_TO_DOWNLOAD)} файлов скачано")
    
    if successful_downloads == len(FILES_TO_DOWNLOAD):
        print("🎉 Все файлы токенизатора успешно скачаны!")
        
        # Проверяем что tokenizer.json валидный JSON
        tokenizer_path = model_dir / "tokenizer.json"
        if tokenizer_path.exists():
            try:
                with open(tokenizer_path, 'r', encoding='utf-8') as f:
                    tokenizer_data = json.load(f)
                print(f"✅ tokenizer.json валидный (vocab_size: {len(tokenizer_data.get('model', {}).get('vocab', {}))})")
            except Exception as e:
                print(f"⚠️ tokenizer.json может быть невалидным: {e}")
        
        return 0
    else:
        print("⚠️ Некоторые файлы не удалось скачать")
        return 1

if __name__ == "__main__":
    sys.exit(main())