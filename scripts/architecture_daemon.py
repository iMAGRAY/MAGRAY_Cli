#!/usr/bin/env python3
"""
Ультракомпактный архитектурный демон для автогенерации минималистичной Mermaid диаграммы

ЦЕЛЬ: Заменить огромные CTL аннотации одной краткой, но точной Mermaid диаграммой архитектуры.

Автор: AI Architecture Daemon
Версия: 1.0
"""

import os
import json
import time
import argparse
import toml
from pathlib import Path
from datetime import datetime
from typing import Dict, List, Set, Tuple, Optional
import re
from watchdog.observers import Observer
from watchdog.events import FileSystemEventHandler
import threading
import sys
import ast
import hashlib
from collections import defaultdict, Counter
import math

# Tree-sitter imports для реального AST парсинга
try:
    import tree_sitter
    from tree_sitter_rust import language as rust_language
    TREE_SITTER_AVAILABLE = True
except ImportError:
    TREE_SITTER_AVAILABLE = False
    print("[WARNING] tree-sitter или tree-sitter-rust не установлены. Используется fallback на regex парсинг.")

class ArchitectureDaemon:
    """Ультракомпактный демон для генерации Mermaid диаграмм архитектуры"""
    
    def __init__(self, project_root: str):
        self.project_root = Path(project_root)
        self.crates_dir = self.project_root / "crates"
        self.claude_md = self.project_root / "CLAUDE.md"
        self.dependencies: Dict[str, Set[str]] = {}
        self.features: Dict[str, List[str]] = {}
        self.file_structures: Dict[str, Dict] = {}  # Детальная структура файлов
        self.imports_map: Dict[str, Set[str]] = {}  # Карта импортов между файлами
        self.duplicates: Dict[str, List[Tuple[str, str]]] = {}  # Карта дубликатов
        self.mocks_registry: Dict[str, List[str]] = {}  # Реестр всех моков
        self.test_utilities: Dict[str, List[str]] = {}  # Test helpers и builders
        self.complexity_metrics: Dict[str, Dict] = {}  # Метрики сложности
        self.tech_debt: List[Dict] = []  # Технический долг
        self.dependency_graph: Dict[str, Set[str]] = {}  # Граф зависимостей
        self.file_cache: Dict[str, Dict] = {}  # Кэш для инкрементального анализа
        self.file_hashes: Dict[str, str] = {}  # Хэши файлов
        self.architectural_issues: Dict[str, List[Dict]] = {}  # Архитектурные проблемы
        self.ast_cache: Dict[str, tuple] = {}  # Кэш AST деревьев (hash -> (tree, result))
        self.circular_deps: List[List[str]] = []  # Найденные циклические зависимости
        
        # Инициализируем tree-sitter парсер для Rust
        self.rust_parser = None
        if TREE_SITTER_AVAILABLE:
            try:
                # Создаем Language объект и парсер
                lang = tree_sitter.Language(rust_language())
                self.rust_parser = tree_sitter.Parser(lang)
                print("[INFO] Tree-sitter парсер для Rust инициализирован")
            except Exception as e:
                print(f"[WARNING] Не удалось инициализировать tree-sitter: {e}")
                self.rust_parser = None
        
    def scan_architecture(self) -> Dict[str, any]:
        """Сканирует структуру проекта и определяет реальную архитектуру"""
        print("[INFO] Глубокий анализ архитектуры проекта...")
        
        architecture = {
            "crates": {},
            "dependencies": {},
            "features": {},
            "file_structures": {},
            "imports": {},
            "mermaid": ""
        }
        
        # Сканируем все crates
        for cargo_toml in self.crates_dir.rglob("Cargo.toml"):
            crate_name = cargo_toml.parent.name
            try:
                with open(cargo_toml, 'r', encoding='utf-8') as f:
                    content = f.read()
                    # Убираем BOM если есть
                    if content.startswith('\ufeff'):
                        content = content[1:]
                    cargo_data = toml.loads(content)
                
                # Извлекаем зависимости
                deps = set()
                if 'dependencies' in cargo_data:
                    for dep_name, dep_config in cargo_data['dependencies'].items():
                        if isinstance(dep_config, dict) and 'path' in dep_config:
                            # Локальная зависимость
                            path_parts = dep_config['path'].split('/')
                            if len(path_parts) >= 2 and path_parts[-2] == '..':
                                deps.add(path_parts[-1])
                
                # Извлекаем features
                features = []
                if 'features' in cargo_data:
                    features = list(cargo_data['features'].keys())
                
                # Сканируем Rust файлы в крейте
                rust_files = self._scan_rust_files(cargo_toml.parent)
                
                architecture["crates"][crate_name] = {
                    "path": str(cargo_toml.parent),
                    "description": self._get_crate_description(crate_name),
                    "files": rust_files
                }
                architecture["dependencies"][crate_name] = list(deps)
                architecture["features"][crate_name] = features
                architecture["file_structures"][crate_name] = rust_files
                
                print(f"  [OK] {crate_name}: {len(deps)} deps, {len(features)} features, {len(rust_files)} files")
                
            except Exception as e:
                print(f"  [ERROR] Ошибка при парсинге {cargo_toml}: {e}")
                continue
        
        # Генерируем Mermaid диаграмму
        architecture["mermaid"] = self._generate_mermaid(architecture)
        
        return architecture
    
    def _get_crate_description(self, crate_name: str) -> str:
        """Возвращает краткое описание crate"""
        descriptions = {
            "cli": "CLI Agent & Commands",
            "memory": "3-Layer HNSW Memory",
            "ai": "AI/ONNX Models & GPU",
            "llm": "Multi-Provider LLM",
            "router": "Smart Task Router",
            "tools": "Tools Registry",
            "common": "Common Utilities",
            "todo": "Task DAG System"
        }
        return descriptions.get(crate_name, f"{crate_name.title()} Crate")
    
    def _calculate_tech_debt(self) -> List[Dict]:
        """Вычисляет технический долг в человеко-часах"""
        debt_items = []
        
        for file_path, metrics in self.complexity_metrics.items():
            cyclomatic = metrics.get('cyclomatic', 0)
            cognitive = metrics.get('cognitive', 0)
            god_score = metrics.get('god_object_score', 0)
            
            # Высокая цикломатическая сложность
            if cyclomatic > 20:
                hours = min((cyclomatic - 20) * 0.5, 16)  # Максимум 16 часов на файл
                debt_items.append({
                    'file': file_path,
                    'type': 'high_cyclomatic_complexity',
                    'severity': 'critical' if cyclomatic > 30 else 'high',
                    'current_value': cyclomatic,
                    'target_value': 10,
                    'estimated_hours': hours,
                    'description': f'Цикломатическая сложность {cyclomatic} (должна быть < 10)'
                })
            
            # Высокая когнитивная сложность  
            if cognitive > 30:
                hours = min((cognitive - 30) * 0.25, 12)  # Максимум 12 часов на файл
                debt_items.append({
                    'file': file_path,
                    'type': 'high_cognitive_complexity',
                    'severity': 'high' if cognitive > 50 else 'medium',
                    'current_value': cognitive,
                    'target_value': 15,
                    'estimated_hours': hours,
                    'description': f'Когнитивная сложность {cognitive} (должна быть < 15)'
                })
            
            # God Object
            if god_score > 0.7:
                debt_items.append({
                    'file': file_path,
                    'type': 'god_object',
                    'severity': 'critical',
                    'current_value': god_score,
                    'target_value': 0.3,
                    'estimated_hours': 16,  # 2 дня на декомпозицию
                    'description': f'God Object вероятность {god_score:.0%}'
                })
        
        # Дубликаты кода
        duplicates = self._analyze_duplicates()
        for sig, locations in duplicates.items():
            if len(locations) > 3:
                debt_items.append({
                    'file': ', '.join([loc[0] for loc in locations[:3]]),
                    'type': 'code_duplication',
                    'severity': 'medium',
                    'current_value': len(locations),
                    'target_value': 1,
                    'estimated_hours': len(locations) * 2,
                    'description': f'Дублирование "{sig}" в {len(locations)} местах'
                })
        
        return sorted(debt_items, key=lambda x: ('critical', 'high', 'medium').index(x['severity']))
    
    def _should_analyze_file(self, file_path: Path) -> bool:
        """Проверяет, нужно ли анализировать файл (инкрементальный анализ)"""
        try:
            # Вычисляем хэш файла
            with open(file_path, 'rb') as f:
                current_hash = hashlib.md5(f.read()).hexdigest()
            
            # Проверяем кэш
            file_key = str(file_path)
            if file_key in self.file_hashes:
                if self.file_hashes[file_key] == current_hash:
                    # Файл не изменился, используем кэш
                    return False
            
            # Сохраняем новый хэш
            self.file_hashes[file_key] = current_hash
            return True
            
        except Exception:
            return True  # В случае ошибки анализируем
    
    def _build_dependency_graph(self, crate_name: str, uses: List[str], file_path: str = None):
        """Строит граф зависимостей между модулями с учетом AST данных"""
        source = file_path if file_path else f"{crate_name}"
        
        # Обрабатываем локальные и внешние зависимости
        local_deps = set()
        external_deps = set()
        
        for use_stmt in uses:
            use_clean = use_stmt.strip()
            
            # Извлекаем импортируемый модуль
            if 'crate::' in use_clean:
                parts = use_clean.replace('crate::', '').split('::')
                if parts:
                    # Локальная зависимость внутри крейта
                    local_deps.add(parts[0])
                    target = f"{crate_name}/{parts[0]}"
                    if source not in self.dependency_graph:
                        self.dependency_graph[source] = set()
                    self.dependency_graph[source].add(target)
                    
            elif 'super::' in use_clean:
                # Зависимость от родительского модуля
                parts = use_clean.replace('super::', '').split('::')
                if parts:
                    local_deps.add(f"../{parts[0]}")
                    
            elif 'self::' in use_clean:
                # Зависимость от текущего модуля
                parts = use_clean.replace('self::', '').split('::')
                if parts:
                    local_deps.add(f"./{parts[0]}")
                    
            elif '::' in use_clean:
                # Внешняя зависимость
                parts = use_clean.split('::')
                if parts:
                    crate_name_dep = parts[0]
                    external_deps.add(crate_name_dep)
                    
                    # Добавляем в граф
                    if source not in self.dependency_graph:
                        self.dependency_graph[source] = set()
                    self.dependency_graph[source].add(crate_name_dep)
            else:
                # Простой импорт (std библиотека или локальный модуль)
                if use_clean in ['std', 'core', 'alloc']:
                    external_deps.add(use_clean)
                else:
                    local_deps.add(use_clean)
        
        # Детектируем циклические зависимости
        self._detect_circular_deps(source)
        
        return local_deps, external_deps
    
    def _detect_circular_deps(self, source: str):
        """Детектирует циклические зависимости для данного источника"""
        if source not in self.dependency_graph:
            return
        
        visited = set()
        stack = []
        
        def dfs(node, path):
            if node in path:
                # Нашли цикл
                cycle = path[path.index(node):] + [node]
                if len(cycle) > 1:
                    self.circular_deps.append(cycle)
                return
            
            if node in visited:
                return
                
            visited.add(node)
            path.append(node)
            
            if node in self.dependency_graph:
                for neighbor in self.dependency_graph[node]:
                    dfs(neighbor, path[:])
        
        dfs(source, [])
    
    def _detect_circular_dependencies(self) -> List[List[str]]:
        """Находит все циклические зависимости в графе"""
        cycles = []
        visited = set()
        rec_stack = set()
        
        def dfs(node, path):
            if node in rec_stack:
                # Нашли цикл
                cycle_start = path.index(node)
                cycles.append(path[cycle_start:])
                return
            
            if node in visited:
                return
                
            visited.add(node)
            rec_stack.add(node)
            path.append(node)
            
            if node in self.dependency_graph:
                for neighbor in self.dependency_graph[node]:
                    dfs(neighbor, path[:])
            
            rec_stack.remove(node)
        
        for node in self.dependency_graph:
            if node not in visited:
                dfs(node, [])
        
        return cycles
    
    def _parse_rust_with_ast(self, content: str, file_path: str) -> Dict:
        """Парсит Rust код используя tree-sitter AST для точного анализа с кэшированием"""
        if not self.rust_parser or not TREE_SITTER_AVAILABLE:
            # Fallback на regex парсинг
            return self._parse_rust_with_regex(content, file_path)
        
        # Проверяем кэш AST
        import hashlib
        content_hash = hashlib.md5(content.encode()).hexdigest()
        
        if content_hash in self.ast_cache:
            # Возвращаем закэшированный результат
            _, cached_result = self.ast_cache[content_hash]
            return cached_result
        
        try:
            # Парсим AST дерево
            tree = self.rust_parser.parse(content.encode('utf8'))
            root_node = tree.root_node
            
            # Результаты парсинга
            result = {
                'structs': [],
                'enums': [],
                'traits': [],
                'functions': [],
                'methods': [],
                'impl_blocks': [],
                'macros': [],
                'type_aliases': [],
                'constants': [],
                'statics': [],
                'uses': [],
                'mods': [],
                'tests': [],
                'mocks': [],
                'async_functions': [],
                'generics': [],
                'lifetimes': [],
                'attributes': []
            }
            
            # Рекурсивный обход AST дерева
            self._traverse_ast_node(root_node, content.encode('utf8'), result)
            
            # Дополнительный анализ для моков и тестов
            self._analyze_mocks_from_ast(result)
            self._analyze_tests_from_ast(result)
            
            # Сохраняем в кэш для повторного использования
            self.ast_cache[content_hash] = (tree, result)
            
            # Ограничиваем размер кэша
            if len(self.ast_cache) > 100:
                # Удаляем старые записи (простая FIFO стратегия)
                oldest_key = next(iter(self.ast_cache))
                del self.ast_cache[oldest_key]
            
            return result
            
        except Exception as e:
            print(f"[WARNING] Ошибка AST парсинга для {file_path}: {e}")
            # Fallback на regex
            return self._parse_rust_with_regex(content, file_path)
    
    def _traverse_ast_node(self, node, source_code: bytes, result: Dict, depth=0):
        """Рекурсивно обходит AST узлы и извлекает информацию"""
        if depth > 100:  # Защита от слишком глубокой рекурсии
            return
            
        node_type = node.type
        
        # Структуры
        if node_type == 'struct_item':
            name_node = self._find_child_by_type(node, 'type_identifier')
            if name_node:
                struct_name = source_code[name_node.start_byte:name_node.end_byte].decode('utf8')
                result['structs'].append(struct_name)
                
                # Проверяем на God Object (много полей)
                field_count = len([c for c in node.children if c.type == 'field_declaration'])
                if field_count > 10:
                    result.setdefault('god_objects', []).append((struct_name, field_count))
        
        # Перечисления
        elif node_type == 'enum_item':
            name_node = self._find_child_by_type(node, 'type_identifier')
            if name_node:
                result['enums'].append(source_code[name_node.start_byte:name_node.end_byte].decode('utf8'))
        
        # Трейты
        elif node_type == 'trait_item':
            name_node = self._find_child_by_type(node, 'type_identifier')
            if name_node:
                result['traits'].append(source_code[name_node.start_byte:name_node.end_byte].decode('utf8'))
        
        # Функции
        elif node_type == 'function_item':
            name_node = self._find_child_by_type(node, 'identifier')
            if name_node:
                func_name = source_code[name_node.start_byte:name_node.end_byte].decode('utf8')
                result['functions'].append(func_name)
                
                # Проверяем async
                if self._has_child_type(node, 'async'):
                    result['async_functions'].append(func_name)
                
                # Проверяем на тест
                if self._has_test_attribute(node, source_code):
                    result['tests'].append(func_name)
        
        # Impl блоки
        elif node_type == 'impl_item':
            type_node = self._find_child_by_type(node, 'type_identifier')
            if type_node:
                impl_type = source_code[type_node.start_byte:type_node.end_byte].decode('utf8')
                result['impl_blocks'].append(impl_type)
                
                # Извлекаем методы из impl блока
                decl_list = self._find_child_by_type(node, 'declaration_list')
                if decl_list:
                    for child in decl_list.children:
                        if child.type == 'function_item':
                            method_name_node = self._find_child_by_type(child, 'identifier')
                            if method_name_node:
                                method_name = source_code[method_name_node.start_byte:method_name_node.end_byte].decode('utf8')
                                result['methods'].append(f"{impl_type}::{method_name}")
        
        # Макросы
        elif node_type == 'macro_definition':
            name_node = self._find_child_by_type(node, 'identifier')
            if name_node:
                result['macros'].append(source_code[name_node.start_byte:name_node.end_byte].decode('utf8'))
        
        # Type aliases
        elif node_type == 'type_alias':
            name_node = self._find_child_by_type(node, 'type_identifier')
            if name_node:
                result['type_aliases'].append(source_code[name_node.start_byte:name_node.end_byte].decode('utf8'))
        
        # Константы
        elif node_type == 'const_item':
            name_node = self._find_child_by_type(node, 'identifier')
            if name_node:
                result['constants'].append(source_code[name_node.start_byte:name_node.end_byte].decode('utf8'))
        
        # Статические переменные
        elif node_type == 'static_item':
            name_node = self._find_child_by_type(node, 'identifier')
            if name_node:
                result['statics'].append(source_code[name_node.start_byte:name_node.end_byte].decode('utf8'))
        
        # Use statements
        elif node_type == 'use_declaration':
            use_text = source_code[node.start_byte:node.end_byte].decode('utf8')
            use_text = use_text.replace('use ', '').replace(';', '').strip()
            result['uses'].append(use_text)
        
        # Модули
        elif node_type == 'mod_item':
            name_node = self._find_child_by_type(node, 'identifier')
            if name_node:
                result['mods'].append(source_code[name_node.start_byte:name_node.end_byte].decode('utf8'))
        
        # Атрибуты (для обнаружения mockall и других)
        elif node_type == 'attribute_item':
            attr_text = source_code[node.start_byte:node.end_byte].decode('utf8')
            result['attributes'].append(attr_text)
            
            # Проверяем на mock атрибуты
            if 'mock' in attr_text.lower() or 'automock' in attr_text.lower():
                result.setdefault('mock_attributes', []).append(attr_text)
        
        # Unsafe блоки
        elif node_type == 'unsafe_block':
            result.setdefault('unsafe_blocks', []).append({
                'line': node.start_point[0] + 1,
                'code': source_code[node.start_byte:node.end_byte].decode('utf8')[:50] + '...'
            })
        
        # Associated types в trait
        elif node_type == 'associated_type':
            name_node = self._find_child_by_type(node, 'type_identifier')
            if name_node:
                result.setdefault('associated_types', []).append(
                    source_code[name_node.start_byte:name_node.end_byte].decode('utf8')
                )
        
        # Generic параметры
        elif node_type == 'type_parameters':
            params_text = source_code[node.start_byte:node.end_byte].decode('utf8')
            result.setdefault('generics', []).append(params_text)
        
        # Lifetime параметры
        elif node_type == 'lifetime':
            lifetime_text = source_code[node.start_byte:node.end_byte].decode('utf8')
            result.setdefault('lifetimes', []).append(lifetime_text)
        
        # Обнаружение моков/стабов/фейков по имени
        if node_type in ['struct_item', 'function_item']:
            name_node = self._find_child_by_type(node, 'identifier') or self._find_child_by_type(node, 'type_identifier')
            if name_node:
                name = source_code[name_node.start_byte:name_node.end_byte].decode('utf8')
                name_lower = name.lower()
                
                # Паттерны для обнаружения моков/стабов
                if any(pattern in name_lower for pattern in ['mock', 'stub', 'fake', 'dummy', 'spy']):
                    result.setdefault('test_doubles', []).append({
                        'name': name,
                        'type': 'mock' if 'mock' in name_lower else 
                                'stub' if 'stub' in name_lower else
                                'fake' if 'fake' in name_lower else
                                'dummy' if 'dummy' in name_lower else 'spy',
                        'node_type': node_type
                    })
        
        # Рекурсивно обходим дочерние узлы
        for child in node.children:
            self._traverse_ast_node(child, source_code, result, depth + 1)
    
    def _find_child_by_type(self, node, type_name: str):
        """Находит дочерний узел по типу"""
        for child in node.children:
            if child.type == type_name:
                return child
        return None
    
    def _has_child_type(self, node, type_name: str) -> bool:
        """Проверяет наличие дочернего узла определенного типа"""
        return any(child.type == type_name for child in node.children)
    
    def _has_test_attribute(self, node, source_code: bytes) -> bool:
        """Проверяет, есть ли у функции тестовый атрибут"""
        # Ищем атрибуты перед функцией
        for child in node.children:
            if child.type == 'attribute_item':
                attr_text = source_code[child.start_byte:child.end_byte].decode('utf8')
                if '#[test]' in attr_text or '#[tokio::test]' in attr_text:
                    return True
        return False
    
    def _analyze_mocks_from_ast(self, ast_result: Dict):
        """Анализирует AST результаты для обнаружения моков"""
        mocks = []
        
        # Ищем структуры с Mock/Fake/Stub в названии
        for struct_name in ast_result.get('structs', []):
            if any(pattern in struct_name for pattern in ['Mock', 'Fake', 'Stub', 'Dummy', 'Test']):
                mocks.append(struct_name)
        
        # Ищем impl блоки для моков
        for impl_type in ast_result.get('impl_blocks', []):
            if any(pattern in impl_type for pattern in ['Mock', 'Fake', 'Stub']):
                mocks.append(f"impl {impl_type}")
        
        # Проверяем атрибуты mockall
        for attr in ast_result.get('attributes', []):
            if 'mockall' in attr or 'automock' in attr:
                mocks.append("[uses mockall]")
        
        ast_result['mocks'] = mocks
    
    def _analyze_tests_from_ast(self, ast_result: Dict):
        """Анализирует AST для подсчета и категоризации тестов"""
        test_count = len(ast_result.get('tests', []))
        
        # Категоризируем тесты
        unit_tests = [t for t in ast_result.get('tests', []) if not t.startswith('test_integration')]
        integration_tests = [t for t in ast_result.get('tests', []) if t.startswith('test_integration')]
        
        ast_result['test_stats'] = {
            'total': test_count,
            'unit': len(unit_tests),
            'integration': len(integration_tests)
        }
    
    def _parse_rust_with_regex(self, content: str, file_path: str) -> Dict:
        """Fallback метод парсинга с использованием regex (старый метод)"""
        # Извлекаем компоненты с помощью regex
        return {
            'structs': re.findall(r'^(?:pub\s+)?struct (\w+)', content, re.MULTILINE),
            'enums': re.findall(r'^(?:pub\s+)?enum (\w+)', content, re.MULTILINE),
            'traits': re.findall(r'^(?:pub\s+)?trait (\w+)', content, re.MULTILINE),
            'functions': re.findall(r'^(?:pub\s+)?(?:async\s+)?fn\s+(\w+)', content, re.MULTILINE),
            'methods': [],  # Будут заполнены отдельно
            'impl_blocks': [],
            'macros': re.findall(r'^macro_rules!\s+(\w+)', content, re.MULTILINE),
            'type_aliases': re.findall(r'^(?:pub\s+)?type\s+(\w+)', content, re.MULTILINE),
            'constants': re.findall(r'^(?:pub\s+)?const\s+(\w+):', content, re.MULTILINE),
            'statics': re.findall(r'^(?:pub\s+)?static\s+(\w+):', content, re.MULTILINE),
            'uses': re.findall(r'^use\s+([^;]+);', content, re.MULTILINE),
            'mods': re.findall(r'^(?:pub\s+)?mod\s+(\w+)', content, re.MULTILINE),
            'tests': re.findall(r'#\[test\]\s*(?:async\s+)?fn\s+(\w+)', content),
            'mocks': [],
            'async_functions': re.findall(r'async\s+fn\s+(\w+)', content)
        }
    
    def _scan_rust_files(self, crate_path: Path) -> Dict[str, Dict]:
        """Сканирует все Rust файлы в крейте используя AST парсинг"""
        files_info = {}
        
        for rust_file in crate_path.rglob("*.rs"):
            # Пропускаем target
            if 'target' in rust_file.parts:
                continue
                
            relative_path = rust_file.relative_to(crate_path)
            file_key = str(relative_path).replace('\\', '/')
            crate_name = crate_path.name
            
            # Определяем тип файла
            is_test = 'test' in file_key or file_key.startswith('tests/')
            is_example = file_key.startswith('examples/')
            is_bench = file_key.startswith('benches/')
            is_mock = 'mock' in file_key.lower()
            is_common = 'common' in file_key or 'utils' in file_key or 'helpers' in file_key
            
            # Инкрементальный анализ - пропускаем неизмененные файлы
            if not self._should_analyze_file(rust_file) and file_key in self.file_cache:
                files_info[file_key] = self.file_cache[file_key]
                continue
            
            try:
                with open(rust_file, 'r', encoding='utf-8') as f:
                    content = f.read()
                
                # Используем AST парсинг если доступен
                full_file_path = f"{crate_name}/{file_key}"
                ast_result = self._parse_rust_with_ast(content, full_file_path)
                
                # Извлекаем компоненты из AST результата
                structs = ast_result.get('structs', [])
                enums = ast_result.get('enums', [])
                traits = ast_result.get('traits', [])
                functions = ast_result.get('functions', [])
                methods = ast_result.get('methods', [])
                uses = ast_result.get('uses', [])
                mocks = ast_result.get('mocks', [])
                tests = ast_result.get('tests', [])
                async_fns = ast_result.get('async_functions', [])
                macros = ast_result.get('macros', [])
                type_aliases = ast_result.get('type_aliases', [])
                consts = ast_result.get('constants', [])
                statics = ast_result.get('statics', [])
                
                # Новые поля из улучшенного AST
                unsafe_blocks = ast_result.get('unsafe_blocks', [])
                associated_types = ast_result.get('associated_types', [])
                test_doubles = ast_result.get('test_doubles', [])
                god_objects = ast_result.get('god_objects', [])
                
                # Дополнительный анализ для тестовых утилит и моков
                test_builders = []
                mock_impls = []
                
                # Дополнительный поиск mock impl если не найдены через AST
                if not mocks and ('mock' in file_key.lower() or 'fake' in file_key.lower()):
                    mock_impl_pattern = r'impl(?:<[^>]+>)?\s+\w+\s+for\s+(Mock\w+|Fake\w+|Stub\w+)'
                    mock_impls = re.findall(mock_impl_pattern, content)
                    mocks.extend(mock_impls)
                
                # Ищем Test builders и helpers
                if is_test or is_common:
                    test_builders = re.findall(r'(?:pub\s+)?struct\s+(\w*(?:Builder|Helper|Generator|Factory|Fixture)\w*)', content)
                    
                    # Сохраняем тестовые утилиты глобально
                    if test_builders:
                        full_path = f"{crate_name}/{file_key}"
                        if crate_name not in self.test_utilities:
                            self.test_utilities[crate_name] = []
                        self.test_utilities[crate_name].extend([(full_path, tb) for tb in test_builders])
                
                # Регистрируем все найденные моки глобально
                if mocks or mock_impls:
                    full_path = f"{crate_name}/{file_key}"
                    if crate_name not in self.mocks_registry:
                        self.mocks_registry[crate_name] = []
                    self.mocks_registry[crate_name].extend([(full_path, m) for m in mocks + mock_impls])
                
                # Определяем тестовые функции из AST результата
                test_fns = tests[:3] if tests else []
                
                # Строим граф зависимостей
                local_deps, external_deps = self._build_dependency_graph(crate_name, uses, full_file_path)
                
                # Вычисляем метрики сложности
                cyclomatic = self._calculate_cyclomatic_complexity(content)
                cognitive = self._calculate_cognitive_complexity(content)
                
                # Подсчитываем поля структур
                fields_count = len(re.findall(r'^\s+(pub\s+)?\w+:\s+', content, re.MULTILINE))
                
                # Определяем God Object
                god_score = self._detect_god_object(structs, methods, fields_count)
                
                # Сохраняем метрики
                full_file_path = f"{crate_name}/{file_key}"
                self.complexity_metrics[full_file_path] = {
                    'cyclomatic': cyclomatic,
                    'cognitive': cognitive,
                    'god_object_score': god_score,
                    'loc': len(content.splitlines()),
                    'methods': len(methods),
                    'fields': fields_count
                }
                
                # Анализ дубликатов - собираем сигнатуры impl блоков
                impl_signatures = []
                for trait_or_type in re.findall(r'impl(?:<[^>]+>)?\s+(?:(\w+)\s+for\s+)?(\w+)', content):
                    if trait_or_type[0]:  # trait impl
                        sig = f"impl {trait_or_type[0]} for {trait_or_type[1]}"
                    else:  # direct impl
                        sig = f"impl {trait_or_type[1]}"
                    impl_signatures.append(sig)
                    
                    # Регистрируем потенциальные дубликаты
                    if sig not in self.duplicates:
                        self.duplicates[sig] = []
                    self.duplicates[sig].append((f"{crate_name}/{file_key}", trait_or_type[1]))
                
                file_info = {
                    "structs": structs[:4],
                    "enums": enums[:3],
                    "traits": traits[:3],
                    "methods": methods[:4],
                    "functions": functions[:4],
                    "async_fns": async_fns[:3],
                    "consts": consts[:2],
                    "statics": statics[:2],
                    "macros": macros[:2],
                    "types": type_aliases[:2],
                    "mocks": mocks[:3],
                    "test_fns": test_fns[:3],
                    "test_builders": test_builders[:2],
                    "test_doubles": test_doubles[:3],
                    "unsafe_blocks": len(unsafe_blocks),
                    "associated_types": associated_types[:2],
                    "god_objects": god_objects[:2],
                    "impl_sigs": impl_signatures[:3],
                    "uses": [u.strip() for u in uses[:3]],
                    "loc": len(content.splitlines()),
                    "is_test": is_test,
                    "is_example": is_example,
                    "is_bench": is_bench,
                    "is_mock": is_mock or len(mocks) > 0 or len(test_doubles) > 0,
                    "is_common": is_common,
                    "complexity": {
                        "cyclomatic": cyclomatic,
                        "cognitive": cognitive,
                        "god_score": god_score
                    }
                }
                
                # Сохраняем в кэш и результаты
                files_info[file_key] = file_info
                self.file_cache[file_key] = file_info
                
            except Exception as e:
                # Игнорируем ошибки чтения
                pass
                
        return files_info
    
    def _calculate_cyclomatic_complexity(self, content: str) -> int:
        """Вычисляет цикломатическую сложность кода"""
        complexity = 1  # Базовая сложность
        
        # Подсчет точек ветвления
        complexity += len(re.findall(r'\bif\b', content))
        complexity += len(re.findall(r'\belse\s+if\b', content))
        complexity += len(re.findall(r'\bmatch\b', content))
        complexity += len(re.findall(r'\bfor\b', content))
        complexity += len(re.findall(r'\bwhile\b', content))
        complexity += len(re.findall(r'\b\?\b', content))  # Тернарный оператор
        complexity += len(re.findall(r'\b&&\b', content))
        complexity += len(re.findall(r'\b\|\|\b', content))
        complexity += len(re.findall(r'=>', content)) // 2  # match arms
        
        return complexity
    
    def _calculate_cognitive_complexity(self, content: str) -> int:
        """Вычисляет когнитивную сложность (более точная метрика)"""
        cognitive = 0
        nesting_level = 0
        
        lines = content.split('\n')
        for line in lines:
            # Увеличиваем уровень вложенности
            if re.search(r'\b(if|for|while|match)\b.*\{', line):
                nesting_level += 1
                cognitive += nesting_level
            elif '{' in line:
                nesting_level += 1
            elif '}' in line:
                nesting_level = max(0, nesting_level - 1)
            
            # Добавляем сложность для логических операторов
            cognitive += len(re.findall(r'\b(&&|\|\|)\b', line)) * (nesting_level + 1)
            
        return cognitive
    
    def _detect_god_object(self, structs: List[str], methods: List[str], fields_count: int = 0) -> float:
        """Определяет вероятность God Object антипаттерна"""
        if not structs:
            return 0.0
        
        # Базовые метрики
        method_count = len(methods)
        struct_count = len(structs)
        
        # God Object индикаторы
        god_score = 0.0
        
        if method_count > 20:
            god_score += 0.3
        elif method_count > 15:
            god_score += 0.2
        elif method_count > 10:
            god_score += 0.1
            
        if fields_count > 15:
            god_score += 0.3
        elif fields_count > 10:
            god_score += 0.2
        elif fields_count > 7:
            god_score += 0.1
            
        # Проверяем имена на признаки God Object
        for struct in structs:
            if any(word in struct.lower() for word in ['manager', 'controller', 'handler', 'service', 'unified']):
                god_score += 0.2
                break
        
        # Слишком много ответственностей
        if method_count > 10 and fields_count > 5:
            god_score += 0.2
            
        return min(1.0, god_score)
    
    def _analyze_duplicates(self) -> Dict[str, List]:
        """Анализирует найденные дубликаты и возвращает отчет"""
        duplicate_report = {}
        
        for sig, locations in self.duplicates.items():
            if len(locations) > 1:
                # Есть дубликаты
                duplicate_report[sig] = locations
        
        return duplicate_report
    
    def _generate_analysis_report(self) -> str:
        """Генерирует отчет об анализе дубликатов, моков и тестовых утилит"""
        report_lines = []
        
        # Анализ технического долга
        self.tech_debt = self._calculate_tech_debt()
        if self.tech_debt:
            report_lines.append("## 💸 Технический долг\n")
            
            total_hours = sum(item['estimated_hours'] for item in self.tech_debt)
            critical_count = sum(1 for item in self.tech_debt if item['severity'] == 'critical')
            high_count = sum(1 for item in self.tech_debt if item['severity'] == 'high')
            
            report_lines.append(f"**Общий долг**: {total_hours:.1f} часов ({total_hours/8:.1f} дней)")
            report_lines.append(f"**Критических проблем**: {critical_count}")
            report_lines.append(f"**Высокий приоритет**: {high_count}\n")
            
            # Топ-5 критических проблем
            for item in self.tech_debt[:5]:
                report_lines.append(f"- [{item['severity'].upper()}] {item['description']}")
                report_lines.append(f"  - Файл: `{item['file']}`")
                report_lines.append(f"  - Оценка: {item['estimated_hours']:.1f} часов")
            report_lines.append("")
        
        # Анализ сложности
        if self.complexity_metrics:
            report_lines.append("## 📊 Метрики сложности\n")
            
            # Файлы с наибольшей сложностью
            complex_files = sorted(
                [(path, m) for path, m in self.complexity_metrics.items()],
                key=lambda x: x[1]['cyclomatic'],
                reverse=True
            )[:5]
            
            report_lines.append("### Самые сложные файлы:")
            for path, metrics in complex_files:
                if metrics['cyclomatic'] > 10:
                    report_lines.append(f"- `{path}`:")
                    report_lines.append(f"  - Цикломатическая: {metrics['cyclomatic']}")
                    report_lines.append(f"  - Когнитивная: {metrics['cognitive']}")
                    if metrics['god_object_score'] > 0.5:
                        report_lines.append(f"  - ⚠️ God Object: {metrics['god_object_score']:.0%}")
            report_lines.append("")
        
        # Циклические зависимости
        cycles = self._detect_circular_dependencies()
        if cycles:
            report_lines.append("## 🔄 Циклические зависимости\n")
            for cycle in cycles[:5]:
                report_lines.append(f"- {' → '.join(cycle)} → {cycle[0]}")
            report_lines.append("")
        
        # Анализ дубликатов
        duplicates = self._analyze_duplicates()
        if duplicates:
            report_lines.append("## 🔍 Обнаруженные дубликаты\n")
            for sig, locations in sorted(duplicates.items())[:10]:  # Топ-10 дубликатов
                report_lines.append(f"- **{sig}** встречается {len(locations)} раз:")
                for path, name in locations[:3]:
                    report_lines.append(f"  - `{path}` ({name})")
                if len(locations) > 3:
                    report_lines.append(f"  - ...и еще {len(locations)-3} мест")
            report_lines.append("")
        
        # Анализ моков
        if self.mocks_registry:
            report_lines.append("## 🎭 Реестр моков и заглушек\n")
            total_mocks = sum(len(mocks) for mocks in self.mocks_registry.values())
            report_lines.append(f"Всего найдено моков: **{total_mocks}**\n")
            
            for crate, mocks in sorted(self.mocks_registry.items()):
                if mocks:
                    report_lines.append(f"### {crate}")
                    unique_mocks = {}
                    for path, mock_name in mocks:
                        if mock_name not in unique_mocks:
                            unique_mocks[mock_name] = []
                        unique_mocks[mock_name].append(path)
                    
                    for mock_name, paths in sorted(unique_mocks.items())[:5]:
                        report_lines.append(f"- `{mock_name}` в {paths[0]}")
            report_lines.append("")
        
        # Анализ тестовых утилит
        if self.test_utilities:
            report_lines.append("## 🛠️ Тестовые утилиты и билдеры\n")
            for crate, utilities in sorted(self.test_utilities.items()):
                if utilities:
                    report_lines.append(f"### {crate}")
                    unique_utils = {}
                    for path, util_name in utilities:
                        if util_name not in unique_utils:
                            unique_utils[util_name] = path
                    
                    for util_name, path in sorted(unique_utils.items())[:5]:
                        report_lines.append(f"- `{util_name}` в {path}")
            report_lines.append("")
        
        return "\n".join(report_lines) if report_lines else ""
    
    def _generate_mermaid(self, arch: Dict) -> str:
        """Генерирует детальную многоуровневую Mermaid диаграмму"""
        lines = [
            "```mermaid",
            "graph TB",
            ""
        ]
        
        # Создаем subgraph для каждого крейта
        for crate_name, crate_info in arch["crates"].items():
            crate_id = crate_name.upper()
            lines.append(f"    subgraph {crate_id}[{crate_info['description']}]")
            
            # Добавляем основные файлы крейта
            if "files" in crate_info and crate_info["files"]:
                # Группируем файлы по директориям
                files_by_dir = {}
                for file_path, file_info in crate_info["files"].items():
                    dir_name = os.path.dirname(file_path) if '/' in file_path else 'root'
                    if dir_name not in files_by_dir:
                        files_by_dir[dir_name] = []
                    files_by_dir[dir_name].append((file_path, file_info))
                
                # Добавляем файлы с их структурами
                for dir_name, files in files_by_dir.items():
                    for file_path, file_info in files[:8]:  # Увеличиваем лимит для большей детализации
                        file_name = os.path.basename(file_path).replace('.rs', '')
                        file_id = f"{crate_id}_{file_name.replace('-', '_').replace('/', '_')}"
                        
                        # Формируем детальное описание файла
                        components = []
                        
                        # Маркируем специальные файлы
                        if file_info.get('is_test'):
                            components.append("TEST")
                        if file_info.get('is_mock'):
                            components.append("MOCK")
                        if file_info.get('is_example'):
                            components.append("EXAMPLE")
                        if file_info.get('is_bench'):
                            components.append("BENCH")
                        
                        # Добавляем компоненты
                        if file_info.get('structs'):
                            components.append(f"S:{','.join(file_info['structs'][:2])}")
                        if file_info.get('traits'):
                            components.append(f"T:{','.join(file_info['traits'][:2])}")
                        if file_info.get('enums'):
                            components.append(f"E:{','.join(file_info['enums'][:2])}")
                        if file_info.get('functions'):
                            components.append(f"fn:{','.join(file_info['functions'][:2])}")
                        if file_info.get('methods'):
                            components.append(f"m:{','.join(file_info['methods'][:2])}")
                        if file_info.get('async_fns'):
                            components.append(f"async:{','.join(file_info['async_fns'][:2])}")
                        if file_info.get('macros'):
                            components.append(f"macro:{','.join(file_info['macros'][:1])}")
                        if file_info.get('mocks'):
                            mock_str = ','.join([m if isinstance(m, str) else str(m) for m in file_info['mocks'][:2]])
                            components.append(f"Mock:{mock_str}")
                        if file_info.get('test_doubles'):
                            test_doubles_str = ','.join([td['name'] for td in file_info['test_doubles'][:2]])
                            components.append(f"TestDouble:{test_doubles_str}")
                        if file_info.get('unsafe_blocks', 0) > 0:
                            components.append(f"unsafe:{file_info['unsafe_blocks']}")
                        if file_info.get('god_objects'):
                            god_str = ','.join([f"{go[0]}({go[1]})" for go in file_info['god_objects'][:1]])
                            components.append(f"GOD:{god_str}")
                        if file_info.get('test_fns'):
                            components.append(f"tests:{len(file_info['test_fns'])}")
                        
                        # Формируем label
                        if components:
                            # Разбиваем на строки для читаемости
                            if len(components) > 3:
                                label = f"{file_name}<br/>{'<br/>'.join(components[:3])}<br/>...+{len(components)-3}"
                            else:
                                label = f"{file_name}<br/>{'<br/>'.join(components)}"
                        else:
                            label = file_name
                        
                        # Определяем стиль узла
                        node_style = ""
                        if file_info.get('is_test'):
                            node_style = ":::testFile"
                        elif file_info.get('is_mock'):
                            node_style = ":::mockFile"
                        elif file_info.get('is_example'):
                            node_style = ":::exampleFile"
                        elif file_info.get('is_bench'):
                            node_style = ":::benchFile"
                            
                        lines.append(f"        {file_id}[{label}]{node_style}")
                        
            lines.append("    end")
            lines.append("")
        
        # Добавляем зависимости между крейтами
        lines.append("    %% Зависимости между крейтами")
        for crate_name, deps in arch["dependencies"].items():
            crate_id = crate_name.upper()
            for dep in deps:
                dep_id = dep.upper()
                if dep_id in [c.upper() for c in arch["crates"].keys()]:
                    lines.append(f"    {crate_id} -.->|uses| {dep_id}")
        
        lines.append("")
        
        # Стилизация
        lines.extend([
            "    classDef crate fill:#e3f2fd,stroke:#1976d2,stroke-width:2px",
            "    classDef file fill:#fff9c4,stroke:#f57c00,stroke-width:1px",
            "    classDef testFile fill:#ffebee,stroke:#c62828,stroke-width:1px,stroke-dasharray: 5 5",
            "    classDef mockFile fill:#fce4ec,stroke:#ad1457,stroke-width:1px,stroke-dasharray: 3 3",
            "    classDef exampleFile fill:#e8f5e9,stroke:#2e7d32,stroke-width:1px",
            "    classDef benchFile fill:#fff3e0,stroke:#e65100,stroke-width:1px",
            "    classDef trait fill:#f3e5f5,stroke:#7b1fa2,stroke-width:1px",
            "    classDef struct fill:#e8f5e9,stroke:#388e3c,stroke-width:1px"
        ])
        
        lines.append("```")
        return "\n".join(lines)
    
    def update_claude_md(self, architecture: Dict):
        """Обновляет секцию AUTO-GENERATED ARCHITECTURE в CLAUDE.md"""
        print("[INFO] Обновление CLAUDE.md...")
        
        if not self.claude_md.exists():
            print("[ERROR] CLAUDE.md не найден")
            return
        
        with open(self.claude_md, 'r', encoding='utf-8') as f:
            content = f.read()
        
        # Находим секцию AUTO-GENERATED ARCHITECTURE или создаем её
        start_marker = "# AUTO-GENERATED ARCHITECTURE"
        
        start_idx = content.find(start_marker)
        if start_idx == -1:
            # Добавляем секцию в конец файла
            print("[INFO] Создание новой секции AUTO-GENERATED ARCHITECTURE...")
            timestamp = datetime.now().strftime("%Y-%m-%d %H:%M:%S UTC")
            new_section = f"""

---

# AUTO-GENERATED ARCHITECTURE

*Last updated: {timestamp}*

## Компактная архитектура MAGRAY CLI

{architecture['mermaid']}

## Статистика проекта

- **Всего crates**: {len(architecture['crates'])}
- **Всего файлов**: {sum(len(c.get('files', {})) for c in architecture['crates'].values())}
- **Активные зависимости**: {sum(len(deps) for deps in architecture['dependencies'].values())}
- **Основные компоненты**: CLI, Memory (3-Layer HNSW), AI/ONNX, LLM Multi-Provider
- **GPU поддержка**: CUDA + TensorRT через feature flags
- **Производительность**: HNSW O(log n) search, SIMD оптимизации

## Ключевые особенности

- **Единый исполняемый файл**: `magray` (target ~16MB)
- **Conditional compilation**: cpu/gpu/minimal variants
- **Memory система**: 3 слоя (Interact/Insights/Assets) с HNSW индексами  
- **AI модели**: Qwen3 embeddings (1024D), BGE-M3 legacy support
- **LLM провайдеры**: OpenAI/Anthropic/Local
- **Production готовность**: Circuit breakers, health checks, metrics

{self._generate_analysis_report()}
"""
            
            new_content = content + new_section
            
            with open(self.claude_md, 'w', encoding='utf-8') as f:
                f.write(new_content)
            
            print(f"[OK] CLAUDE.md обновлен ({timestamp})")
            return
        
        # Если секция уже существует, обновляем её
        # Ищем следующий заголовок # или конец файла
        next_section_idx = content.find("\n# ", start_idx + len(start_marker))
        if next_section_idx == -1:
            end_idx = len(content)
        else:
            end_idx = next_section_idx
        
        # Генерируем обновленную секцию
        timestamp = datetime.now().strftime("%Y-%m-%d %H:%M:%S UTC")
        new_section = f"""# AUTO-GENERATED ARCHITECTURE

*Last updated: {timestamp}*

## Компактная архитектура MAGRAY CLI

{architecture['mermaid']}

## Статистика проекта

- **Всего crates**: {len(architecture['crates'])}
- **Всего файлов**: {sum(len(c.get('files', {})) for c in architecture['crates'].values())}
- **Активные зависимости**: {sum(len(deps) for deps in architecture['dependencies'].values())}
- **Основные компоненты**: CLI, Memory (3-Layer HNSW), AI/ONNX, LLM Multi-Provider
- **GPU поддержка**: CUDA + TensorRT через feature flags
- **Производительность**: HNSW O(log n) search, SIMD оптимизации

## Ключевые особенности

- **Единый исполняемый файл**: `magray` (target ~16MB)
- **Conditional compilation**: cpu/gpu/minimal variants
- **Memory система**: 3 слоя (Interact/Insights/Assets) с HNSW индексами  
- **AI модели**: Qwen3 embeddings (1024D), BGE-M3 legacy support
- **LLM провайдеры**: OpenAI/Anthropic/Local
- **Production готовность**: Circuit breakers, health checks, metrics

{self._generate_analysis_report()}
"""
        
        # Заменяем секцию
        before_section = content[:start_idx]
        after_section = content[end_idx:]
        new_content = before_section + new_section + after_section
        
        with open(self.claude_md, 'w', encoding='utf-8') as f:
            f.write(new_content)
        
        print(f"[OK] CLAUDE.md обновлен ({timestamp})")
    
    def watch_mode(self):
        """Запускает watchdog режим для мониторинга изменений"""
        print("[INFO] Запуск watch режима...")
        
        class ArchitectureHandler(FileSystemEventHandler):
            def __init__(self, daemon):
                self.daemon = daemon
                self.last_update = 0
                
            def on_modified(self, event):
                if event.is_directory:
                    return
                    
                if event.src_path.endswith('Cargo.toml'):
                    # Debounce - обновляем не чаще раза в 5 секунд
                    now = time.time()
                    if now - self.last_update < 5:
                        return
                        
                    self.last_update = now
                    print(f"[WATCH] Обнаружены изменения в {event.src_path}")
                    
                    # Отложенное обновление через 2 секунды
                    threading.Timer(2.0, self._delayed_update).start()
                    
            def _delayed_update(self):
                try:
                    arch = self.daemon.scan_architecture()
                    self.daemon.update_claude_md(arch)
                    print("[OK] Автообновление завершено")
                except Exception as e:
                    print(f"[ERROR] Ошибка автообновления: {e}")
        
        observer = Observer()
        observer.schedule(
            ArchitectureHandler(self),
            str(self.crates_dir), 
            recursive=True
        )
        
        observer.start()
        print(f"[WATCH] Мониторинг {self.crates_dir} запущен")
        
        try:
            while True:
                time.sleep(1)
        except KeyboardInterrupt:
            observer.stop()
            print("\n[INFO] Watch режим остановлен")
        
        observer.join()
    
    def run_once(self):
        """Единократное обновление диаграммы"""
        print("[INFO] Запуск единократного анализа...")
        arch = self.scan_architecture()
        self.update_claude_md(arch)
        print("[OK] Анализ завершен")
        
        return arch

def main():
    parser = argparse.ArgumentParser(
        description="Ультракомпактный архитектурный демон для MAGRAY CLI"
    )
    parser.add_argument(
        '--watch', '-w', 
        action='store_true',
        help='Запустить в watch режиме для автообновлений'
    )
    parser.add_argument(
        '--project-root', '-p',
        default='.',
        help='Путь к корню проекта (по умолчанию: текущая директория)'
    )
    
    args = parser.parse_args()
    
    # Определяем корень проекта
    project_root = Path(args.project_root).resolve()
    if not (project_root / "Cargo.toml").exists():
        print("[ERROR] Не найден Cargo.toml в корне проекта")
        print(f"Проверьте путь: {project_root}")
        sys.exit(1)
    
    print(f"[INFO] Проект: {project_root}")
    
    daemon = ArchitectureDaemon(str(project_root))
    
    if args.watch:
        daemon.watch_mode()
    else:
        daemon.run_once()

if __name__ == "__main__":
    main()