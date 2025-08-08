#!/usr/bin/env python3
"""
Поиск самых свежих версий файлов в истории Cursor
с августа 2025 года
"""

import os
import json
from pathlib import Path
from datetime import datetime
from urllib.parse import unquote
import shutil

def find_august_files():
    """Найти все файлы с августа 2025"""
    
    history_base = Path(r'C:\Users\1\AppData\Roaming\Cursor\User\History')
    august_files = []
    
    print("Scanning Cursor history for August 2025 files...")
    print("=" * 80)
    
    for history_dir in history_base.iterdir():
        if not history_dir.is_dir():
            continue
            
        entries_file = history_dir / 'entries.json'
        if not entries_file.exists():
            continue
            
        try:
            with open(entries_file, 'r', encoding='utf-8') as f:
                data = json.load(f)
                
            for entry in data.get('entries', []):
                # Проверяем timestamp
                timestamp = entry.get('timestamp', 0)
                if timestamp:
                    dt = datetime.fromtimestamp(timestamp / 1000)
                    
                    # Ищем файлы с августа 2025
                    if dt.year == 2025 and dt.month == 8:
                        resource = entry.get('resource', '')
                        if resource.startswith('file:///'):
                            file_path = unquote(resource.replace('file:///', ''))
                            file_path = file_path.replace('/', '\\')
                            
                            # Ищем соответствующий файл в истории
                            for hist_file in history_dir.iterdir():
                                if hist_file.suffix and hist_file.stem != 'entries':
                                    august_files.append({
                                        'source': hist_file,
                                        'target': file_path,
                                        'timestamp': dt,
                                        'size': hist_file.stat().st_size
                                    })
                                    
        except Exception as e:
            continue
    
    # Сортируем по времени (новые первые)
    august_files.sort(key=lambda x: x['timestamp'], reverse=True)
    
    # Группируем по целевым файлам, берем самую новую версию
    latest_versions = {}
    for file_info in august_files:
        target = file_info['target']
        if target not in latest_versions:
            latest_versions[target] = file_info
            print(f"Found: {Path(target).name}")
            print(f"  Date: {file_info['timestamp']}")
            print(f"  Size: {file_info['size']:,} bytes")
            print(f"  Source: {file_info['source'].name}")
            print()
    
    return latest_versions

def restore_august_files():
    """Восстановить файлы с августа"""
    
    latest_files = find_august_files()
    
    if not latest_files:
        print("No August 2025 files found!")
        return False
    
    print("\n" + "=" * 80)
    print(f"RESTORING {len(latest_files)} FILES FROM AUGUST 2025")
    print("=" * 80 + "\n")
    
    success = 0
    failed = 0
    
    for target_path, file_info in latest_files.items():
        source = file_info['source']
        target = Path(target_path)
        
        if not source.exists():
            print(f"❌ Source not found: {source.name}")
            failed += 1
            continue
            
        try:
            # Создаем директорию если нужно
            target.parent.mkdir(parents=True, exist_ok=True)
            
            # Копируем файл
            shutil.copy2(source, target)
            
            if target.exists():
                print(f"✅ Restored: {target.name} ({file_info['size']:,} bytes)")
                success += 1
            else:
                print(f"❌ Failed: {target.name}")
                failed += 1
                
        except Exception as e:
            print(f"❌ Error restoring {target.name}: {e}")
            failed += 1
    
    print("\n" + "=" * 80)
    print(f"RESULTS: {success} restored, {failed} failed")
    print("=" * 80)
    
    return success > 0

if __name__ == "__main__":
    import sys
    success = restore_august_files()
    sys.exit(0 if success else 1)