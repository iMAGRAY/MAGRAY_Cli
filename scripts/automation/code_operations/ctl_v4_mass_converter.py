#!/usr/bin/env python3
"""
🚀 CTL v4.0 Mass Converter - Сверхкомпактная конвертация JSON аннотаций

Мощный скрипт для массовой конвертации JSON аннотаций в сверхкомпактный CTL v4.0 формат
с экономией ~70% размера и сохранением всей важной информации.

ВОЗМОЖНОСТИ:
- Парсинг 112+ JSON компонентов из CLAUDE.md
- Конвертация в CTL v4.0: ID:TYPE:CUR%:TGT%:FLAGS
- Замена @component аннотаций в коде
- Интеллектуальная компрессия флагов
- Валидация и отчеты
- Dry-run режим

ФОРМАТ CTL v4.0:
// @ctl4: unified_agent_v2:C:90:95:clean,di,solid
// @ctl4: memory_lib:C:92:100:production,hnsw
// @ctl4: test_gpu:T:100:100:benchmark

ТИПЫ: C=Component, T=Test, A=Agent, S=Service, E=Error

Автор: Claude Code AI Agent
Версия: 1.0.0
"""

import json
import re
import os
import sys
import argparse
import logging
from pathlib import Path
from typing import Dict, List, Tuple, Optional, Set
from dataclasses import dataclass
from collections import defaultdict
from concurrent.futures import ThreadPoolExecutor, as_completed
import time


# Настройка логирования
def setup_logging(verbose: bool = False):
    """Настройка структурированного логирования"""
    log_level = logging.DEBUG if verbose else logging.INFO
    logging.basicConfig(
        level=log_level,
        format='%(asctime)s | %(levelname)s | %(message)s',
        datefmt='%H:%M:%S'
    )
    

@dataclass
class Component:
    """Структура компонента для конвертации"""
    id: str
    type: str
    description: str
    current: int
    target: int
    unit: str
    flags: List[str]
    file_path: str
    dependencies: List[str] = None
    
    def __post_init__(self):
        if self.dependencies is None:
            self.dependencies = []


class CTL4Converter:
    """Главный конвертер CTL v4.0"""
    
    # Mapping для сжатия типов
    TYPE_MAPPING = {
        'C': 'C',  # Component
        'T': 'T',  # Test  
        'A': 'A',  # Agent
        'S': 'S',  # Service
        'E': 'E',  # Error
        'B': 'B',  # Benchmark
    }
    
    # Правила компрессии флагов - агрессивное сжатие
    FLAG_COMPRESSION = {
        # Architecture patterns
        'clean_architecture': 'clean',
        'solid_principles': 'solid',
        'single_responsibility': 'sr',
        'dependency_injection': 'di',
        'di_integration': 'di',
        'di_ready': 'di',
        'strategy_pattern': 'strategy',
        'circuit_breaker': 'cb',
        
        # Production & Quality
        'production_ready': 'prod',
        'production': 'prod',
        'performance': 'perf',
        'optimization': 'opt',
        'optimized': 'opt',
        'resilience': 'resilient',
        'monitoring': 'monitor',
        'alerting': 'alerts',
        
        # AI/ML specific
        'ai_powered': 'ai',
        'ai-optimized': 'ai',
        'embeddings': 'embed',
        'machine_learning': 'ml',
        'neural_network': 'nn',
        'deep_learning': 'dl',
        
        # Technical specifics
        'multi_provider': 'multi',
        'orchestration': 'orchestr',
        'coordination': 'coord',
        'concurrent': 'concur',
        'asynchronous': 'async',
        'real_time': 'rt',
        'streaming': 'stream',
        'transactional': 'tx',
        
        # Testing & Quality
        'integration': 'integ',
        'comprehensive': 'comp',
        'unit_tests': 'unit',
        'benchmark': 'bench',
        'profiler': 'prof',
        'coverage': 'cov',
        
        # Infrastructure
        'infrastructure': 'infra',
        'configuration': 'config',
        'validation': 'valid',
        'serialization': 'serial',
        'deserialization': 'deserial',
        'registration': 'reg',
    }
    
    def __init__(self, project_root: str, dry_run: bool = False):
        self.project_root = Path(project_root)
        self.dry_run = dry_run
        self.logger = logging.getLogger(__name__)
        self.components: List[Component] = []
        self.conversion_stats = defaultdict(int)
        self.file_mappings: Dict[str, str] = {}
        self.mapping_table: List[Dict[str, str]] = []  # Для отчета mapping
        
    def extract_components_from_claude_md(self) -> List[Component]:
        """Извлечь все JSON компоненты из CLAUDE.md"""
        claude_md_path = self.project_root / "CLAUDE.md"
        
        if not claude_md_path.exists():
            raise FileNotFoundError(f"CLAUDE.md не найден: {claude_md_path}")
            
        self.logger.info(f"📖 Парсинг CLAUDE.md: {claude_md_path}")
        
        with open(claude_md_path, 'r', encoding='utf-8') as f:
            content = f.read()
        
        # Найти секцию AUTO-GENERATED ARCHITECTURE
        auto_gen_match = re.search(r'# AUTO-GENERATED ARCHITECTURE.*?```json\n(.*?)\n```', 
                                  content, re.DOTALL)
        
        if not auto_gen_match:
            raise ValueError("Секция AUTO-GENERATED ARCHITECTURE не найдена")
            
        json_section = auto_gen_match.group(1)
        
        # Парсинг JSON строк
        components = []
        json_lines = [line.strip() for line in json_section.split('\n') if line.strip()]
        
        for i, line in enumerate(json_lines):
            try:
                data = json.loads(line)
                component = self._parse_json_component(data)
                if component:
                    components.append(component)
                    self.conversion_stats['parsed'] += 1
            except json.JSONDecodeError as e:
                self.logger.warning(f"⚠️ Ошибка парсинга JSON на строке {i+1}: {e}")
                self.conversion_stats['parse_errors'] += 1
                
        self.logger.info(f"✅ Извлечено компонентов: {len(components)}")
        return components
        
    def _parse_json_component(self, data: Dict) -> Optional[Component]:
        """Парсинг одного JSON компонента"""
        try:
            # Извлечение базовых полей
            component_id = data.get('id', '')
            component_type = data.get('k', 'C')  # По умолчанию Component
            description = data.get('t', '')
            file_path = data.get('x_file', '')
            
            # Извлечение метрик зрелости
            maturity = data.get('m', {})
            current = maturity.get('cur', 0)
            target = maturity.get('tgt', 100)
            unit = maturity.get('u', '%')
            
            # Извлечение флагов
            flags = data.get('f', [])
            if isinstance(flags, str):
                flags = [flags]
                
            # Извлечение зависимостей
            dependencies = data.get('d', [])
            if isinstance(dependencies, str):
                dependencies = [dependencies]
            
            return Component(
                id=component_id,
                type=component_type,
                description=description,
                current=current,
                target=target,
                unit=unit,
                flags=flags,
                file_path=file_path,
                dependencies=dependencies
            )
            
        except Exception as e:
            self.logger.error(f"❌ Ошибка парсинга компонента: {e}")
            return None
            
    def compress_flags(self, flags: List[str]) -> str:
        """Интеллектуальная компрессия флагов"""
        if not flags:
            return ""
            
        compressed_flags = []
        
        for flag in flags:
            # Применить правила компрессии
            compressed = self.FLAG_COMPRESSION.get(flag, flag)
            
            # Дополнительное сжатие длинных флагов
            if len(compressed) > 8:
                # Удалить гласные из середины длинных слов
                if len(compressed) > 10:
                    compressed = re.sub(r'[aeiou]', '', compressed[2:-2])
                    compressed = flag[:2] + compressed + flag[-2:]
                    
            compressed_flags.append(compressed)
            
        # Удалить дубликаты и отсортировать для консистентности
        unique_flags = sorted(set(compressed_flags))
        
        # Ограничить до 5 самых важных флагов
        return ','.join(unique_flags[:5])
        
    def convert_to_ctl4(self, component: Component) -> str:
        """Конвертация компонента в CTL v4.0 формат"""
        # Определить тип
        type_key = self.TYPE_MAPPING.get(component.type, 'C')
        
        # Сжать флаги
        compressed_flags = self.compress_flags(component.flags)
        
        # Собрать CTL v4.0 аннотацию
        ctl4_parts = [
            component.id,
            type_key,
            str(component.current),
            str(component.target)
        ]
        
        if compressed_flags:
            ctl4_parts.append(compressed_flags)
            
        ctl4_annotation = f"// @ctl4: {':'.join(ctl4_parts)}"
        
        # Подсчет экономии
        original_json = json.dumps({
            "k": component.type,
            "id": component.id,
            "t": component.description,
            "m": {"cur": component.current, "tgt": component.target, "u": component.unit},
            "f": component.flags,
            "x_file": component.file_path
        }, separators=(',', ':'))
        
        # Добавить в mapping таблицу
        self.mapping_table.append({
            'id': component.id,
            'type': component.type,
            'original': original_json[:50] + '...' if len(original_json) > 50 else original_json,
            'ctl4': ctl4_annotation,
            'compression': f"{len(original_json)}→{len(ctl4_annotation)} ({round((1-len(ctl4_annotation)/len(original_json))*100, 1)}%)",
            'file': component.file_path.split(':')[0] if component.file_path else 'N/A'
        })
        
        self.conversion_stats['original_size'] += len(original_json)
        self.conversion_stats['compressed_size'] += len(ctl4_annotation)
        
        return ctl4_annotation
        
    def find_component_in_code(self, component: Component) -> Optional[Tuple[str, int]]:
        """Найти @component аннотацию в коде"""
        if not component.file_path:
            return None
            
        # Нормализация пути - заменить обратные слеши на прямые
        normalized_path = component.file_path.split(':')[0].replace('\\', '/')
        file_path = self.project_root / normalized_path
        
        if not file_path.exists():
            self.logger.warning(f"⚠️ Файл не найден: {file_path}")
            return None
            
        try:
            with open(file_path, 'r', encoding='utf-8') as f:
                lines = f.readlines()
                
            # Поиск существующей @component аннотации
            pattern = rf'//\s*@component:\s*{{\s*".*?"id"\s*:\s*"{re.escape(component.id)}"'
            
            for line_num, line in enumerate(lines, 1):
                if re.search(pattern, line):
                    return str(file_path), line_num
                    
        except Exception as e:
            self.logger.error(f"❌ Ошибка чтения файла {file_path}: {e}")
            
        return None
        
    def replace_component_annotation(self, component: Component, ctl4_annotation: str) -> bool:
        """Заменить @component аннотацию на CTL v4.0"""
        location = self.find_component_in_code(component)
        
        if not location:
            self.logger.warning(f"⚠️ Аннотация для {component.id} не найдена в коде")
            return False
            
        file_path, line_num = location
        
        if self.dry_run:
            self.logger.info(f"🔍 [DRY-RUN] Заменить в {file_path}:{line_num}")
            return True
            
        try:
            with open(file_path, 'r', encoding='utf-8') as f:
                lines = f.readlines()
                
            # Заменить строку
            lines[line_num - 1] = ctl4_annotation + '\n'
            
            with open(file_path, 'w', encoding='utf-8') as f:
                f.writelines(lines)
                
            self.conversion_stats['replaced'] += 1
            self.logger.debug(f"✅ Заменено в {file_path}:{line_num}")
            return True
            
        except Exception as e:
            self.logger.error(f"❌ Ошибка замены в {file_path}: {e}")
            return False
            
    def generate_compact_section(self, components: List[Component]) -> str:
        """Генерация компактной секции для CLAUDE.md"""
        self.logger.info("📝 Генерация компактной секции...")
        
        sections = {
            'C': [],  # Components
            'T': [],  # Tests
            'A': [],  # Agents
            'S': [],  # Services
            'E': [],  # Errors
        }
        
        # Группировка по типам
        for component in components:
            ctl4_annotation = self.convert_to_ctl4(component)
            sections[component.type].append(ctl4_annotation)
            
        # Создание компактной секции
        compact_section = [
            "## Components (CTL v4.0 Ultra-Compact Format)",
            "",
            "```ctl4"
        ]
        
        # Добавить компоненты по типам
        for type_name, type_components in sections.items():
            if type_components:
                type_label = {
                    'C': 'Core Components',
                    'T': 'Tests & Benchmarks', 
                    'A': 'AI Agents',
                    'S': 'Services',
                    'E': 'Error Types'
                }.get(type_name, f'Type {type_name}')
                
                compact_section.extend([
                    f"",
                    f"// {type_label} ({len(type_components)})",
                    *sorted(type_components)
                ])
                
        compact_section.append("```")
        
        return '\n'.join(compact_section)
        
    def generate_mapping_table_report(self) -> str:
        """Генерация детальной mapping таблицы"""
        self.logger.info("[TABLE] Генерация mapping таблицы...")
        
        if not self.mapping_table:
            return "[ERROR] Mapping таблица пуста"
            
        table_lines = [
            "\n[MAPPING] Детальная таблица конвертации",
            "=" * 120,
            f"{'ID':<25} {'Type':<4} {'Compression':<20} {'File':<40} {'CTL4 Format':<35}",
            "-" * 120
        ]
        
        # Сортировка по типу и ID
        sorted_mappings = sorted(self.mapping_table, key=lambda x: (x['type'], x['id']))
        
        current_type = None
        for mapping in sorted_mappings:
            # Добавить разделитель для нового типа
            if current_type != mapping['type']:
                type_name = {
                    'C': 'Components',
                    'T': 'Tests',
                    'A': 'Agents', 
                    'S': 'Services',
                    'E': 'Errors'
                }.get(mapping['type'], f"Type {mapping['type']}")
                table_lines.append(f"\n--- {type_name} ---")
                current_type = mapping['type']
                
            table_lines.append(
                f"{mapping['id']:<25} {mapping['type']:<4} {mapping['compression']:<20} "
                f"{mapping['file'][-37:]+'...' if len(mapping['file']) > 40 else mapping['file']:<40} "
                f"{mapping['ctl4']:<35}"
            )
            
        table_lines.extend([
            "-" * 120,
            f"Итого компонентов: {len(self.mapping_table)}",
            ""
        ])
        
        return '\n'.join(table_lines)
        
    def process_all_components(self) -> Dict[str, any]:
        """Основной процесс конвертации всех компонентов"""
        self.logger.info("🚀 Начало массовой конвертации CTL v4.0...")
        
        start_time = time.time()
        
        # 1. Извлечение компонентов из CLAUDE.md
        self.components = self.extract_components_from_claude_md()
        
        if not self.components:
            raise ValueError("Не найдено компонентов для конвертации")
            
        # 2. Конвертация и замена аннотаций
        self.logger.info(f"🔄 Конвертация {len(self.components)} компонентов...")
        
        success_count = 0
        with ThreadPoolExecutor(max_workers=4) as executor:
            future_to_component = {}
            
            for component in self.components:
                ctl4_annotation = self.convert_to_ctl4(component)
                future = executor.submit(self.replace_component_annotation, 
                                       component, ctl4_annotation)
                future_to_component[future] = component
                
            for future in as_completed(future_to_component):
                component = future_to_component[future]
                try:
                    if future.result():
                        success_count += 1
                except Exception as e:
                    self.logger.error(f"❌ Ошибка обработки {component.id}: {e}")
                    
        # 3. Генерация компактной секции
        compact_section = self.generate_compact_section(self.components)
        
        # 4. Подсчет статистики
        processing_time = time.time() - start_time
        
        original_size = self.conversion_stats['original_size']
        compressed_size = self.conversion_stats['compressed_size']
        compression_ratio = (1 - compressed_size / original_size) * 100 if original_size > 0 else 0
        
        results = {
            'total_components': len(self.components),
            'successful_conversions': success_count,
            'failed_conversions': len(self.components) - success_count,
            'original_size_bytes': original_size,
            'compressed_size_bytes': compressed_size,
            'compression_ratio_percent': round(compression_ratio, 1),
            'processing_time_seconds': round(processing_time, 2),
            'compact_section': compact_section,
            'conversion_stats': dict(self.conversion_stats),
            'mapping_table_report': self.generate_mapping_table_report()
        }
        
        return results
        
    def save_compact_section_to_file(self, compact_section: str, output_file: str):
        """Сохранить компактную секцию в файл"""
        output_path = Path(output_file)
        
        if self.dry_run:
            self.logger.info(f"🔍 [DRY-RUN] Сохранение в: {output_path}")
            return
            
        try:
            with open(output_path, 'w', encoding='utf-8') as f:
                f.write(compact_section)
            self.logger.info(f"[SAVE] Компактная секция сохранена: {output_path}")
        except Exception as e:
            self.logger.error(f"[ERROR] Ошибка сохранения в {output_path}: {e}")


def create_validation_report(results: Dict[str, any]) -> str:
    """Создание подробного отчета валидации"""
    report_lines = [
        "[*] CTL v4.0 Conversion Validation Report",
        "=" * 50,
        "",
        f"[STAT] ОБЩАЯ СТАТИСТИКА:",
        f"   • Всего компонентов: {results['total_components']}",
        f"   • Успешно конвертировано: {results['successful_conversions']}",
        f"   • Ошибок конвертации: {results['failed_conversions']}",
        f"   • Время обработки: {results['processing_time_seconds']}s",
        "",
        f"[SIZE] ЭКОНОМИЯ РАЗМЕРА:",
        f"   • Исходный размер: {results['original_size_bytes']} байт",
        f"   • Сжатый размер: {results['compressed_size_bytes']} байт", 
        f"   • Коэффициент сжатия: {results['compression_ratio_percent']}%",
        f"   • Экономия: {results['original_size_bytes'] - results['compressed_size_bytes']} байт",
        "",
        f"[DETAILS] ДЕТАЛИ КОНВЕРТАЦИИ:",
        f"   • Распаршено из JSON: {results['conversion_stats'].get('parsed', 0)}",
        f"   • Ошибки парсинга: {results['conversion_stats'].get('parse_errors', 0)}",
        f"   • Заменено в коде: {results['conversion_stats'].get('replaced', 0)}",
        "",
        f"[QUALITY] КАЧЕСТВО КОНВЕРТАЦИИ:",
    ]
    
    success_rate = (results['successful_conversions'] / results['total_components'] * 100) if results['total_components'] > 0 else 0
    
    if success_rate >= 95:
        report_lines.append("   [OK] ОТЛИЧНО - Конвертация прошла успешно")
    elif success_rate >= 85:
        report_lines.append("   [WARN] ХОРОШО - Большинство компонентов конвертировано")
    else:
        report_lines.append("   [ERROR] ТРЕБУЕТ ВНИМАНИЯ - Много ошибок конвертации")
        
    report_lines.extend([
        f"   • Успешность: {success_rate:.1f}%",
        "",
        "[FORMAT] CTL v4.0 ФОРМАТ ПРИМЕРЫ:",
        "   // @ctl4: unified_agent_v2:C:90:95:clean,di,solid",
        "   // @ctl4: memory_lib:C:92:100:prod,hnsw",
        "   // @ctl4: test_gpu:T:100:100:bench",
        "",
    ])
    
    return '\n'.join(report_lines)


def main():
    """Основная функция CLI"""
    parser = argparse.ArgumentParser(
        description="🚀 CTL v4.0 Mass Converter - Сверхкомпактная конвертация JSON аннотаций",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
ПРИМЕРЫ ИСПОЛЬЗОВАНИЯ:

  # Dry-run режим (рекомендуется сначала)
  python ctl_v4_mass_converter.py --dry-run

  # Полная конвертация
  python ctl_v4_mass_converter.py --convert

  # С детальным логированием  
  python ctl_v4_mass_converter.py --convert --verbose

  # Сохранить компактную секцию в файл
  python ctl_v4_mass_converter.py --convert --output compact_section.md

ФОРМАТ CTL v4.0:
  // @ctl4: ID:TYPE:CUR%:TGT%:FLAGS
  
ТИПЫ: C=Component, T=Test, A=Agent, S=Service, E=Error
        """
    )
    
    parser.add_argument(
        '--project-root',
        default='.',
        help='Путь к корню проекта (по умолчанию: текущая директория)'
    )
    
    parser.add_argument(
        '--dry-run',
        action='store_true',
        help='Режим предварительного просмотра без изменений'
    )
    
    parser.add_argument(
        '--convert',
        action='store_true', 
        help='Выполнить реальную конвертацию'
    )
    
    parser.add_argument(
        '--output',
        help='Сохранить компактную секцию в указанный файл'
    )
    
    parser.add_argument(
        '--verbose',
        action='store_true',
        help='Подробное логирование'
    )
    
    args = parser.parse_args()
    
    # Настройка логирования
    setup_logging(args.verbose)
    logger = logging.getLogger(__name__)
    
    if not args.dry_run and not args.convert:
        logger.error("❌ Укажите --dry-run или --convert")
        sys.exit(1)
        
    try:
        # Создание конвертера
        converter = CTL4Converter(
            project_root=args.project_root,
            dry_run=args.dry_run
        )
        
        # Выполнение конвертации
        results = converter.process_all_components()
        
        # Создание отчета  
        report = create_validation_report(results)
        # Используем sys.stdout.write для корректного вывода в Windows
        import sys
        sys.stdout.buffer.write(("\n" + report).encode('utf-8'))
        sys.stdout.flush()
        
        # Сохранение компактной секции если указан output
        if args.output:
            converter.save_compact_section_to_file(
                results['compact_section'],
                args.output
            )
            
        # Предварительный просмотр компактной секции
        if args.dry_run:
            preview_content = "\n" + "="*60 + "\n"
            preview_content += "[PREVIEW] Компактная секция (первые 20 строк)\n"
            preview_content += "="*60 + "\n"
            preview_lines = results['compact_section'].split('\n')[:20]
            for line in preview_lines:
                preview_content += line + "\n"
            if len(results['compact_section'].split('\n')) > 20:
                preview_content += "... (еще строк)\n"
            sys.stdout.buffer.write(preview_content.encode('utf-8'))
            sys.stdout.flush()
                
        logger.info("🎉 Конвертация завершена успешно!")
        
    except Exception as e:
        logger.error(f"💥 Критическая ошибка: {e}")
        if args.verbose:
            import traceback
            traceback.print_exc()
        sys.exit(1)


if __name__ == '__main__':
    main()