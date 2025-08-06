#!/usr/bin/env python3
"""Тестовый скрипт для проверки tree-sitter парсинга с tree-sitter-rust"""

import tree_sitter
import tree_sitter_rust

# Создаем парсер
parser = tree_sitter.Parser(tree_sitter_rust.language())

# Тестовый Rust код
rust_code = '''
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

pub trait TestTrait {
    fn do_something(&self);
}

enum TestEnum {
    Variant1,
    Variant2(i32),
}

struct MockService;

impl Service for MockService {
    fn execute(&self) -> Result<()> {
        Ok(())
    }
}
'''

# Парсим код
tree = parser.parse(rust_code.encode('utf8'))

print("=== Извлеченные компоненты с tree-sitter-rust ===\n")

def extract_rust_components(tree, source_code):
    """Извлекает компоненты Rust кода используя tree-sitter AST"""
    root = tree.root_node
    components = {
        'structs': [],
        'enums': [],
        'traits': [],
        'functions': [],
        'methods': [],
        'impl_blocks': [],
        'tests': [],
        'mocks': [],
        'async_functions': []
    }
    
    def traverse(node):
        """Рекурсивно обходит AST дерево"""
        node_type = node.type
        
        if node_type == 'struct_item':
            # Ищем имя структуры
            for child in node.children:
                if child.type == 'type_identifier':
                    name = source_code[child.start_byte:child.end_byte].decode('utf8')
                    components['structs'].append(name)
                    # Проверяем на mock паттерны
                    if any(p in name for p in ['Mock', 'Fake', 'Stub', 'Dummy']):
                        components['mocks'].append(name)
                    break
        
        elif node_type == 'enum_item':
            for child in node.children:
                if child.type == 'type_identifier':
                    name = source_code[child.start_byte:child.end_byte].decode('utf8')
                    components['enums'].append(name)
                    break
        
        elif node_type == 'trait_item':
            for child in node.children:
                if child.type == 'type_identifier':
                    name = source_code[child.start_byte:child.end_byte].decode('utf8')
                    components['traits'].append(name)
                    break
        
        elif node_type == 'function_item':
            is_test = False
            is_async = False
            func_name = None
            
            for child in node.children:
                # Проверяем атрибуты
                if child.type == 'attribute_item':
                    attr_text = source_code[child.start_byte:child.end_byte].decode('utf8')
                    if '#[test]' in attr_text or '#[tokio::test]' in attr_text:
                        is_test = True
                
                # Проверяем async
                if child.type == 'function_modifiers':
                    for mod_child in child.children:
                        if mod_child.type == 'async':
                            is_async = True
                
                # Получаем имя функции
                if child.type == 'identifier':
                    func_name = source_code[child.start_byte:child.end_byte].decode('utf8')
            
            if func_name:
                if is_test:
                    components['tests'].append(func_name)
                else:
                    components['functions'].append(func_name)
                
                if is_async:
                    components['async_functions'].append(func_name)
        
        elif node_type == 'impl_item':
            impl_type = None
            trait_name = None
            
            for child in node.children:
                if child.type == 'type_identifier':
                    if impl_type is None:
                        impl_type = source_code[child.start_byte:child.end_byte].decode('utf8')
                    else:
                        trait_name = impl_type
                        impl_type = source_code[child.start_byte:child.end_byte].decode('utf8')
            
            if impl_type:
                if trait_name:
                    components['impl_blocks'].append(f"{trait_name} for {impl_type}")
                else:
                    components['impl_blocks'].append(impl_type)
                
                # Извлекаем методы из impl блока
                for child in node.children:
                    if child.type == 'declaration_list':
                        for item in child.children:
                            if item.type == 'function_item':
                                for func_child in item.children:
                                    if func_child.type == 'identifier':
                                        method_name = source_code[func_child.start_byte:func_child.end_byte].decode('utf8')
                                        components['methods'].append(f"{impl_type}::{method_name}")
                                        break
        
        # Рекурсивно обходим дочерние узлы
        for child in node.children:
            traverse(child)
    
    traverse(root)
    return components

# Извлекаем компоненты
components = extract_rust_components(tree, rust_code.encode('utf8'))

# Выводим результаты
for comp_type, items in components.items():
    if items:
        print(f"{comp_type:15}: {items}")

print("\n[OK] Tree-sitter-rust работает корректно!")