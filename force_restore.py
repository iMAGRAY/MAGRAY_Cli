#!/usr/bin/env python3
"""
Принудительное восстановление файлов из истории Cursor
с перезаписью существующих файлов
"""

import shutil
from pathlib import Path
import sys

def force_restore():
    """Принудительно восстановить ключевые файлы"""
    
    restorations = [
        # README файлы
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\-663df50e\GekM.md',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\README.md',
            'desc': 'Main README.md'
        },
        # Проверим что в последней версии CLAUDE.md
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\-4c3113c5\971o.md', 
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\CLAUDE.md',
            'desc': 'CLAUDE.md configuration'
        },
        # Main.rs 
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\54e9e6a5\1GGJ.rs',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\crates\cli\src\main.rs',
            'desc': 'CLI main.rs'
        },
        # Tool orchestrator
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\-3c64f04a\0Goa.rs',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\crates\cli\src\orchestrator\tool_orchestrator.rs',
            'desc': 'Tool orchestrator'
        },
        # Memory API
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\-1067d525\3aQX.rs',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\crates\memory\src\api.rs',
            'desc': 'Memory API'
        },
        # Makefile
        {
            'source': r'C:\Users\1\AppData\Roaming\Cursor\User\History\-5c6ba04a\i3Eu',
            'target': r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\Makefile',
            'desc': 'Makefile'
        }
    ]
    
    print("=" * 60)
    print("FORCE RESTORE FROM CURSOR HISTORY")
    print("=" * 60)
    print()
    
    success_count = 0
    fail_count = 0
    
    for item in restorations:
        source = Path(item['source'])
        target = Path(item['target'])
        
        print(f"Restoring: {item['desc']}")
        print(f"  From: {source.name}")
        print(f"  To:   {target}")
        
        if not source.exists():
            print(f"  ERROR: Source file not found!")
            fail_count += 1
            continue
            
        try:
            # Создаем директорию если нужно
            target.parent.mkdir(parents=True, exist_ok=True)
            
            # Копируем с перезаписью
            shutil.copy2(source, target)
            
            # Проверяем размер
            size = target.stat().st_size
            print(f"  SUCCESS: {size} bytes written")
            success_count += 1
            
        except Exception as e:
            print(f"  ERROR: {e}")
            fail_count += 1
        
        print()
    
    print("=" * 60)
    print(f"COMPLETED: {success_count} success, {fail_count} failed")
    print("=" * 60)
    
    return success_count > 0

if __name__ == "__main__":
    success = force_restore()
    sys.exit(0 if success else 1)