#!/usr/bin/env python3
"""
Восстановить правильные версии файлов MAGRAY CLI
"""

import shutil
from pathlib import Path

def restore_magray_files():
    """Восстановить файлы проекта MAGRAY"""
    
    restorations = [
        # README с MAGRAY CLI
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\-2b242f8a\BH2y.md',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\README.md',
            'desc': 'MAGRAY CLI README'
        },
        # Альтернативные версии README
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\-522516f5\dsgh.md', 
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\README_alt.md',
            'desc': 'Alternative README'
        },
        # Последняя версия main.rs
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\-3c64f04a\0Goa.rs',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\src\main.rs',
            'desc': 'Latest main.rs'
        },
        # CLAUDE.md
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\-4c3113c5\971o.md',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\CLAUDE.md',
            'desc': 'CLAUDE.md configuration'
        },
        # Cargo.toml
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\-1cff928a\BuXg.toml',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\Cargo.toml',
            'desc': 'Cargo.toml workspace'
        },
        # Makefile
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\-5c6ba04a\i3Eu',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\Makefile',
            'desc': 'MAGRAY CLI Build System'
        }
    ]
    
    print("=" * 80)
    print("RESTORING MAGRAY CLI FILES")
    print("=" * 80)
    print()
    
    success = 0
    failed = 0
    
    for item in restorations:
        source = Path(item['source'])
        target = Path(item['target'])
        
        print(f"Restoring: {item['desc']}")
        print(f"  From: {source}")
        print(f"  To:   {target}")
        
        if not source.exists():
            print(f"  ERROR: Source not found!")
            failed += 1
            print()
            continue
            
        try:
            # Создаем директорию если нужно
            target.parent.mkdir(parents=True, exist_ok=True)
            
            # Копируем файл
            shutil.copy2(source, target)
            
            # Проверяем
            if target.exists():
                size = target.stat().st_size
                print(f"  SUCCESS: {size:,} bytes")
                
                # Показываем первую строку для .md файлов
                if target.suffix == '.md':
                    try:
                        content = target.read_text(encoding='utf-8', errors='ignore')
                        first_line = content.split('\n')[0]
                        print(f"  Content: {first_line[:60]}...")
                    except:
                        pass
                        
                success += 1
            else:
                print(f"  ERROR: Failed to copy")
                failed += 1
                
        except Exception as e:
            print(f"  ERROR: {e}")
            failed += 1
            
        print()
    
    print("=" * 80)
    print(f"RESULTS: {success} success, {failed} failed")
    print("=" * 80)
    
    return success > 0

if __name__ == "__main__":
    import sys
    success = restore_magray_files()
    sys.exit(0 if success else 1)