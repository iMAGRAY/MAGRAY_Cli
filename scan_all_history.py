#!/usr/bin/env python3
"""
Полное сканирование истории Cursor - найти ВСЕ файлы
и их временные метки
"""

import os
import json
from pathlib import Path
from datetime import datetime
from urllib.parse import unquote
import shutil

def scan_complete_history():
    """Сканировать всю историю и найти самые свежие файлы"""
    
    history_base = Path(r'C:\Users\1\AppData\Roaming\Cursor\User\History')
    all_files = []
    
    print("Complete scan of Cursor history...")
    print("=" * 80)
    
    # Сканируем все директории
    for history_dir in history_base.iterdir():
        if not history_dir.is_dir():
            continue
            
        # Проверяем entries.json
        entries_file = history_dir / 'entries.json'
        
        # Собираем все файлы в директории
        for hist_file in history_dir.iterdir():
            if hist_file.is_file() and hist_file.name != 'entries.json':
                # Используем время модификации самого файла
                mtime = datetime.fromtimestamp(hist_file.stat().st_mtime)
                
                # Пытаемся найти соответствие в entries.json
                target_path = None
                if entries_file.exists():
                    try:
                        with open(entries_file, 'r', encoding='utf-8') as f:
                            data = json.load(f)
                        
                        for entry in data.get('entries', []):
                            resource = entry.get('resource', '')
                            if resource.startswith('file:///'):
                                target_path = unquote(resource.replace('file:///', ''))
                                target_path = target_path.replace('/', '\\')
                                break
                    except:
                        pass
                
                # Если не нашли путь в entries, пробуем угадать по расширению
                if not target_path:
                    ext = hist_file.suffix
                    if ext == '.md':
                        if 'README' in str(hist_file):
                            target_path = r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\README.md'
                    elif ext == '.rs':
                        target_path = r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\src\main.rs'
                    elif ext == '.toml':
                        target_path = r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\Cargo.toml'
                
                all_files.append({
                    'source': hist_file,
                    'target': target_path or 'unknown',
                    'mtime': mtime,
                    'size': hist_file.stat().st_size,
                    'dir': history_dir.name
                })
    
    # Сортируем по времени модификации (новые первые)
    all_files.sort(key=lambda x: x['mtime'], reverse=True)
    
    print(f"Found {len(all_files)} total files in history\n")
    print("Latest 20 files by modification time:")
    print("-" * 80)
    
    for i, file_info in enumerate(all_files[:20]):
        print(f"{i+1}. {file_info['mtime'].strftime('%Y-%m-%d %H:%M:%S')}")
        print(f"   File: {file_info['source'].name}")
        print(f"   Size: {file_info['size']:,} bytes")
        print(f"   Dir: {file_info['dir']}")
        if file_info['target'] != 'unknown':
            print(f"   Target: {Path(file_info['target']).name}")
        print()
    
    # Найдем файлы README
    print("\n" + "=" * 80)
    print("Looking for README files...")
    print("-" * 80)
    
    readme_files = []
    for file_info in all_files:
        # Проверяем содержимое файла на наличие "MAGRAY"
        if file_info['size'] > 100:  # Не пустой файл
            try:
                content = file_info['source'].read_text(encoding='utf-8', errors='ignore')
                if 'MAGRAY' in content or 'CLI Agent' in content:
                    readme_files.append(file_info)
                    print(f"Found potential README: {file_info['source'].name}")
                    print(f"  Modified: {file_info['mtime']}")
                    print(f"  Size: {file_info['size']:,} bytes")
                    print(f"  First line: {content.split(chr(10))[0][:60]}...")
                    print()
            except:
                pass
    
    return all_files, readme_files

def restore_latest():
    """Восстановить самые свежие версии"""
    
    all_files, readme_files = scan_complete_history()
    
    if readme_files:
        print("\n" + "=" * 80)
        print("RESTORING LATEST README")
        print("=" * 80)
        
        # Берем самый свежий README
        latest_readme = readme_files[0]
        source = latest_readme['source']
        target = Path(r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\README.md')
        
        try:
            shutil.copy2(source, target)
            print(f"✅ Restored README.md from {source.name}")
            print(f"   Size: {target.stat().st_size:,} bytes")
            
            # Показываем первые строки
            content = target.read_text(encoding='utf-8')
            lines = content.split('\n')[:5]
            print("\n   First 5 lines:")
            for line in lines:
                print(f"   {line}")
                
        except Exception as e:
            print(f"❌ Failed to restore README: {e}")
    
    return True

if __name__ == "__main__":
    import sys
    success = restore_latest()
    sys.exit(0 if success else 1)