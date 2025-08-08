#!/usr/bin/env python3
"""
Анализатор временной линии изменений в истории Cursor IDE
Восстанавливает последовательность редактирования файлов
"""

import os
import json
from pathlib import Path
from datetime import datetime
from urllib.parse import unquote
import re
from collections import defaultdict
from typing import List, Dict, Tuple

# Путь к истории Cursor
CURSOR_HISTORY_PATH = Path(r"C:\Users\1\AppData\Roaming\Cursor\User\History")

# Целевая директория проекта
PROJECT_PATH = "MAGRAY_Cli"

def extract_file_path_from_resource(resource_url: str) -> Path:
    """Извлечь путь файла из URL ресурса"""
    match = re.search(r'file:///(.*)', resource_url)
    if match:
        encoded_path = match.group(1)
        decoded_path = unquote(encoded_path).replace('/', '\\')
        decoded_path = decoded_path.replace('%3A', ':').replace('c:', 'C:')
        return Path(decoded_path)
    return None

def parse_timestamp(timestamp: int) -> datetime:
    """Конвертировать timestamp в datetime"""
    return datetime.fromtimestamp(timestamp / 1000)

def find_all_changes() -> List[Dict]:
    """Найти все изменения файлов проекта MAGRAY_Cli с временными метками"""
    all_changes = []
    
    for history_dir in CURSOR_HISTORY_PATH.iterdir():
        if not history_dir.is_dir():
            continue
            
        entries_file = history_dir / "entries.json"
        if not entries_file.exists():
            continue
            
        try:
            with open(entries_file, 'r', encoding='utf-8') as f:
                data = json.load(f)
                
            resource = data.get('resource', '')
            if PROJECT_PATH not in resource:
                continue
                
            file_path = extract_file_path_from_resource(resource)
            if not file_path:
                continue
                
            entries = data.get('entries', [])
            for entry in entries:
                history_file = history_dir / entry['id']
                if history_file.exists():
                    timestamp = entry.get('timestamp', 0)
                    
                    # Читаем первые строки файла для контекста
                    preview = ""
                    try:
                        with open(history_file, 'r', encoding='utf-8', errors='ignore') as f:
                            lines = f.readlines()[:5]
                            preview = ''.join(lines)[:200]
                            total_lines = len(f.readlines()) + 5
                    except:
                        preview = "[Unable to read file]"
                        total_lines = 0
                    
                    change_info = {
                        'timestamp': timestamp,
                        'datetime': parse_timestamp(timestamp),
                        'file_path': file_path,
                        'relative_path': file_path.relative_to(Path("C:\\Users\\1\\Documents\\GitHub\\MAGRAY_Cli")) if "MAGRAY_Cli" in str(file_path) else file_path.name,
                        'history_file': history_file,
                        'history_dir': history_dir.name,
                        'entry_id': entry['id'],
                        'preview': preview,
                        'total_lines': total_lines
                    }
                    all_changes.append(change_info)
                    
        except Exception as e:
            print(f"Error reading {entries_file}: {e}")
            
    return sorted(all_changes, key=lambda x: x['timestamp'])

def group_by_time_windows(changes: List[Dict], window_minutes: int = 5) -> List[List[Dict]]:
    """Группировать изменения по временным окнам"""
    if not changes:
        return []
    
    groups = []
    current_group = [changes[0]]
    
    for change in changes[1:]:
        time_diff = (change['datetime'] - current_group[-1]['datetime']).total_seconds() / 60
        
        if time_diff <= window_minutes:
            current_group.append(change)
        else:
            groups.append(current_group)
            current_group = [change]
    
    if current_group:
        groups.append(current_group)
    
    return groups

def analyze_change_patterns(changes: List[Dict]) -> Dict:
    """Анализировать паттерны изменений"""
    file_changes = defaultdict(list)
    
    for change in changes:
        rel_path = str(change['relative_path'])
        file_changes[rel_path].append(change)
    
    # Найти наиболее часто изменяемые файлы
    most_changed = sorted(
        [(path, len(changes_list)) for path, changes_list in file_changes.items()],
        key=lambda x: x[1],
        reverse=True
    )[:10]
    
    return {
        'total_files': len(file_changes),
        'total_changes': len(changes),
        'most_changed': most_changed,
        'file_changes': file_changes
    }

def create_restoration_script(changes: List[Dict]) -> str:
    """Создать скрипт для восстановления изменений в правильном порядке"""
    script_lines = [
        "#!/usr/bin/env python3",
        "# Скрипт восстановления изменений в хронологическом порядке",
        "",
        "import shutil",
        "from pathlib import Path",
        "import os",
        "",
        "def restore_changes():",
        "    changes = ["
    ]
    
    for change in changes:
        script_lines.append(f"        {{")
        script_lines.append(f"            'source': r'{change['history_file']}',")
        script_lines.append(f"            'target': r'{change['file_path']}',")
        script_lines.append(f"            'timestamp': '{change['datetime']}',")
        script_lines.append(f"        }},")
    
    script_lines.extend([
        "    ]",
        "",
        "    print(f'Restoring {len(changes)} file versions...')",
        "    ",
        "    for i, change in enumerate(changes, 1):",
        "        source = Path(change['source'])",
        "        target = Path(change['target'])",
        "        ",
        "        # Create directories if needed",
        "        target.parent.mkdir(parents=True, exist_ok=True)",
        "        ",
        "        # Copy file",
        "        try:",
        "            shutil.copy2(source, target)",
        "            print(f'[{i}/{len(changes)}] Restored: {target.name} ({change[\"timestamp\"]})')",
        "        except Exception as e:",
        "            print(f'[{i}/{len(changes)}] Failed: {target.name} - {e}')",
        "",
        "if __name__ == '__main__':",
        "    restore_changes()",
    ])
    
    return '\n'.join(script_lines)

def main():
    print("=" * 80)
    print("TIMELINE ANALYSIS FOR MAGRAY_Cli PROJECT")
    print("=" * 80)
    print()
    
    print("Scanning Cursor history...")
    changes = find_all_changes()
    
    if not changes:
        print("ERROR: No project changes found in history")
        return
    
    print(f"Found {len(changes)} file versions\n")
    
    # Временной диапазон
    first_change = changes[0]['datetime']
    last_change = changes[-1]['datetime']
    duration = last_change - first_change
    
    print(f"Период изменений:")
    print(f"   Начало:  {first_change.strftime('%Y-%m-%d %H:%M:%S')}")
    print(f"   Конец:   {last_change.strftime('%Y-%m-%d %H:%M:%S')}")
    print(f"   Длительность: {duration}")
    print()
    
    # Анализ паттернов
    patterns = analyze_change_patterns(changes)
    
    print(f"Статистика изменений:")
    print(f"   Всего файлов изменено: {patterns['total_files']}")
    print(f"   Всего версий сохранено: {patterns['total_changes']}")
    print()
    
    print("Наиболее часто изменяемые файлы:")
    for path, count in patterns['most_changed']:
        print(f"   {count:3}x - {path}")
    print()
    
    # Группировка по сессиям работы
    sessions = group_by_time_windows(changes, window_minutes=10)
    
    print(f"Сессии работы (группировка по 10 минут):")
    for i, session in enumerate(sessions, 1):
        session_start = session[0]['datetime']
        session_end = session[-1]['datetime']
        duration = (session_end - session_start).total_seconds() / 60
        
        print(f"\n   Сессия {i}: {session_start.strftime('%H:%M')} - {session_end.strftime('%H:%M')} ({duration:.1f} мин)")
        print(f"   Изменено файлов: {len(set(str(c['relative_path']) for c in session))}")
        
        # Показываем уникальные файлы в сессии
        unique_files = list(set(str(c['relative_path']) for c in session))[:5]
        for file_path in unique_files:
            print(f"      • {file_path}")
        if len(unique_files) > 5:
            print(f"      ... и еще {len(set(str(c['relative_path']) for c in session)) - 5} файлов")
    
    print("\n" + "=" * 80)
    print("ДЕТАЛЬНАЯ ХРОНОЛОГИЯ ИЗМЕНЕНИЙ")
    print("=" * 80 + "\n")
    
    # Показываем последние N изменений с деталями
    recent_changes = changes[-30:] if len(changes) > 30 else changes
    
    for i, change in enumerate(recent_changes, 1):
        print(f"[{i:3}] {change['datetime'].strftime('%H:%M:%S')} - {change['relative_path']}")
        print(f"     Директория: {change['history_dir']}")
        print(f"     ID: {change['entry_id']}")
        print(f"     Размер: ~{change['total_lines']} строк")
        
        # Показываем preview кода
        if change['preview'] and change['preview'] != "[Unable to read file]":
            preview_lines = change['preview'].split('\n')[:2]
            for line in preview_lines:
                if line.strip():
                    print(f"     > {line[:80]}...")
                    break
        print()
    
    if len(changes) > 30:
        print(f"... показаны последние 30 из {len(changes)} изменений")
    
    # Создаем скрипт восстановления
    print("\n" + "=" * 80)
    print("СОЗДАНИЕ СКРИПТА ВОССТАНОВЛЕНИЯ")
    print("=" * 80 + "\n")
    
    restoration_script = create_restoration_script(changes)
    script_path = Path("restore_timeline.py")
    
    with open(script_path, 'w', encoding='utf-8') as f:
        f.write(restoration_script)
    
    print(f"[OK] Скрипт восстановления создан: {script_path}")
    print(f"   Содержит {len(changes)} изменений в хронологическом порядке")
    print()
    print("Для восстановления всех изменений выполните:")
    print("   python restore_timeline.py")
    print()
    print("ВНИМАНИЕ: Это перезапишет текущие файлы!")
    print("   Рекомендуется сначала сделать backup:")
    print("   git stash   # или   git commit -am 'Backup before restore'")

if __name__ == "__main__":
    main()