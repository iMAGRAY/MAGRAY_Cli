#!/usr/bin/env python3
"""
Скрипт для восстановления файлов проекта MAGRAY_Cli из истории Cursor IDE
"""

import os
import json
import shutil
from pathlib import Path
from urllib.parse import unquote
import re

# Путь к истории Cursor
CURSOR_HISTORY_PATH = Path(r"C:\Users\1\AppData\Roaming\Cursor\User\History")

# Целевая директория для восстановления
TARGET_DIR = Path(r"C:\Users\1\Documents\GitHub\MAGRAY_Cli")

def extract_file_path_from_resource(resource_url):
    """Извлечь путь файла из URL ресурса"""
    # Извлекаем путь из URL вида: file:///c%3A/Users/1/Documents/GitHub/MAGRAY_Cli/...
    match = re.search(r'file:///(.*)', resource_url)
    if match:
        encoded_path = match.group(1)
        # Декодируем URL и исправляем слэши
        decoded_path = unquote(encoded_path).replace('/', '\\')
        # Исправляем двоеточие диска
        decoded_path = decoded_path.replace('%3A', ':').replace('c:', 'C:')
        return Path(decoded_path)
    return None

def find_magray_files():
    """Найти все файлы проекта MAGRAY_Cli в истории Cursor"""
    magray_files = {}
    
    for history_dir in CURSOR_HISTORY_PATH.iterdir():
        if not history_dir.is_dir():
            continue
            
        entries_file = history_dir / "entries.json"
        if not entries_file.exists():
            continue
            
        try:
            with open(entries_file, 'r', encoding='utf-8') as f:
                data = json.load(f)
                
            resource = data.get('resource', '')
            if 'MAGRAY_Cli' in resource:
                file_path = extract_file_path_from_resource(resource)
                if file_path:
                    entries = data.get('entries', [])
                    if entries:
                        # Берем последнюю версию файла
                        last_entry = entries[-1]
                        history_file = history_dir / last_entry['id']
                        
                        if history_file.exists():
                            timestamp = last_entry.get('timestamp', 0)
                            
                            # Сохраняем только самую новую версию каждого файла
                            rel_path = file_path.relative_to(Path("C:\\Users\\1\\Documents\\GitHub\\MAGRAY_Cli"))
                            if rel_path not in magray_files or magray_files[rel_path]['timestamp'] < timestamp:
                                magray_files[rel_path] = {
                                    'source': history_file,
                                    'target': file_path,
                                    'timestamp': timestamp
                                }
        except Exception as e:
            print(f"Ошибка при чтении {entries_file}: {e}")
            
    return magray_files

def restore_files(files_dict):
    """Восстановить файлы из истории"""
    restored_count = 0
    error_count = 0
    
    for rel_path, file_info in sorted(files_dict.items()):
        source = file_info['source']
        target = file_info['target']
        
        # Создаем директории если их нет
        target.parent.mkdir(parents=True, exist_ok=True)
        
        try:
            # Копируем файл
            shutil.copy2(source, target)
            print(f"  [OK] Restored: {rel_path}")
            restored_count += 1
        except Exception as e:
            print(f"  [ERROR] Failed to restore {rel_path}: {e}")
            error_count += 1
            
    return restored_count, error_count

def main():
    print("Searching for MAGRAY_Cli project files in Cursor history...")
    files = find_magray_files()
    
    if not files:
        print("ERROR: No project files found in history")
        return
        
    print(f"Found {len(files)} project files")
    
    # Показываем структуру проекта
    print("\nProject structure:")
    directories = set()
    for rel_path in sorted(files.keys()):
        directories.add(rel_path.parent)
        
    for dir_path in sorted(directories):
        level = len(dir_path.parts)
        indent = "  " * level
        print(f"{indent}[DIR] {dir_path}")
        
    print(f"\nRestoring files to {TARGET_DIR}...")
    restored, errors = restore_files(files)
    
    print(f"\nSUCCESS: Restored {restored} files")
    if errors > 0:
        print(f"ERRORS: Failed to restore {errors} files")
    
    print("\nRestoration complete!")

if __name__ == "__main__":
    main()