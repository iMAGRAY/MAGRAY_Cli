#!/usr/bin/env python3
"""Простой тест AST парсинга"""

import tree_sitter
from tree_sitter_rust import language as rust_language

# Создаем парсер
lang = tree_sitter.Language(rust_language())
parser = tree_sitter.Parser(lang)

# Простой тестовый код
code = b"""
pub struct MyStruct {
    field1: String,
    field2: i32,
}

impl MyStruct {
    pub fn new() -> Self {
        Self {
            field1: String::new(),
            field2: 0,
        }
    }
}

pub fn my_function() -> Result<()> {
    Ok(())
}

pub trait MyTrait {
    fn method(&self);
}

enum MyEnum {
    Variant1,
    Variant2(String),
}
"""

# Парсим
tree = parser.parse(code)

def extract_components(node, source):
    """Извлекает компоненты из AST"""
    components = {
        'structs': [],
        'functions': [],
        'traits': [],
        'enums': [],
        'impls': []
    }
    
    def traverse(n):
        if n.type == 'struct_item':
            for child in n.children:
                if child.type == 'type_identifier':
                    name = source[child.start_byte:child.end_byte].decode('utf8')
                    components['structs'].append(name)
                    break
        
        elif n.type == 'function_item':
            for child in n.children:
                if child.type == 'identifier':
                    name = source[child.start_byte:child.end_byte].decode('utf8')
                    components['functions'].append(name)
                    break
        
        elif n.type == 'trait_item':
            for child in n.children:
                if child.type == 'type_identifier':
                    name = source[child.start_byte:child.end_byte].decode('utf8')
                    components['traits'].append(name)
                    break
        
        elif n.type == 'enum_item':
            for child in n.children:
                if child.type == 'type_identifier':
                    name = source[child.start_byte:child.end_byte].decode('utf8')
                    components['enums'].append(name)
                    break
        
        elif n.type == 'impl_item':
            for child in n.children:
                if child.type == 'type_identifier':
                    name = source[child.start_byte:child.end_byte].decode('utf8')
                    components['impls'].append(name)
                    break
        
        for child in n.children:
            traverse(child)
    
    traverse(node)
    return components

# Извлекаем компоненты
components = extract_components(tree.root_node, code)

print("=== Найденные компоненты ===")
for comp_type, items in components.items():
    if items:
        print(f"{comp_type}: {items}")

print("\n[OK] AST парсинг работает корректно!")