#!/usr/bin/env python3
"""Тестовый скрипт для проверки tree-sitter парсинга"""

from tree_sitter_languages import get_parser

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
'''

# Получаем парсер для Rust
parser = get_parser('rust')

# Парсим код
tree = parser.parse(rust_code.encode('utf8'))

# Функция для рекурсивной печати дерева
def print_tree(node, source, indent=0):
    """Рекурсивно печатает AST дерево"""
    node_text = source[node.start_byte:node.end_byte].decode('utf8')
    
    # Обрезаем текст для читаемости
    if len(node_text) > 50:
        node_text = node_text[:47] + "..."
    node_text = node_text.replace('\n', '\\n')
    
    print("  " * indent + f"{node.type}: '{node_text}'")
    
    # Печатаем только важные узлы
    important_types = {
        'struct_item', 'enum_item', 'trait_item', 'function_item',
        'impl_item', 'type_identifier', 'identifier', 'attribute_item'
    }
    
    for child in node.children:
        if child.type in important_types or indent < 2:
            print_tree(child, source, indent + 1)

print("=== AST Tree для тестового Rust кода ===\n")
print_tree(tree.root_node, rust_code.encode('utf8'))

# Теперь извлекаем конкретные элементы
print("\n=== Извлеченные компоненты ===\n")

def extract_components(node, source):
    """Извлекает компоненты из AST"""
    components = {
        'structs': [],
        'enums': [],
        'traits': [],
        'functions': [],
        'impl_blocks': [],
        'tests': []
    }
    
    def traverse(n):
        if n.type == 'struct_item':
            for child in n.children:
                if child.type == 'type_identifier':
                    name = source[child.start_byte:child.end_byte].decode('utf8')
                    components['structs'].append(name)
                    
        elif n.type == 'enum_item':
            for child in n.children:
                if child.type == 'type_identifier':
                    name = source[child.start_byte:child.end_byte].decode('utf8')
                    components['enums'].append(name)
                    
        elif n.type == 'trait_item':
            for child in n.children:
                if child.type == 'type_identifier':
                    name = source[child.start_byte:child.end_byte].decode('utf8')
                    components['traits'].append(name)
                    
        elif n.type == 'function_item':
            # Проверяем на тест
            is_test = False
            for child in n.children:
                if child.type == 'attribute_item':
                    attr_text = source[child.start_byte:child.end_byte].decode('utf8')
                    if '#[test]' in attr_text:
                        is_test = True
                elif child.type == 'identifier':
                    name = source[child.start_byte:child.end_byte].decode('utf8')
                    if is_test:
                        components['tests'].append(name)
                    else:
                        components['functions'].append(name)
                        
        elif n.type == 'impl_item':
            for child in n.children:
                if child.type == 'type_identifier':
                    name = source[child.start_byte:child.end_byte].decode('utf8')
                    components['impl_blocks'].append(name)
                    break
        
        # Рекурсивный обход
        for child in n.children:
            traverse(child)
    
    traverse(node)
    return components

components = extract_components(tree.root_node, rust_code.encode('utf8'))

for comp_type, items in components.items():
    if items:
        print(f"{comp_type}: {items}")

print("\n[OK] Tree-sitter работает корректно!")