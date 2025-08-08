#!/usr/bin/env python3
"""
Полное восстановление ВСЕХ файлов из истории Cursor
с проверкой и отчетом
"""

import os
import shutil
from pathlib import Path
import json
from datetime import datetime

# Импортируем данные из analyze_cursor_timeline
exec(open('analyze_cursor_timeline.py', encoding='utf-8').read())

def complete_restore():
    """Полное восстановление с проверкой"""
    
    print("=" * 80)
    print("COMPLETE RESTORATION FROM CURSOR HISTORY")
    print("=" * 80)
    print()
    
    print("Scanning history...")
    all_changes = find_all_changes()
    
    if not all_changes:
        print("ERROR: No changes found in history")
        return False
    
    print(f"Found {len(all_changes)} file versions to restore\n")
    
    # Группируем по файлам чтобы взять последнюю версию каждого
    file_versions = {}
    for change in all_changes:
        file_path = str(change['file_path'])
        if file_path not in file_versions or change['timestamp'] > file_versions[file_path]['timestamp']:
            file_versions[file_path] = change
    
    print(f"Unique files to restore: {len(file_versions)}\n")
    
    success = 0
    failed = 0
    skipped = 0
    
    for file_path, change in file_versions.items():
        source = Path(change['history_file'])
        target = Path(file_path)
        
        # Пропускаем если источник не существует
        if not source.exists():
            print(f"SKIP: Source not found for {target.name}")
            skipped += 1
            continue
        
        try:
            # Создаем директории если нужно
            target.parent.mkdir(parents=True, exist_ok=True)
            
            # Копируем файл
            shutil.copy2(source, target)
            
            # Проверяем что файл скопировался
            if target.exists():
                size = target.stat().st_size
                print(f"OK {target.name} ({size:,} bytes) - {change['datetime'].strftime('%Y-%m-%d %H:%M')}")
                success += 1
            else:
                print(f"FAIL {target.name} - Failed to copy")
                failed += 1
                
        except Exception as e:
            print(f"FAIL {target.name} - Error: {e}")
            failed += 1
    
    print()
    print("=" * 80)
    print("RESTORATION SUMMARY")
    print("=" * 80)
    print(f"Successfully restored: {success} files")
    print(f"Failed: {failed} files")
    print(f"Skipped (source not found): {skipped} files")
    print(f"Total processed: {len(file_versions)} files")
    print()
    
    # Создаем отчет
    report = {
        'timestamp': datetime.now().isoformat(),
        'total_versions': len(all_changes),
        'unique_files': len(file_versions),
        'restored': success,
        'failed': failed,
        'skipped': skipped,
        'files': []
    }
    
    for file_path, change in file_versions.items():
        report['files'].append({
            'path': str(file_path),
            'last_modified': change['datetime'].isoformat(),
            'history_location': str(change['history_file']),
            'restored': Path(file_path).exists()
        })
    
    # Сохраняем отчет
    with open('restoration_report.json', 'w', encoding='utf-8') as f:
        json.dump(report, f, indent=2, ensure_ascii=False)
    
    print(f"Report saved to: restoration_report.json")
    
    return success > 0

if __name__ == "__main__":
    import sys
    success = complete_restore()
    sys.exit(0 if success else 1)