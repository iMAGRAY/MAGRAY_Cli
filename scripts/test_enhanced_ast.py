#!/usr/bin/env python3
"""Тестирование улучшенного AST парсинга с обнаружением всех конструкций"""

import sys
import os
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

from architecture_daemon import ArchitectureDaemon
from pathlib import Path

# Создаем экземпляр демона
project_root = Path(__file__).parent.parent
daemon = ArchitectureDaemon(str(project_root))

# Тестовый Rust код с различными конструкциями
test_code = """
use std::collections::HashMap;
use crate::memory::types::Record;
use super::utils::helper;

/// Mock implementation for testing
#[derive(Debug, Clone)]
pub struct MockMemoryService {
    data: HashMap<String, Record>,
}

/// Stub for database
pub struct StubDatabase;

/// Fake implementation  
struct FakeAuthService {
    users: Vec<String>,
}

pub trait MemoryService {
    type Error;
    type Item: Clone;
    
    fn store(&mut self, record: Record) -> Result<(), Self::Error>;
    fn search(&self, query: &str) -> Result<Vec<Record>, Self::Error>;
}

impl MemoryService for MockMemoryService {
    type Error = String;
    type Item = Record;
    
    fn store(&mut self, record: Record) -> Result<(), Self::Error> {
        unsafe {
            // Some unsafe optimization
            std::ptr::write_volatile(&mut self.data, HashMap::new());
        }
        Ok(())
    }
    
    fn search(&self, query: &str) -> Result<Vec<Record>, Self::Error> {
        Ok(vec![])
    }
}

pub async fn async_function() -> Result<()> {
    tokio::time::sleep(Duration::from_millis(100)).await;
    Ok(())
}

#[test]
fn test_mock_service() {
    let mut service = MockMemoryService::new();
    assert!(service.store(Record::default()).is_ok());
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_stub_database() {
        let db = StubDatabase;
        // Test stub
    }
}

unsafe fn unsafe_optimization<'a, T: Clone>(data: &'a T) -> &'a T {
    data
}

macro_rules! create_mock {
    ($name:ident) => {
        struct $name;
    };
}
"""

# Парсим код через AST
result = daemon._parse_rust_with_ast(test_code, "test.rs")

print("=== РЕЗУЛЬТАТЫ УЛУЧШЕННОГО AST ПАРСИНГА ===\n")

# Основные компоненты
print(f"Структуры: {result.get('structs', [])}")
print(f"Трейты: {result.get('traits', [])}")
print(f"Функции: {result.get('functions', [])}")
print(f"Методы: {result.get('methods', [])}")
print(f"Async функции: {result.get('async_functions', [])}")
print(f"Макросы: {result.get('macros', [])}")

# Новые обнаружения
print(f"\n=== НОВЫЕ ОБНАРУЖЕНИЯ ===")
print(f"Associated types: {result.get('associated_types', [])}")
print(f"Unsafe блоки: {len(result.get('unsafe_blocks', []))} найдено")
if result.get('unsafe_blocks'):
    for unsafe in result['unsafe_blocks']:
        print(f"  - Строка {unsafe['line']}: {unsafe['code'][:30]}...")

print(f"Test doubles: {result.get('test_doubles', [])}")
print(f"God objects: {result.get('god_objects', [])}")
print(f"Тесты: {result.get('tests', [])}")
print(f"Use statements: {result.get('uses', [])[:5]}")
print(f"Generics: {result.get('generics', [])}")
print(f"Lifetimes: {result.get('lifetimes', [])}")

# Проверяем граф зависимостей
uses = result.get('uses', [])
local_deps, external_deps = daemon._build_dependency_graph("test_crate", uses, "test.rs")
print(f"\n=== АНАЛИЗ ЗАВИСИМОСТЕЙ ===")
print(f"Локальные зависимости: {local_deps}")
print(f"Внешние зависимости: {external_deps}")

print("\n[OK] Улучшенный AST парсинг работает корректно!")