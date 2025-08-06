#!/usr/bin/env python3
"""Тестовый скрипт для проверки tree-sitter парсинга v3"""

import tree_sitter
from tree_sitter_rust import language

# Создаем Language объект
rust_language = tree_sitter.Language(language())

# Создаем парсер
parser = tree_sitter.Parser(rust_language)

# Тестовый Rust код
rust_code = b'''
pub struct TestStruct {
    name: String,
    value: i32,
}

impl TestStruct {
    pub fn new(name: String) -> Self {
        Self { name, value: 0 }
    }
    
    pub async fn process(&mut self) {
        self.value += 1;
    }
}

#[test]
fn test_creation() {
    let test = TestStruct::new("test".to_string());
    assert_eq!(test.value, 0);
}

struct MockService;

impl Service for MockService {
    fn execute(&self) -> Result<()> {
        Ok(())
    }
}
'''

# Парсим код
tree = parser.parse(rust_code)

print("=== Tree-sitter AST parsing успешно работает! ===\n")
print(f"Root node type: {tree.root_node.type}")
print(f"Children count: {len(tree.root_node.children)}")

# Простой тест извлечения структур
structs = []
for child in tree.root_node.children:
    if child.type == 'struct_item':
        for subchild in child.children:
            if subchild.type == 'type_identifier':
                name = rust_code[subchild.start_byte:subchild.end_byte].decode('utf8')
                structs.append(name)
                break

print(f"\nНайденные структуры: {structs}")
print("\n[OK] Tree-sitter работает!")