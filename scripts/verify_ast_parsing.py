#!/usr/bin/env python3
"""Детальная проверка AST парсинга в демоне"""

import sys
import os
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

from architecture_daemon import ArchitectureDaemon
from pathlib import Path

# Создаем экземпляр демона
project_root = Path(__file__).parent.parent
daemon = ArchitectureDaemon(str(project_root))

# Проверяем инициализацию tree-sitter
print("=== ПРОВЕРКА TREE-SITTER ===")
if daemon.rust_parser:
    print("[OK] Tree-sitter парсер инициализирован")
else:
    print("[ERROR] Tree-sitter парсер НЕ инициализирован")

# Тестируем на реальном файле проекта
test_files = [
    project_root / "crates" / "memory" / "src" / "lib.rs",
    project_root / "crates" / "cli" / "src" / "main.rs",
    project_root / "crates" / "ai" / "src" / "lib.rs"
]

for test_file in test_files:
    if not test_file.exists():
        continue
    
    print(f"\n=== АНАЛИЗ ФАЙЛА: {test_file.name} ===")
    
    with open(test_file, 'r', encoding='utf-8') as f:
        content = f.read()
    
    # Вызываем AST парсер напрямую
    result = daemon._parse_rust_with_ast(content, str(test_file))
    
    # Выводим результаты
    print(f"Структуры:     {len(result.get('structs', []))} шт.")
    if result.get('structs'):
        print(f"  Примеры: {result['structs'][:3]}")
    
    print(f"Функции:       {len(result.get('functions', []))} шт.")
    if result.get('functions'):
        print(f"  Примеры: {result['functions'][:3]}")
    
    print(f"Трейты:        {len(result.get('traits', []))} шт.")
    if result.get('traits'):
        print(f"  Примеры: {result['traits'][:3]}")
    
    print(f"Impl блоки:    {len(result.get('impl_blocks', []))} шт.")
    if result.get('impl_blocks'):
        print(f"  Примеры: {result['impl_blocks'][:3]}")
    
    print(f"Методы:        {len(result.get('methods', []))} шт.")
    if result.get('methods'):
        print(f"  Примеры: {result['methods'][:3]}")
    
    print(f"Тесты:         {len(result.get('tests', []))} шт.")
    print(f"Моки:          {len(result.get('mocks', []))} шт.")
    print(f"Async функции: {len(result.get('async_functions', []))} шт.")

# Проверяем общую статистику
print("\n=== ОБЩАЯ СТАТИСТИКА ПРОЕКТА ===")
architecture = daemon.scan_architecture()
total_files = sum(len(files) for files in architecture['file_structures'].values())
print(f"Всего crates: {len(architecture['crates'])}")
print(f"Всего файлов проанализировано: {total_files}")

# Проверяем что используется именно AST, а не regex
print("\n=== ПРОВЕРКА МЕТОДОВ ПАРСИНГА ===")

# Создаем простой тестовый Rust код
test_code = """
pub struct TestStruct {
    field: String,
}

impl TestStruct {
    pub fn new() -> Self {
        Self { field: String::new() }
    }
}

#[test]
fn test_function() {
    assert!(true);
}
"""

# Парсим через AST
ast_result = daemon._parse_rust_with_ast(test_code, "test.rs")
# Парсим через regex (fallback)
regex_result = daemon._parse_rust_with_regex(test_code, "test.rs")

print("AST парсер нашел:")
print(f"  Структуры: {ast_result.get('structs', [])}")
print(f"  Функции: {ast_result.get('functions', [])}")
print(f"  Методы: {ast_result.get('methods', [])}")
print(f"  Тесты: {ast_result.get('tests', [])}")

print("\nRegex парсер нашел:")
print(f"  Структуры: {regex_result.get('structs', [])}")
print(f"  Функции: {regex_result.get('functions', [])}")
print(f"  Тесты: {regex_result.get('tests', [])}")

if ast_result.get('structs') == ['TestStruct'] and ast_result.get('methods'):
    print("\n[OK] AST ПАРСИНГ РАБОТАЕТ КОРРЕКТНО!")
else:
    print("\n[WARNING] AST парсинг работает, но результаты неожиданные")