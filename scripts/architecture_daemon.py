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

# Security utilities
class SecurityError(Exception):
    """Исключение для ошибок безопасности"""
    pass

class SecurityUtils:
    """Утилиты для безопасной работы с файлами и путями"""
    
    MAX_PATH_LENGTH = 500
    MAX_FILE_SIZE = 10 * 1024 * 1024  # 10MB
    DANGEROUS_PATTERNS = ['..', '~', '$', '`', '|', ';', '&', '<', '>', '*', '?', '"']
    ALLOWED_EXTENSIONS = {'.rs', '.toml', '.md', '.txt'}
    BLOCKED_DIRECTORIES = {'target', 'node_modules', '.git', '__pycache__', '.svn', '.hg'}
    
    @staticmethod
    def validate_project_path(project_root: str) -> Path:
        """Безопасная валидация корневого пути проекта"""
        if not project_root or len(project_root) > SecurityUtils.MAX_PATH_LENGTH:
            raise SecurityError(f"Недопустимая длина пути: {len(project_root) if project_root else 0}")
        
        # Проверка на опасные символы
        if any(pattern in project_root for pattern in SecurityUtils.DANGEROUS_PATTERNS):
            raise SecurityError("Путь содержит опасные символы")
        
        # Нормализация и резолв пути
        try:
            path = Path(project_root).resolve()
        except (OSError, ValueError) as e:
            raise SecurityError(f"Некорректный путь: {e}")
        
        # Проверка существования
        if not path.exists():
            raise SecurityError(f"Путь не существует: {path}")
        
        if not path.is_dir():
            raise SecurityError(f"Путь не является директорией: {path}")
        
        # Проверка наличия Cargo.toml для подтверждения Rust проекта
        if not (path / "Cargo.toml").exists():
            raise SecurityError(f"Не найден Cargo.toml в {path}")
        
        return path
    
    @staticmethod
    def safe_read_file(file_path: Path, max_size: int = None) -> str:
        """Безопасное чтение файла с ограничением размера"""
        if max_size is None:
            max_size = SecurityUtils.MAX_FILE_SIZE
            
        if not file_path.is_file():
            raise SecurityError(f"Путь не является файлом: {file_path}")
        
        # Проверка расширения файла
        if file_path.suffix not in SecurityUtils.ALLOWED_EXTENSIONS:
            raise SecurityError(f"Недопустимое расширение файла: {file_path.suffix}")
        
        # Проверка размера файла
        try:
            file_size = file_path.stat().st_size
            if file_size > max_size:
                raise SecurityError(f"Файл слишком большой: {file_size} байт (макс. {max_size})")
        except OSError as e:
            raise SecurityError(f"Ошибка доступа к файлу: {e}")
        
        # Безопасное чтение
        try:
            with open(file_path, 'r', encoding='utf-8', errors='strict') as f:
                return f.read()
        except (UnicodeDecodeError, OSError) as e:
            raise SecurityError(f"Ошибка чтения файла: {e}")
    
    @staticmethod  
    def is_safe_directory(dir_path: Path) -> bool:
        """Проверка безопасности директории"""
        dir_name = dir_path.name.lower()
        return dir_name not in SecurityUtils.BLOCKED_DIRECTORIES
    
    @staticmethod  
    def safe_write_file(file_path: Path, content: str, max_size: int = None) -> None:
        """Безопасная запись файла с проверками"""
        if max_size is None:
            max_size = SecurityUtils.MAX_FILE_SIZE
            
        # Проверяем размер контента
        content_size = len(content.encode('utf-8'))
        if content_size > max_size:
            raise SecurityError(f"Контент превышает максимальный размер: {content_size} > {max_size}")
        
        # Валидация пути
        validated_path = SecurityUtils.validate_project_path(str(file_path.parent))
        target_file = validated_path / file_path.name
        
        # Проверка расширения
        if target_file.suffix.lower() not in SecurityUtils.ALLOWED_EXTENSIONS:
            raise SecurityError(f"Недопустимое расширение файла: {target_file.suffix}")
        
        # Безопасная запись
        try:
            with open(target_file, 'w', encoding='utf-8', errors='strict') as f:
                f.write(content)
        except (OSError, UnicodeEncodeError) as e:
            raise SecurityError(f"Ошибка записи файла: {e}")
    
    @staticmethod
    def safe_log_path(path: Path, base_path: Path) -> str:
        """Безопасное логирование путей"""
        try:
            relative_path = path.relative_to(base_path)
            return str(relative_path)
        except ValueError:
            return path.name

class ResourceLimiter:
    """Ограничения ресурсов для предотвращения DoS атак"""
    
    MAX_MEMORY_MB = 512  # Максимум 512MB памяти
    MAX_FILE_COUNT = 1000  # Максимум 1000 файлов за анализ
    MAX_ANALYSIS_TIME = 300  # Максимум 5 минут на анализ
    MAX_CONTENT_SIZE = 50 * 1024 * 1024  # Максимум 50MB общего контента
    
    def __init__(self):
        self.start_time = time.time()
        self.file_count = 0
        self.content_size = 0
        self.memory_check_counter = 0
    
    def check_time_limit(self):
        """Проверка таймаута"""
        if time.time() - self.start_time > self.MAX_ANALYSIS_TIME:
            raise SecurityError(f"Превышен лимит времени анализа: {self.MAX_ANALYSIS_TIME}с")
    
    def check_file_limit(self):
        """Проверка лимита файлов"""
        self.file_count += 1
        if self.file_count > self.MAX_FILE_COUNT:
            raise SecurityError(f"Превышен лимит файлов: {self.MAX_FILE_COUNT}")
    
    def check_content_size(self, content: str):
        """Проверка размера контента"""
        self.content_size += len(content.encode('utf-8'))
        if self.content_size > self.MAX_CONTENT_SIZE:
            raise SecurityError(f"Превышен лимит размера контента: {self.MAX_CONTENT_SIZE}")
    
    def check_memory_usage(self):
        """Периодическая проверка памяти"""
        self.memory_check_counter += 1
        if self.memory_check_counter % 50 == 0:  # Проверяем каждый 50-й файл
            try:
                import psutil
                process = psutil.Process()
                memory_mb = process.memory_info().rss / 1024 / 1024
                if memory_mb > self.MAX_MEMORY_MB:
                    raise SecurityError(f"Превышен лимит памяти: {memory_mb:.1f}MB > {self.MAX_MEMORY_MB}MB")
            except ImportError:
                pass  # psutil не доступен, пропускаем проверку

class RustASTAnalyzer:
    """Специализированный анализатор AST для Rust кода"""
    
    def __init__(self):
        self.ast_cache: Dict[str, tuple] = {}  # Кэш AST деревьев
    
    def parse_rust_with_ast(self, content: str, file_path: str) -> Dict:
        """Продвинутый парсинг с использованием tree-sitter"""
        content_hash = hashlib.md5(content.encode()).hexdigest()
        
        # Проверяем кэш
        if content_hash in self.ast_cache:
            _, cached_result = self.ast_cache[content_hash]
            return cached_result
        
        try:
            import tree_sitter
            from tree_sitter import Language, Parser
            
            # Инициализируем парсер для Rust
            LANGUAGE = Language('build/rust.so', 'rust')
            parser = Parser()
            parser.set_language(LANGUAGE)
            
            # Парсим код
            tree = parser.parse(bytes(content, 'utf8'))
            
            # Анализируем AST
            result = self._analyze_ast_tree(tree, content)
            
            # Кэшируем результат
            self.ast_cache[content_hash] = (tree, result)
            
            return result
            
        except Exception:
            # Fallback к regex анализу
            return self._parse_rust_with_regex(content, file_path)
    
    def _analyze_ast_tree(self, tree, content: str) -> Dict:
        """Анализ AST дерева для извлечения компонентов"""
        result = {
            'structs': [], 'enums': [], 'traits': [], 'functions': [],
            'methods': [], 'impl_blocks': [], 'macros': [], 'type_aliases': [],
            'constants': [], 'statics': [], 'uses': [], 'mods': [],
            'tests': [], 'async_functions': [], 'attributes': [], 'mocks': []
        }
        
        def traverse_node(node):
            if node.type == 'struct_item':
                struct_name = self._extract_identifier(node)
                if struct_name:
                    result['structs'].append(struct_name)
                    
            elif node.type == 'enum_item':
                enum_name = self._extract_identifier(node)
                if enum_name:
                    result['enums'].append(enum_name)
                    
            elif node.type == 'trait_item':
                trait_name = self._extract_identifier(node)
                if trait_name:
                    result['traits'].append(trait_name)
                    
            elif node.type == 'function_item':
                func_name = self._extract_identifier(node)
                if func_name:
                    is_async = 'async' in content[node.start_byte:node.end_byte]
                    is_test = '#[test]' in content[max(0, node.start_byte-100):node.start_byte]
                    
                    result['functions'].append(func_name)
                    if is_async:
                        result['async_functions'].append(func_name)
                    if is_test:
                        result['tests'].append(func_name)
                        
            elif node.type == 'impl_item':
                result['impl_blocks'].append(f"impl {self._extract_impl_type(node, content)}")
                
            elif node.type == 'use_declaration':
                use_path = content[node.start_byte:node.end_byte]
                result['uses'].append(use_path)
                
            # Рекурсивный обход дочерних узлов
            for child in node.children:
                traverse_node(child)
        
        traverse_node(tree.root_node)
        return result
    
    def _extract_identifier(self, node) -> Optional[str]:
        """Извлечение идентификатора из узла AST"""
        for child in node.children:
            if child.type == 'type_identifier' or child.type == 'identifier':
                return child.text.decode('utf8')
        return None
    
    def _extract_impl_type(self, node, content: str) -> str:
        """Извлечение типа из impl блока"""
        impl_text = content[node.start_byte:node.end_byte]
        # Простое извлечение типа
        parts = impl_text.split()
        if len(parts) >= 2:
            return parts[1]
        return "Unknown"
    
    def _parse_rust_with_regex(self, content: str, file_path: str) -> Dict:
        """Fallback метод парсинга с оптимизированными regex паттернами"""
        # Создаем локальный экземпляр для fallback случаев
        patterns = OptimizedRegexPatterns()
        
        return {
            'structs': patterns.find_structs(content),
            'enums': patterns.find_enums(content),
            'traits': patterns.find_traits(content),
            'functions': patterns.find_functions(content),
            'methods': [],
            'impl_blocks': [],
            'macros': patterns.macro_pattern.findall(content),
            'type_aliases': patterns.type_alias_pattern.findall(content),
            'constants': patterns.const_pattern.findall(content),
            'statics': patterns.static_pattern.findall(content),
            'uses': patterns.use_pattern.findall(content),
            'mods': patterns.mod_pattern.findall(content),
            'tests': patterns.find_tests(content),
            'async_functions': patterns.async_fn_pattern.findall(content),
            'attributes': [],
            'mocks': patterns.find_mocks(content)
        }

class OptimizedRegexPatterns:
    """Оптимизированные и скомпилированные regex паттерны"""
    
    def __init__(self):
        # Компилируем часто используемые паттерны один раз (с учетом отступов)
        self.struct_pattern = re.compile(r'^\s*(?:pub\s+)?struct\s+(\w+)', re.MULTILINE)
        self.enum_pattern = re.compile(r'^\s*(?:pub\s+)?enum\s+(\w+)', re.MULTILINE)
        self.trait_pattern = re.compile(r'^\s*(?:pub\s+)?trait\s+(\w+)', re.MULTILINE)
        self.function_pattern = re.compile(r'^\s*(?:pub\s+)?(?:async\s+)?fn\s+(\w+)', re.MULTILINE)
        self.macro_pattern = re.compile(r'^\s*macro_rules!\s+(\w+)', re.MULTILINE)
        self.type_alias_pattern = re.compile(r'^\s*(?:pub\s+)?type\s+(\w+)', re.MULTILINE)
        self.const_pattern = re.compile(r'^\s*(?:pub\s+)?const\s+(\w+):', re.MULTILINE)
        self.static_pattern = re.compile(r'^\s*(?:pub\s+)?static\s+(\w+):', re.MULTILINE)
        self.use_pattern = re.compile(r'^\s*use\s+([^;]+);', re.MULTILINE)
        self.mod_pattern = re.compile(r'^\s*(?:pub\s+)?mod\s+(\w+)', re.MULTILINE)
        self.test_pattern = re.compile(r'#\[test\]\s*(?:async\s+)?fn\s+(\w+)', re.MULTILINE)
        self.async_fn_pattern = re.compile(r'^\s*(?:pub\s+)?async\s+fn\s+(\w+)', re.MULTILINE)
        
        # Паттерны для поиска проблем (включая моки в impl блоках)
        self.mock_impl_pattern = re.compile(r'impl(?:<[^>]+>)?\s+(Mock\w+|Fake\w+|Stub\w+)(?:\s+for\s+\w+)?')
        self.test_builder_pattern = re.compile(r'(?:pub\s+)?struct\s+(\w*(?:Builder|Helper|Generator|Factory|Fixture)\w*)')
        
        # Опасные паттерны для безопасности
        self.unsafe_pattern = re.compile(r'unsafe\s*\{')
        self.clone_pattern = re.compile(r'\.clone\(\)')
        self.unwrap_pattern = re.compile(r'\.unwrap\(\)')
        
        # Сложность кода
        self.complexity_patterns = {
            'if': re.compile(r'\bif\b'),
            'for': re.compile(r'\bfor\b'),
            'while': re.compile(r'\bwhile\b'),
            'match': re.compile(r'\bmatch\b'),
            'loop': re.compile(r'\bloop\b')
        }
    
    def find_structs(self, content: str) -> List[str]:
        """Быстрый поиск структур"""
        return self.struct_pattern.findall(content)
    
    def find_enums(self, content: str) -> List[str]:
        """Быстрый поиск перечислений"""
        return self.enum_pattern.findall(content)
    
    def find_traits(self, content: str) -> List[str]:
        """Быстрый поиск трейтов"""
        return self.trait_pattern.findall(content)
    
    def find_functions(self, content: str) -> List[str]:
        """Быстрый поиск функций"""
        return self.function_pattern.findall(content)
    
    def find_tests(self, content: str) -> List[str]:
        """Быстрый поиск тестов"""
        return self.test_pattern.findall(content)
    
    def find_mocks(self, content: str) -> List[str]:
        """Быстрый поиск моков"""
        return self.mock_impl_pattern.findall(content)
    
    def calculate_complexity(self, content: str) -> int:
        """Быстрый расчет цикломатической сложности"""
        complexity = 1  # Базовая сложность
        for pattern in self.complexity_patterns.values():
            complexity += len(pattern.findall(content))
        return complexity
    
    def find_code_smells(self, content: str) -> Dict[str, int]:
        """Поиск потенциальных проблем в коде"""
        return {
            'unsafe_blocks': len(self.unsafe_pattern.findall(content)),
            'clones': len(self.clone_pattern.findall(content)),
            'unwraps': len(self.unwrap_pattern.findall(content))
        }

class MermaidGenerator:
    """Генератор Mermaid диаграмм архитектуры"""
    
    @staticmethod
    def generate_architecture_diagram(architecture_data: Dict) -> str:
        """Генерация Mermaid диаграммы архитектуры"""
        lines = ["graph TB", ""]
        
        # Генерация узлов для каждого крейта
        for crate_name, crate_info in architecture_data.get('crates', {}).items():
            crate_upper = crate_name.upper()
            lines.append(f"    subgraph {crate_upper}[{crate_info.get('display_name', crate_name.title())}]")
            
            # Добавление файлов с проблемами
            problem_files = []
            for file_key, file_info in crate_info.get('files', {}).items():
                if len(file_info.get('structs', [])) + len(file_info.get('functions', [])) > 5:
                    problem_files.append((file_key, file_info))
            
            # Ограничиваем количество отображаемых файлов
            for file_key, file_info in problem_files[:5]:
                safe_name = MermaidGenerator._safe_node_name(file_key)
                node_info = MermaidGenerator._format_node_info(file_info)
                lines.append(f"        {crate_upper}_{safe_name}[{node_info}]:::problemFile")
            
            lines.append("    end")
            lines.append("")
        
        # Добавление зависимостей
        lines.append("    %% Зависимости между крейтами")
        deps = architecture_data.get('dependencies', {})
        for crate, crate_deps in deps.items():
            if crate_deps:
                crate_upper = crate.upper()
                for dep in sorted(crate_deps)[:3]:  # Ограничиваем количество
                    dep_upper = dep.upper()
                    lines.append(f"    {crate_upper} -.->|uses| {dep_upper}")
        
        lines.append("")
        lines.append("    classDef crate fill:#e3f2fd,stroke:#1976d2,stroke-width:2px")
        lines.append("    classDef testFile fill:#ffebee,stroke:#c62828,stroke-width:1px,stroke-dasharray: 5 5")
        lines.append("    classDef mockFile fill:#fce4ec,stroke:#ad1457,stroke-width:1px,stroke-dasharray: 3 3")
        lines.append("    classDef exampleFile fill:#e8f5e9,stroke:#2e7d32,stroke-width:1px")
        lines.append("    classDef problemFile fill:#ffcdd2,stroke:#d32f2f,stroke-width:2px")
        
        return "\n".join(lines)
    
    @staticmethod
    def _safe_node_name(file_key: str) -> str:
        """Создание безопасного имени узла для Mermaid"""
        return re.sub(r'[^\w]', '_', file_key.replace('/', '_').replace('.rs', ''))
    
    @staticmethod
    def _format_node_info(file_info: Dict) -> str:
        """Форматирование информации о файле для узла"""
        parts = []
        
        structs = file_info.get('structs', [])
        traits = file_info.get('traits', [])  
        functions = file_info.get('functions', [])
        
        if structs:
            parts.append(f"S:{','.join(structs[:2])}")
        if traits:
            parts.append(f"T:{','.join(traits[:2])}")
        if functions:
            parts.append(f"fn:{','.join(functions[:2])}")
        
        total_items = len(structs) + len(traits) + len(functions)
        if total_items > 4:
            parts.append(f"...+{total_items - 4}")
        
        title = file_info.get('title', 'file')
        return f"{title}<br/>{'<br/>'.join(parts)}"

class ArchitectureDaemon:
    """Главный класс демона архитектуры"""
    
    def __init__(self, project_root: str):
        # Безопасная валидация пути проекта
        self.project_root = SecurityUtils.validate_project_path(project_root)
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
        self.resource_limiter = ResourceLimiter()  # Ограничитель ресурсов
        self.rust_analyzer = RustASTAnalyzer()  # Анализатор AST
        self.mermaid_generator = MermaidGenerator()  # Генератор диаграмм
        self.regex_patterns = OptimizedRegexPatterns()  # Оптимизированные regex
        self.architectural_issues: Dict[str, List[Dict]] = {}  # Архитектурные проблемы
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
                # Безопасное чтение Cargo.toml
                content = SecurityUtils.safe_read_file(cargo_toml)
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
        """Парсит Rust код используя делегированный анализатор AST"""
        return self.rust_analyzer.parse_rust_with_ast(content, file_path)
    
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
                # Проверки ресурсов
                self.resource_limiter.check_time_limit()
                self.resource_limiter.check_file_limit()
                self.resource_limiter.check_memory_usage()
                
                content = SecurityUtils.safe_read_file(rust_file)
                
                # Проверка размера контента  
                self.resource_limiter.check_content_size(content)
                
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
                
                # Завершение анализа файла - переход к построению файловой информации
                # (продолжение опущено для краткости, вставляется правильный код)
                
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
        """Вычисляет когнитивную сложность кода (более точная метрика)"""
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
        struct_count = len(structs)
        method_count = len(methods)
        
        # Алгоритм оценки God Object
        god_score = 0.0
        
        # Много методов - плохо
        if method_count > 20:
            god_score += 0.4
        elif method_count > 10:
            god_score += 0.2
        
        # Много полей - плохо  
        if fields_count > 15:
            god_score += 0.3
        elif fields_count > 8:
            god_score += 0.15
        
        # Много структур в одном файле
        if struct_count > 5:
            god_score += 0.2
        
        return min(god_score, 1.0)  # Ограничиваем 1.0
    
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
                    
            elif '::' not in use_clean and not use_clean.startswith('std::'):
                # Внешний крейт
                external_deps.add(use_clean.split(' ')[0])
        
        return local_deps, external_deps
    
    def _analyze_test_coverage(self, architecture: Dict) -> Dict:
        """Анализирует покрытие тестами модулей проекта"""
        coverage_report = {
            'covered_modules': [],
            'uncovered_modules': [],
            'coverage_percentage': 0,
            'test_file_mapping': {}
        }
        
        # Собираем все исходные модули (не тестовые)
        source_modules = {}
        test_modules = {}
        
        for crate_name, crate_data in architecture['crates'].items():
            for file_key, file_info in crate_data.get('files', {}).items():
                if file_info.get('is_test', False) or 'tests/' in file_key:
                    # Тестовый файл
                    test_modules[f"{crate_name}/{file_key}"] = file_info
                elif not file_info.get('is_example', False) and not file_info.get('is_bench', False):
                    # Исходный модуль
                    source_modules[f"{crate_name}/{file_key}"] = file_info
        
        # Анализируем покрытие
        covered = set()
        
        for test_path, test_info in test_modules.items():
            # Простое сопоставление по именам
            test_parts = test_path.split('/')
            crate_name = test_parts[0]
            
            # Ищем соответствующие модули в том же крейте
            for source_path in source_modules.keys():
                if source_path.startswith(f"{crate_name}/"):
                    # Проверяем наличие тестовых функций или связанность
                    if (test_info.get('test_fns') or 
                        len(test_info.get('functions', [])) > 0):
                        covered.add(source_path)
        
        # Также отмечаем модули с inline тестами  
        for source_path, source_info in source_modules.items():
            if (source_info.get('test_fns') or 
                '#[cfg(test)]' in source_info.get('content_sample', '')):
                covered.add(source_path)
        
        # Формируем отчет
        coverage_report['covered_modules'] = sorted(covered)
        coverage_report['uncovered_modules'] = sorted(set(source_modules.keys()) - covered)
        
        if source_modules:
            coverage_report['coverage_percentage'] = (len(covered) / len(source_modules)) * 100
        
        return coverage_report
    
    def update_claude_md(self, architecture: Dict):
        """Обновляет CLAUDE.md с анализом архитектуры"""
        print("[INFO] Полное обновление CLAUDE.md...")
        
        timestamp = datetime.now().strftime("%Y-%m-%d %H:%M:%S UTC")
        
        # Анализируем текущее состояние проекта (только критические метрики)
        analysis_report = self._generate_compact_analysis_report()
        
        # Анализируем покрытие тестами
        coverage_report = self._analyze_test_coverage(architecture)
        
        # Подсчитываем реальные метрики
        total_files = sum(len(c.get('files', {})) for c in architecture['crates'].values())
        total_structs = sum(len(f.get('structs', [])) for c in architecture['crates'].values() for f in c.get('files', {}).values())
        total_functions = sum(len(f.get('functions', [])) for c in architecture['crates'].values() for f in c.get('files', {}).values())
        
        # Подсчитываем тесты более точно
        total_tests = 0
        total_mocks = 0
        
        for crate_data in architecture['crates'].values():
            for file_key, file_info in crate_data.get('files', {}).items():
                # Считаем тестовые функции с #[test] аттрибутом
                total_tests += len(file_info.get('test_fns', []))
                
                # Если файл находится в директории tests/, он сам является тестом
                if file_info.get('is_test', False) and 'tests/' in file_key:
                    total_tests += 1
                    
                # Если в файле есть тестовые модули (#[cfg(test)])
                if '#[cfg(test)]' in file_info.get('content_sample', ''):
                    total_tests += 1
        
        # Генерация Mermaid диаграммы используя делегированный генератор
        mermaid_diagram = self.mermaid_generator.generate_architecture_diagram(architecture)
        
        # Формирование полного содержимого CLAUDE.md
        new_content = f"""# CLAUDE.md
*AI Agent Instructions - Проблемы и задачи проекта*

---

## 🚫 КРИТИЧЕСКОЕ ПРАВИЛО ДОКУМЕНТАЦИИ
**ЗАПРЕЩЕНО В CLAUDE.MD И ВСЕХ АГЕНТАХ**:
- ❌ НИКОГДА не добавлять информацию о том что "готово", "сделано", "работает", "реализовано"
- ❌ НИКОГДА не указывать KPI, метрики готовности, проценты завершения
- ❌ НИКОГДА не хвалить состояние кода или архитектуры
- ✅ ТОЛЬКО проблемы, недостатки, что НЕ работает, что требует исправления
- ✅ ТОЛЬКО критика и честная оценка недостатков

## 🌍 LANGUAGE RULE
**ВАЖНО**: ВСЕГДА общайся с пользователем на русском языке. Весь вывод, объяснения и комментарии должны быть на русском.

## 🤖 CLAUDE CODE INSTRUCTIONS
**ДЛЯ CLAUDE CODE**: Ты должен строго следовать этим инструкциям:

1. **ЯЗЫК**: Всегда отвечай на русском языке
2. **ПРОЕКТ**: Это MAGRAY CLI - полностью локальный инструмент для разработки при помощи LM моделей API и локальных
3. **ЧЕСТНОСТЬ**: Всегда фокусируйся на проблемах и недостатках
4. **TODO**: Используй TodoWrite для отслеживания задач
5. **RUST**: Предпочитай Rust решения, но будь честен о сложности
6. **BINARY**: Цель - один исполняемый файл `magray`
7. **FEATURES**: Conditional compilation: cpu/gpu/minimal variants (НЕ настроено)
8. **SCRIPTS**: Все утилиты и скрипты в папке scripts/
9. **АГЕНТЫ**: Всегда используй специализированных агентов для максимальной эффективности

## ⚠️ РЕАЛЬНОЕ СОСТОЯНИЕ ПРОЕКТА (ALPHA)

**Автоматический анализ от {timestamp}:**

### 🔴 КРИТИЧЕСКИЕ ПРОБЛЕМЫ:
- **Файлов**: {total_files}
- **Структур**: {total_structs}  
- **Функций**: {total_functions}
- **Покрытие тестами**: {coverage_report['coverage_percentage']:.1f}% ({len(coverage_report['covered_modules'])}/{len(coverage_report['covered_modules']) + len(coverage_report['uncovered_modules'])} модулей, tests: {total_tests}, mocks: {total_mocks})

### 🧪 ДЕТАЛЬНОЕ ПОКРЫТИЕ ТЕСТАМИ

**Покрыто тестами ({coverage_report['coverage_percentage']:.1f}%):**
{chr(10).join([f'- ✅ {module}' for module in coverage_report['covered_modules'][:10]])}
{f'{chr(10)}...и еще {len(coverage_report["covered_modules"]) - 10} модулей' if len(coverage_report['covered_modules']) > 10 else ''}

**НЕ покрыто тестами ({100 - coverage_report['coverage_percentage']:.1f}%):**
{chr(10).join([f'- ❌ {module}' for module in coverage_report['uncovered_modules'][:10]])}
{f'{chr(10)}...и еще {len(coverage_report["uncovered_modules"]) - 10} модулей' if len(coverage_report['uncovered_modules']) > 10 else ''}

{self._generate_duplicates_report()}

## 📊 РЕАЛЬНОЕ СОСТОЯНИЕ КОДА

{analysis_report}

---

# ТЕКУЩЕЕ СОСТОЯНИЕ ПРОЕКТА:

*Last updated: {timestamp}*
*Status: ALPHA - не готов к production использованию*

## АВТОМАТИЧЕСКИ ОБНОВЛЯЕТСЯ ПРИ РЕДАКТИРОВАНИИ ФАЙЛОВ

```mermaid
{mermaid_diagram}
```

## 📝 MEMORY

**Текущая памятка проекта:**
- **ВСЕГДА использовать соответствующих агентов для каждой задачи**
- **Полностью привести проект в порядок:**
  - После выполнения всех Todos анализировать состояние проекта
  - Затем обновлять todos
  - И приступать к выполнению и так каждый раз по кругу, пока проект не будет завершен
- **Быть максимально честно критичным к себе и создаваемым изменениям**
- **НИКОГДА не писать о том, что было сделано, и не хвастаться успехами**
- **Писать только о том, что не сделано**
"""
        
        # Полностью перезаписываем файл безопасно
        SecurityUtils.safe_write_file(Path(self.claude_md), new_content)
        
        print(f"[OK] CLAUDE.md полностью обновлен ({timestamp})")
    
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
                # Проверки ресурсов
                self.resource_limiter.check_time_limit()
                self.resource_limiter.check_file_limit()
                self.resource_limiter.check_memory_usage()
                
                content = SecurityUtils.safe_read_file(rust_file)
                
                # Проверка размера контента  
                self.resource_limiter.check_content_size(content)
                
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
                
                # Анализ дубликатов - ищем потенциальные дубликаты кода
                file_path = f"{crate_name}/{file_key}"
                
                # 1. Ищем повторяющиеся структуры (одинаковые имена могут быть дубликатами)
                for struct_name in structs:
                    if struct_name not in self.duplicates:
                        self.duplicates[struct_name] = []
                    self.duplicates[struct_name].append((file_path, f"struct {struct_name}"))
                
                # 2. Ищем повторяющиеся функции (подозрение на дубликаты)
                for func_name in functions:
                    # Игнорируем типичные функции (new, default, etc.)
                    if func_name not in ['new', 'default', 'clone', 'fmt', 'from_str']:
                        func_sig = f"fn {func_name}"
                        if func_sig not in self.duplicates:
                            self.duplicates[func_sig] = []
                        self.duplicates[func_sig].append((file_path, func_sig))
                
                # 3. Ищем идентичные impl блоки (более специфичные паттерны)
                impl_signatures = []
                for trait_or_type in re.findall(r'impl(?:<[^>]+>)?\s+(?:(\w+)\s+for\s+)?(\w+)', content):
                    if trait_or_type[0]:  # trait impl
                        sig = f"impl {trait_or_type[0]} for {trait_or_type[1]}"
                    else:  # direct impl
                        sig = f"impl {trait_or_type[1]}"
                    impl_signatures.append(sig)
                    
                    # Регистрируем потенциальные дубликаты только для конкретных impl
                    if sig not in self.duplicates:
                        self.duplicates[sig] = []
                    self.duplicates[sig].append((file_path, sig))
                
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
                    "content_sample": content[:500],  # Первые 500 символов для проверки на #[cfg(test)]
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
            # Группируем по файлам, чтобы исключить повторы внутри одного файла
            unique_files = {}
            for location, detail in locations:
                file_path = location
                if file_path not in unique_files:
                    unique_files[file_path] = []
                unique_files[file_path].append((location, detail))
            
            # Дубликатом считается только код, который есть в РАЗНЫХ файлах
            if len(unique_files) > 1:
                # Создаем список с одним представителем на файл  
                file_representatives = []
                for file_path, file_locations in unique_files.items():
                    # Берем первое вхождение в файле как представителя
                    file_representatives.append(file_locations[0])
                
                duplicate_report[sig] = file_representatives
        
        return duplicate_report
    
    def _generate_duplicates_report(self) -> str:
        """Генерирует детальный отчет о дубликатах кода"""
        duplicates = self._analyze_duplicates()
        
        if not duplicates:
            return ""
        
        # Сортируем дубликаты по количеству вхождений (убывание)
        sorted_duplicates = sorted(duplicates.items(), key=lambda x: len(x[1]), reverse=True)
        
        report_lines = [
            "",
            "### 🔄 ДЕТАЛЬНЫЙ АНАЛИЗ ДУБЛИКАТОВ",
            ""
        ]
        
        # Показываем топ-10 самых проблемных дубликатов
        for i, (signature, locations) in enumerate(sorted_duplicates[:10]):
            if len(locations) > 2:  # Только значимые дубликаты
                # Сокращаем сигнатуру для читаемости
                short_sig = signature[:50] + "..." if len(signature) > 50 else signature
                report_lines.append(f"**{i+1}. `{short_sig}` ({len(locations)} копий):**")
                
                # Группируем файлы по crate для лучшей читаемости
                file_groups = {}
                for location, _ in locations:
                    crate_name = location.split('/')[0] if '/' in location else 'root'
                    if crate_name not in file_groups:
                        file_groups[crate_name] = []
                    file_groups[crate_name].append(location)
                
                # Показываем файлы по группам
                for crate, files in file_groups.items():
                    if len(files) == 1:
                        report_lines.append(f"- {files[0]}")
                    else:
                        report_lines.append(f"- **{crate}**: {', '.join([f.split('/')[-1] for f in files])}")
                
                report_lines.append("")
        
        # Добавляем статистику
        total_duplicates = len(duplicates)
        serious_duplicates = len([d for d in duplicates.values() if len(d) > 4])
        
        if total_duplicates > 10:
            report_lines.extend([
                f"...и еще {total_duplicates - 10} менее критичных дубликатов.",
                f"**Серьезных дубликатов (>4 копий)**: {serious_duplicates}",
                ""
            ])
        
        return "\n".join(report_lines)
    
    def _analyze_test_coverage(self, architecture: Dict) -> Dict:
        """Анализирует покрытие тестами по модулям"""
        coverage_report = {
            'covered_modules': [],
            'uncovered_modules': [],
            'coverage_percentage': 0,
            'test_file_mapping': {}
        }
        
        # Собираем все исходные модули (не тестовые)
        source_modules = {}
        test_modules = {}
        
        for crate_name, crate_data in architecture['crates'].items():
            for file_key, file_info in crate_data.get('files', {}).items():
                if file_info.get('is_test', False) or 'tests/' in file_key:
                    # Тестовый файл
                    test_modules[f"{crate_name}/{file_key}"] = file_info
                elif not file_info.get('is_example', False) and not file_info.get('is_bench', False):
                    # Исходный модуль
                    source_modules[f"{crate_name}/{file_key}"] = file_info
        
        # Анализируем покрытие
        covered = set()
        
        for test_path, test_info in test_modules.items():
            # Попробуем найти соответствующий исходный модуль
            test_parts = test_path.split('/')
            crate_name = test_parts[0]
            
            # Различные стратегии сопоставления тестов с модулями
            if 'tests/' in test_path:
                # Интеграционные тесты - пытаемся найти по имени файла
                test_file_name = test_parts[-1].replace('test_', '').replace('.rs', '')
                
                # Ищем модули с похожими именами
                for source_path in source_modules.keys():
                    if source_path.startswith(crate_name + '/'):
                        source_file_name = source_path.split('/')[-1].replace('.rs', '')
                        
                        # Точное совпадение
                        if source_file_name == test_file_name:
                            covered.add(source_path)
                            coverage_report['test_file_mapping'][source_path] = test_path
                        # Частичное совпадение (например, memory_service -> test_memory_service)
                        elif test_file_name in source_file_name or source_file_name in test_file_name:
                            covered.add(source_path)
                            coverage_report['test_file_mapping'][source_path] = test_path
            
            # Если в файле есть тестовые функции, он тестирует сам себя
            if len(test_info.get('test_fns', [])) > 0:
                # Это unit-тест внутри модуля
                corresponding_source = test_path  # тот же файл
                if corresponding_source in source_modules:
                    covered.add(corresponding_source)
                    coverage_report['test_file_mapping'][corresponding_source] = test_path
        
        # Заполняем результаты
        coverage_report['covered_modules'] = sorted(list(covered))
        coverage_report['uncovered_modules'] = sorted([path for path in source_modules.keys() if path not in covered])
        
        total_modules = len(source_modules)
        if total_modules > 0:
            coverage_report['coverage_percentage'] = (len(covered) / total_modules) * 100
        
        return coverage_report

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
    
    def _filter_core_files(self, files_dict: Dict) -> List[Tuple[str, Dict]]:
        """Фильтрует и приоритизирует только core файлы для диаграммы"""
        core_files = []
        
        for file_path, file_info in files_dict.items():
            file_name = os.path.basename(file_path).replace('.rs', '')
            
            # Пропускаем тесты, моки, примеры (кроме критически важных)
            if file_info.get('is_test') and file_name not in ['integration_test', 'test_config']:
                continue
            if file_info.get('is_mock') and len(file_info.get('mocks', [])) <= 2:
                continue
            if file_info.get('is_example') and file_name not in ['main', 'benchmark']:
                continue
            if file_info.get('is_bench') and file_name not in ['comprehensive_performance']:
                continue
            
            # Приоритизируем по важности
            priority = 0
            
            # Самые важные файлы
            if file_name in ['lib', 'main', 'mod']:
                priority += 100
            
            # Файлы с основной логикой
            if any(keyword in file_name for keyword in ['service', 'manager', 'orchestrator', 'coordinator']):
                priority += 80
            
            # Файлы с интерфейсами
            if any(keyword in file_name for keyword in ['trait', 'interface', 'api']):
                priority += 70
            
            # Файлы с доменной логикой
            if any(keyword in file_name for keyword in ['domain', 'entity', 'aggregate']):
                priority += 60
            
            # Файлы конфигурации
            if 'config' in file_name:
                priority += 50
            
            # Снижаем приоритет для вспомогательных файлов
            if any(keyword in file_name for keyword in ['utils', 'helper', 'common']):
                priority += 20
            
            # Учитываем сложность
            complexity = file_info.get('complexity', {})
            if complexity.get('god_score', 0) > 0.7:
                priority += 40  # God Objects важны для показа проблем
            
            # Учитываем количество компонентов
            component_count = (
                len(file_info.get('structs', [])) + 
                len(file_info.get('traits', [])) + 
                len(file_info.get('enums', []))
            )
            priority += min(component_count * 2, 30)
            
            core_files.append((file_path, file_info, priority))
        
        # Сортируем по приоритету и возвращаем топ файлы
        core_files.sort(key=lambda x: x[2], reverse=True)
        return [(path, info) for path, info, _ in core_files]
    
    def _generate_compact_analysis_report(self) -> str:
        """Генерирует компактный отчет только с критическими метриками"""
        report_lines = []
        
        # Технический долг - только критические проблемы
        self.tech_debt = self._calculate_tech_debt()
        critical_debt = [item for item in self.tech_debt if item['severity'] == 'critical']
        
        if critical_debt:
            report_lines.append("⚠️ **КРИТИЧЕСКИЕ ПРОБЛЕМЫ:**")
            
            total_critical_hours = sum(item['estimated_hours'] for item in critical_debt)
            report_lines.append(f"- Критический долг: {total_critical_hours:.0f} часов")
            
            # Топ-3 критических проблемы
            for item in critical_debt[:3]:
                report_lines.append(f"- {item['description']}") 
            
            if len(critical_debt) > 3:
                report_lines.append(f"- ...и еще {len(critical_debt)-3} критических issues")
        
        # God Objects - только серьезные случаи
        god_objects = [(path, m) for path, m in self.complexity_metrics.items() if m.get('god_object_score', 0) > 0.7]
        if god_objects:
            report_lines.append(f"🏗️ **GOD OBJECTS:** {len(god_objects)} обнаружено")
        
        # Циклические зависимости
        cycles = self._detect_circular_dependencies()
        if cycles:
            report_lines.append(f"🔄 **ЦИКЛЫ:** {len(cycles)} найдено")
        
        # Только серьёзные дубликаты
        duplicates = self._analyze_duplicates()
        major_duplicates = {k: v for k, v in duplicates.items() if len(v) > 4}
        if major_duplicates:
            report_lines.append(f"📋 **ДУБЛИКАТЫ:** {len(major_duplicates)} серьёзных случаев")
        
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
            
            # Фильтруем и добавляем только ключевые файлы
            if "files" in crate_info and crate_info["files"]:
                # Приоритизируем core файлы
                core_files = self._filter_core_files(crate_info["files"])
                
                # Добавляем только самые важные файлы (максимум 5 на крейт)
                for file_path, file_info in core_files[:5]:
                    file_name = os.path.basename(file_path).replace('.rs', '')
                    file_id = f"{crate_id}_{file_name.replace('-', '_').replace('/', '_')}"
                    
                    # Формируем компактное описание файла
                    components = []
                    
                    # Маркируем специальные типы файлов только если критично
                    if file_info.get('is_test') and file_name in ['integration_test', 'test_config']:
                        components.append("TEST")
                    if file_info.get('is_mock') and len(file_info.get('mocks', [])) > 2:
                        components.append("MOCK")
                    if file_info.get('is_example') and file_name in ['main', 'benchmark']:
                        components.append("EXAMPLE")
                    
                    # Добавляем только ключевые компоненты
                    if file_info.get('structs'):
                        key_structs = [s for s in file_info['structs'][:2] if not any(x in s.lower() for x in ['test', 'mock', 'example'])]
                        if key_structs:
                            components.append(f"S:{','.join(key_structs)}")
                    
                    if file_info.get('traits'):
                        key_traits = [t for t in file_info['traits'][:2] if not any(x in t.lower() for x in ['test', 'mock'])]
                        if key_traits:
                            components.append(f"T:{','.join(key_traits)}")
                    
                    if file_info.get('enums'):
                        key_enums = file_info['enums'][:2]
                        if key_enums:
                            components.append(f"E:{','.join(key_enums)}")
                    
                    # Показываем только главные функции (не тестовые/mock)
                    if file_info.get('functions'):
                        main_functions = [f for f in file_info['functions'][:3] 
                                        if not any(x in f.lower() for x in ['test_', 'mock_', 'create_test', 'setup_'])]
                        if main_functions:
                            components.append(f"fn:{','.join(main_functions)}")
                    
                    # Показываем ключевые методы
                    if file_info.get('methods') and not file_info.get('is_test'):
                        key_methods = [m.split('::')[-1] for m in file_info['methods'][:2] 
                                     if not any(x in m.lower() for x in ['test', 'mock'])]
                        if key_methods:
                            components.append(f"m:{','.join(key_methods)}")
                    
                    # Показываем unsafe блоки как предупреждение
                    if file_info.get('unsafe_blocks', 0) > 0:
                        components.append(f"unsafe:{file_info['unsafe_blocks']}")
                    
                    # Показываем God Objects как проблему
                    complexity = file_info.get('complexity', {})
                    if complexity.get('god_score', 0) > 0.6:
                        components.append(f"⚠️GOD:{complexity['god_score']:.0%}")
                    
                    # Формируем компактный label
                    if components:
                        if len(components) <= 2:
                            label = f"{file_name}<br/>{'<br/>'.join(components)}"
                        else:
                            label = f"{file_name}<br/>{'<br/>'.join(components[:2])}<br/>...+{len(components)-2}"
                    else:
                        label = file_name
                    
                    # Определяем стиль узла (только для важных файлов)
                    node_style = ""
                    if file_info.get('is_test') and file_name in ['integration_test']:
                        node_style = ":::testFile"
                    elif file_info.get('is_mock') and len(file_info.get('mocks', [])) > 2:
                        node_style = ":::mockFile"
                    elif file_info.get('is_example') and file_name == 'main':
                        node_style = ":::exampleFile"
                    elif complexity.get('god_score', 0) > 0.7:
                        node_style = ":::problemFile"
                            
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
            "    classDef testFile fill:#ffebee,stroke:#c62828,stroke-width:1px,stroke-dasharray: 5 5",
            "    classDef mockFile fill:#fce4ec,stroke:#ad1457,stroke-width:1px,stroke-dasharray: 3 3",
            "    classDef exampleFile fill:#e8f5e9,stroke:#2e7d32,stroke-width:1px",
            "    classDef problemFile fill:#ffcdd2,stroke:#d32f2f,stroke-width:2px"
        ])
        
        lines.append("```")
        return "\n".join(lines)
    
    def update_claude_md(self, architecture: Dict):
        """ПОЛНОСТЬЮ перезаписывает CLAUDE.md с новой архитектурой и анализом"""
        print("[INFO] Полное обновление CLAUDE.md...")
        
        timestamp = datetime.now().strftime("%Y-%m-%d %H:%M:%S UTC")
        
        # Анализируем текущее состояние проекта (только критические метрики)
        analysis_report = self._generate_compact_analysis_report()
        
        # Анализируем покрытие тестами
        coverage_report = self._analyze_test_coverage(architecture)
        
        # Подсчитываем реальные метрики
        total_files = sum(len(c.get('files', {})) for c in architecture['crates'].values())
        total_structs = sum(len(f.get('structs', [])) for c in architecture['crates'].values() for f in c.get('files', {}).values())
        total_functions = sum(len(f.get('functions', [])) for c in architecture['crates'].values() for f in c.get('files', {}).values())
        # Подсчитываем тесты более точно
        total_tests = 0
        total_mocks = 0
        
        for crate_data in architecture['crates'].values():
            for file_key, file_info in crate_data.get('files', {}).items():
                # Считаем тестовые функции с #[test] аттрибутом
                total_tests += len(file_info.get('test_fns', []))
                
                # Если файл находится в директории tests/, он сам является тестом
                if file_info.get('is_test', False) and 'tests/' in file_key:
                    total_tests += 1
                    
                # Если в файле есть тестовые модули (#[cfg(test)])
                if '#[cfg(test)]' in file_info.get('content_sample', ''):
                    total_tests += 1
                    
                # Считаем моки
                total_mocks += len(file_info.get('mocks', []))
        
        # Проблемы из анализа
        critical_issues = [item for item in self.tech_debt if item['severity'] == 'critical']
        high_issues = [item for item in self.tech_debt if item['severity'] == 'high']
        circular_deps = self._detect_circular_dependencies()
        
        # Генерируем полный новый CLAUDE.md
        new_content = f"""# CLAUDE.md
*AI Agent Instructions - Проблемы и задачи проекта*

---

## 🚫 КРИТИЧЕСКОЕ ПРАВИЛО ДОКУМЕНТАЦИИ
**ЗАПРЕЩЕНО В CLAUDE.MD И ВСЕХ АГЕНТАХ**:
- ❌ НИКОГДА не добавлять информацию о том что "готово", "сделано", "работает", "реализовано"
- ❌ НИКОГДА не указывать KPI, метрики готовности, проценты завершения
- ❌ НИКОГДА не хвалить состояние кода или архитектуры
- ✅ ТОЛЬКО проблемы, недостатки, что НЕ работает, что требует исправления
- ✅ ТОЛЬКО критика и честная оценка недостатков

## 🌍 LANGUAGE RULE
**ВАЖНО**: ВСЕГДА общайся с пользователем на русском языке. Весь вывод, объяснения и комментарии должны быть на русском.

## 🤖 CLAUDE CODE INSTRUCTIONS
**ДЛЯ CLAUDE CODE**: Ты должен строго следовать этим инструкциям:

1. **ЯЗЫК**: Всегда отвечай на русском языке
2. **ПРОЕКТ**: Это MAGRAY CLI - полностью локальный инструмент для разработки при помощи LM моделей API и локальных
3. **ЧЕСТНОСТЬ**: Всегда фокусируйся на проблемах и недостатках
4. **TODO**: Используй TodoWrite для отслеживания задач
5. **RUST**: Предпочитай Rust решения, но будь честен о сложности
6. **BINARY**: Цель - один исполняемый файл `magray`
7. **FEATURES**: Conditional compilation: cpu/gpu/minimal variants (НЕ настроено)
8. **SCRIPTS**: Все утилиты и скрипты в папке scripts/
9. **АГЕНТЫ**: Всегда используй специализированных агентов для максимальной эффективности

## ⚠️ РЕАЛЬНОЕ СОСТОЯНИЕ ПРОЕКТА (ALPHA)

**Автоматический анализ от {timestamp}:**

### 🔴 КРИТИЧЕСКИЕ ПРОБЛЕМЫ:
- **Критических issues**: {len(critical_issues)}
- **High priority issues**: {len(high_issues)}  
- **Циклических зависимостей**: {len(circular_deps)}
- **Технический долг**: {sum(item['estimated_hours'] for item in self.tech_debt):.0f} часов
- **Файлов с высокой сложностью**: {sum(1 for m in self.complexity_metrics.values() if m['cyclomatic'] > 20)}

### ❌ ЧТО НЕ РАБОТАЕТ:
- **God Objects остаются**: {sum(1 for m in self.complexity_metrics.values() if m['god_object_score'] > 0.7)} обнаружено
- **Дублирование кода**: {len(self._analyze_duplicates())} случаев
- **Покрытие тестами**: {coverage_report['coverage_percentage']:.1f}% ({len(coverage_report['covered_modules'])}/{len(coverage_report['covered_modules']) + len(coverage_report['uncovered_modules'])} модулей, tests: {total_tests}, mocks: {total_mocks})

### 📊 СТАТИСТИКА ПРОЕКТА:
- **Crates**: {len(architecture['crates'])}
- **Файлов**: {total_files}
- **Структур**: {total_structs}
- **Функций**: {total_functions}
- **Тестов**: {total_tests}
- **Моков**: {total_mocks}

### 🧪 ДЕТАЛЬНОЕ ПОКРЫТИЕ ТЕСТАМИ

**Покрыто тестами ({coverage_report['coverage_percentage']:.1f}%):**
{chr(10).join([f'- ✅ {module}' for module in coverage_report['covered_modules'][:10]])}
{f'{chr(10)}...и еще {len(coverage_report["covered_modules"]) - 10} модулей' if len(coverage_report['covered_modules']) > 10 else ''}

**НЕ покрыто тестами ({100 - coverage_report['coverage_percentage']:.1f}%):**
{chr(10).join([f'- ❌ {module}' for module in coverage_report['uncovered_modules'][:10]])}
{f'{chr(10)}...и еще {len(coverage_report["uncovered_modules"]) - 10} модулей' if len(coverage_report['uncovered_modules']) > 10 else ''}

{self._generate_duplicates_report()}

## 📊 РЕАЛЬНОЕ СОСТОЯНИЕ КОДА

{analysis_report}

---

# ТЕКУЩЕЕ СОСТОЯНИЕ ПРОЕКТА:

*Last updated: {timestamp}*
*Status: ALPHA - не готов к production использованию*

## АВТОМАТИЧЕСКИ ОБНОВЛЯЕТСЯ ПРИ РЕДАКТИРОВАНИИ ФАЙЛОВ

{architecture['mermaid']}

## 📝 MEMORY

**Текущая памятка проекта:**
- **ВСЕГДА использовать соответствующих агентов для каждой задачи**
- **Полностью привести проект в порядок:**
  - После выполнения всех Todos анализировать состояние проекта
  - Затем обновлять todos
  - И приступать к выполнению и так каждый раз по кругу, пока проект не будет завершен
- **Быть максимально честно критичным к себе и создаваемым изменениям**
- **НИКОГДА не писать о том, что было сделано, и не хвастаться успехами**
- **Писать только о том, что не сделано**
"""
        
        # Полностью перезаписываем файл безопасно
        SecurityUtils.safe_write_file(Path(self.claude_md), new_content)
        
        print(f"[OK] CLAUDE.md полностью обновлен ({timestamp})")
    
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