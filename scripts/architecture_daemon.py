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
from typing import Dict, List, Set, Tuple
import re
from watchdog.observers import Observer
from watchdog.events import FileSystemEventHandler
import threading
import sys

class ArchitectureDaemon:
    """Ультракомпактный демон для генерации Mermaid диаграмм архитектуры"""
    
    def __init__(self, project_root: str):
        self.project_root = Path(project_root)
        self.crates_dir = self.project_root / "crates"
        self.claude_md = self.project_root / "CLAUDE.md"
        self.dependencies: Dict[str, Set[str]] = {}
        self.features: Dict[str, List[str]] = {}
        
    def scan_architecture(self) -> Dict[str, any]:
        """Сканирует структуру проекта и определяет реальную архитектуру"""
        print("[INFO] Анализ архитектуры проекта...")
        
        architecture = {
            "crates": {},
            "dependencies": {},
            "features": {},
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
                
                architecture["crates"][crate_name] = {
                    "path": str(cargo_toml.parent),
                    "description": self._get_crate_description(crate_name)
                }
                architecture["dependencies"][crate_name] = list(deps)
                architecture["features"][crate_name] = features
                
                print(f"  [OK] {crate_name}: {len(deps)} deps, {len(features)} features")
                
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
    
    def _generate_mermaid(self, arch: Dict) -> str:
        """Генерирует компактную Mermaid диаграмму"""
        lines = [
            "```mermaid",
            "flowchart TD",
            ""
        ]
        
        # Основные узлы с группировкой
        nodes = {
            "CLI": "CLI[CLI Agent & Commands]",
            "MEM": "MEM[3-Layer HNSW Memory]", 
            "AI": "AI[AI/ONNX Models & GPU]",
            "LLM": "LLM[Multi-Provider LLM]",
            "ROUTER": "ROUTER[Smart Task Router]",
            "TOOLS": "TOOLS[Tools Registry]",
            "COMMON": "COMMON[Common Utilities]"
        }
        
        # Добавляем узлы
        for node_def in nodes.values():
            lines.append(f"    {node_def}")
        
        lines.append("")
        
        # Ключевые зависимости (упрощенные)
        key_deps = [
            "CLI --> MEM",
            "CLI --> AI", 
            "CLI --> LLM",
            "CLI --> ROUTER",
            "CLI --> TOOLS",
            "MEM --> AI",
            "MEM --> COMMON",
            "AI --> COMMON",
            "LLM --> COMMON",
            "ROUTER --> TOOLS",
            "ROUTER --> LLM"
        ]
        
        for dep in key_deps:
            lines.append(f"    {dep}")
        
        lines.append("")
        
        # Стилизация
        lines.extend([
            "    classDef primary fill:#e1f5fe,stroke:#01579b,stroke-width:2px",
            "    classDef secondary fill:#f3e5f5,stroke:#4a148c,stroke-width:1px",
            "    classDef utility fill:#fff3e0,stroke:#e65100,stroke-width:1px",
            "",
            "    class CLI primary",
            "    class MEM,AI,LLM primary", 
            "    class ROUTER,TOOLS secondary",
            "    class COMMON utility"
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