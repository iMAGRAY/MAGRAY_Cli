#!/usr/bin/env python3
"""
Автоматизированный скрипт для замены .unwrap() на proper error handling в Rust коде.
Использует AST-подобный анализ для безопасной замены с учетом контекста.
"""

import os
import re
import sys
import argparse
from pathlib import Path
from typing import List, Tuple, Optional
from dataclasses import dataclass
from collections import defaultdict

@dataclass
class UnwrapOccurrence:
    file_path: str
    line_number: int
    line_content: str
    context_before: List[str]
    context_after: List[str]
    suggested_fix: str
    fix_type: str  # 'result', 'option', 'expect', 'map_err'

class UnwrapFixer:
    def __init__(self, dry_run: bool = True, verbose: bool = False):
        self.dry_run = dry_run
        self.verbose = verbose
        self.stats = defaultdict(int)
        
    def find_unwraps(self, file_path: Path) -> List[UnwrapOccurrence]:
        """Находит все вызовы .unwrap() в файле с контекстом."""
        occurrences = []
        
        try:
            with open(file_path, 'r', encoding='utf-8') as f:
                lines = f.readlines()
        except Exception as e:
            print(f"Error reading {file_path}: {e}")
            return occurrences
            
        for i, line in enumerate(lines):
            if '.unwrap()' in line:
                # Получаем контекст (3 строки до и после)
                context_before = lines[max(0, i-3):i]
                context_after = lines[i+1:min(len(lines), i+4)]
                
                # Анализируем контекст для определения типа fix
                fix_type, suggested_fix = self.analyze_context(
                    line, context_before, context_after, i
                )
                
                occurrence = UnwrapOccurrence(
                    file_path=str(file_path),
                    line_number=i + 1,
                    line_content=line.rstrip(),
                    context_before=[l.rstrip() for l in context_before],
                    context_after=[l.rstrip() for l in context_after],
                    suggested_fix=suggested_fix,
                    fix_type=fix_type
                )
                occurrences.append(occurrence)
                
        return occurrences
    
    def analyze_context(self, line: str, before: List[str], after: List[str], 
                        line_num: int) -> Tuple[str, str]:
        """Анализирует контекст и предлагает подходящий fix."""
        
        # Проверяем, находимся ли мы в функции, возвращающей Result
        in_result_fn = any('-> Result' in l or '-> anyhow::Result' in l 
                          for l in before[-10:] if l)
        
        # Проверяем, это тест или нет
        in_test = any('#[test]' in l or '#[cfg(test)]' in l 
                     for l in before[-20:] if l)
        
        # Получаем выражение перед .unwrap()
        unwrap_match = re.search(r'(\S+)\.unwrap\(\)', line)
        if not unwrap_match:
            return 'unknown', line
            
        expr = unwrap_match.group(1)
        
        # Определяем отступ
        indent = len(line) - len(line.lstrip())
        indent_str = ' ' * indent
        
        # В тестах можно использовать expect с описательным сообщением
        if in_test:
            if 'Some(' in line or 'Ok(' in line:
                return 'expect', line.replace('.unwrap()', '.expect("test assertion failed")')
            return 'expect', line.replace('.unwrap()', '.expect("test value should be present")')
        
        # В функциях, возвращающих Result
        if in_result_fn:
            # Для простых случаев используем ?
            if line.strip().endswith('.unwrap()') or line.strip().endswith('.unwrap();'):
                return 'result', line.replace('.unwrap()', '?')
            
            # Для более сложных случаев с context
            if 'env::' in line or 'std::env' in line:
                return 'context', line.replace('.unwrap()', 
                    '.context("Failed to read environment variable")?')
            elif 'File::' in line or 'fs::' in line:
                return 'context', line.replace('.unwrap()', 
                    '.context("Failed to perform file operation")?')
            elif 'parse' in line.lower():
                return 'context', line.replace('.unwrap()', 
                    '.context("Failed to parse value")?')
            else:
                return 'result', line.replace('.unwrap()', '?')
        
        # В остальных случаях используем unwrap_or_default или expect
        if 'Option' in str(before) or 'Some(' in line:
            return 'option', line.replace('.unwrap()', '.unwrap_or_default()')
        
        # Fallback на expect с описательным сообщением
        context_hint = self.guess_context_from_expr(expr, line)
        return 'expect', line.replace('.unwrap()', f'.expect("{context_hint}")')
    
    def guess_context_from_expr(self, expr: str, line: str) -> str:
        """Пытается угадать контекст из выражения для сообщения об ошибке."""
        if 'lock' in expr.lower():
            return "Failed to acquire lock"
        elif 'channel' in expr.lower() or 'recv' in expr.lower():
            return "Channel communication failed"
        elif 'parse' in expr.lower():
            return "Failed to parse value"
        elif 'file' in expr.lower() or 'path' in expr.lower():
            return "File operation failed"
        elif 'env' in expr.lower():
            return "Environment variable not found"
        elif 'config' in expr.lower():
            return "Configuration error"
        elif 'db' in expr.lower() or 'database' in expr.lower():
            return "Database operation failed"
        else:
            return "Operation failed"
    
    def fix_file(self, file_path: Path) -> bool:
        """Исправляет unwrap() в одном файле."""
        occurrences = self.find_unwraps(file_path)
        
        if not occurrences:
            return False
            
        if self.verbose:
            print(f"\n📄 {file_path}")
            print(f"   Found {len(occurrences)} unwrap() calls")
        
        if self.dry_run:
            for occ in occurrences:
                print(f"\n  Line {occ.line_number}: {occ.fix_type}")
                print(f"  - {occ.line_content}")
                print(f"  + {occ.suggested_fix}")
            return True
            
        # Читаем файл
        with open(file_path, 'r', encoding='utf-8') as f:
            lines = f.readlines()
        
        # Применяем fixes (в обратном порядке, чтобы не сбить номера строк)
        for occ in reversed(occurrences):
            lines[occ.line_number - 1] = occ.suggested_fix + '\n'
            self.stats[occ.fix_type] += 1
        
        # Записываем обратно
        with open(file_path, 'w', encoding='utf-8') as f:
            f.writelines(lines)
            
        print(f"✅ Fixed {len(occurrences)} unwrap() calls in {file_path}")
        return True
    
    def add_anyhow_context_import(self, file_path: Path):
        """Добавляет use anyhow::Context если необходимо."""
        with open(file_path, 'r', encoding='utf-8') as f:
            content = f.read()
            
        if 'context(' in content.lower() and 'use anyhow::Context' not in content:
            # Находим место для вставки после других use statements
            lines = content.split('\n')
            insert_idx = 0
            
            for i, line in enumerate(lines):
                if line.startswith('use '):
                    insert_idx = i + 1
                elif insert_idx > 0 and not line.startswith('use '):
                    break
                    
            if insert_idx > 0:
                lines.insert(insert_idx, 'use anyhow::Context;')
                with open(file_path, 'w', encoding='utf-8') as f:
                    f.write('\n'.join(lines))
                print(f"  Added anyhow::Context import to {file_path}")
    
    def process_directory(self, dir_path: Path, exclude_patterns: List[str] = None):
        """Обрабатывает все .rs файлы в директории."""
        exclude_patterns = exclude_patterns or ['target/', 'tests/', '.git/']
        
        rs_files = []
        for root, dirs, files in os.walk(dir_path):
            # Исключаем определенные директории
            dirs[:] = [d for d in dirs if not any(p in f"{root}/{d}" for p in exclude_patterns)]
            
            for file in files:
                if file.endswith('.rs'):
                    rs_files.append(Path(root) / file)
        
        print(f"🔍 Found {len(rs_files)} Rust files to process\n")
        
        modified_files = []
        total_unwraps = 0
        
        for file_path in rs_files:
            occurrences = self.find_unwraps(file_path)
            if occurrences:
                total_unwraps += len(occurrences)
                if not self.dry_run:
                    self.fix_file(file_path)
                    self.add_anyhow_context_import(file_path)
                modified_files.append((file_path, len(occurrences)))
        
        # Печатаем статистику
        print(f"\n📊 Summary:")
        print(f"  Total unwrap() calls found: {total_unwraps}")
        print(f"  Files with unwrap(): {len(modified_files)}")
        
        if not self.dry_run:
            print(f"\n  Fixes applied:")
            for fix_type, count in self.stats.items():
                print(f"    {fix_type}: {count}")
        
        # Топ файлов с наибольшим количеством unwrap
        if modified_files:
            print(f"\n  Top files with most unwrap() calls:")
            for file_path, count in sorted(modified_files, key=lambda x: x[1], reverse=True)[:10]:
                print(f"    {file_path}: {count}")

def main():
    parser = argparse.ArgumentParser(description='Fix .unwrap() calls in Rust code')
    parser.add_argument('path', nargs='?', default='crates/', 
                       help='Path to directory or file to process')
    parser.add_argument('--dry-run', action='store_true', default=True,
                       help='Show what would be changed without modifying files')
    parser.add_argument('--apply', action='store_true',
                       help='Actually apply the fixes')
    parser.add_argument('--verbose', '-v', action='store_true',
                       help='Verbose output')
    parser.add_argument('--exclude', nargs='*', default=['target/', 'tests/'],
                       help='Patterns to exclude from processing')
    
    args = parser.parse_args()
    
    # Если указан --apply, отключаем dry-run
    if args.apply:
        args.dry_run = False
        print("⚠️  Running in APPLY mode - files will be modified!")
        response = input("Continue? (y/n): ")
        if response.lower() != 'y':
            print("Aborted.")
            return
    
    fixer = UnwrapFixer(dry_run=args.dry_run, verbose=args.verbose)
    
    path = Path(args.path)
    if path.is_file():
        fixer.fix_file(path)
    elif path.is_dir():
        fixer.process_directory(path, exclude_patterns=args.exclude)
    else:
        print(f"Error: {path} not found")
        sys.exit(1)

if __name__ == '__main__':
    main()