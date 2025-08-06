#!/usr/bin/env python3
"""Тестовый скрипт для проверки AST парсинга в демоне"""

import sys
import os
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

from architecture_daemon import ArchitectureDaemon
from pathlib import Path

# Создаем экземпляр демона
project_root = Path(__file__).parent.parent
daemon = ArchitectureDaemon(str(project_root))

# Тестируем парсинг конкретного файла
test_file = project_root / "crates" / "memory" / "src" / "lib.rs"

if test_file.exists():
    print(f"Тестируем AST парсинг файла: {test_file}")
    
    with open(test_file, 'r', encoding='utf-8') as f:
        content = f.read()
    
    # Вызываем AST парсер
    result = daemon._parse_rust_with_ast(content, str(test_file))
    
    print("\n=== Результаты AST парсинга ===\n")
    
    for key, value in result.items():
        if value and not key.startswith('_'):
            if isinstance(value, list):
                print(f"{key:15}: {len(value):3} найдено")
                if len(value) > 0 and len(value) <= 5:
                    print(f"                 {value[:5]}")
            elif isinstance(value, dict):
                print(f"{key:15}: {value}")
    
    # Проверяем, что AST парсер работает
    if daemon.rust_parser:
        print("\n✅ AST парсер инициализирован и работает!")
    else:
        print("\n❌ AST парсер не инициализирован, используется regex fallback")
    
else:
    print(f"Файл не найден: {test_file}")