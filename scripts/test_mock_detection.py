#!/usr/bin/env python3
"""Тестирование обнаружения моков, стабов и test doubles"""

import sys
import os
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

from architecture_daemon import ArchitectureDaemon
from pathlib import Path

# Создаем экземпляр демона
project_root = Path(__file__).parent.parent
daemon = ArchitectureDaemon(str(project_root))

# Тестируем на файле с моками
test_file = project_root / "crates" / "memory" / "tests" / "test_mocks_and_stubs.rs"

if test_file.exists():
    print(f"Анализируем файл: {test_file}")
    
    with open(test_file, 'r', encoding='utf-8') as f:
        content = f.read()
    
    # Парсим через AST
    result = daemon._parse_rust_with_ast(content, str(test_file))
    
    print("\n=== ОБНАРУЖЕНИЕ TEST DOUBLES ===\n")
    
    # Показываем найденные компоненты
    print(f"Структуры: {result.get('structs', [])}")
    print(f"Трейты: {result.get('traits', [])}")
    print(f"Функции: {result.get('functions', [])}")
    
    # Test doubles
    print(f"\nTest Doubles найдено: {len(result.get('test_doubles', []))}")
    for td in result.get('test_doubles', []):
        print(f"  - {td['type'].upper()}: {td['name']} ({td['node_type']})")
    
    # Mock атрибуты
    print(f"\nMock атрибуты: {result.get('mock_attributes', [])}")
    
    # Тесты
    print(f"\nТесты: {result.get('tests', [])}")
    
    # Проверяем обнаружение всех типов
    expected_mocks = ['MockMemoryService']
    expected_stubs = ['StubDatabase']
    expected_fakes = ['FakeMemoryService']
    expected_dummies = ['DummyLogger']
    expected_spies = ['SpyService']
    
    found_names = [td['name'] for td in result.get('test_doubles', [])]
    
    print("\n=== ПРОВЕРКА ПОЛНОТЫ ОБНАРУЖЕНИЯ ===")
    for expected in expected_mocks + expected_stubs + expected_fakes + expected_dummies + expected_spies:
        if expected in found_names:
            print(f"[OK] {expected} найден")
        else:
            print(f"[MISSING] {expected} НЕ найден")
    
    # Общая статистика
    total_expected = len(expected_mocks + expected_stubs + expected_fakes + expected_dummies + expected_spies)
    total_found = len([n for n in found_names if n in expected_mocks + expected_stubs + expected_fakes + expected_dummies + expected_spies])
    
    print(f"\nИтого: {total_found}/{total_expected} test doubles обнаружено")
    
    if total_found == total_expected:
        print("\n[OK] Все test doubles успешно обнаружены!")
    else:
        print(f"\n[WARNING] Обнаружено только {total_found} из {total_expected}")
else:
    print(f"Файл не найден: {test_file}")