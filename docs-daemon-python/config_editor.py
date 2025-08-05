#!/usr/bin/env python3
"""
Settings Editor - Простой редактор настроек JSON для CTL Sync

Интерактивный редактор для управления файлом settings.json без необходимости
редактировать JSON вручную.
"""

# @component: {"k":"C","id":"config_editor","t":"Interactive JSON config editor","m":{"cur":95,"tgt":100,"u":"%"},"f":["python","config","editor","json","interactive"]}

import json
import sys
from pathlib import Path
from typing import Any, Dict

try:
    import click
except ImportError:
    print("Error: click library required. Install with: pip install click")
    sys.exit(1)

from ctl_sync.json_config import CtlJsonConfig
from ctl_sync.utils import colored_print


@click.group()
@click.option('--config', '-c', type=click.Path(path_type=Path), 
              help='Path to settings.json file')
@click.pass_context
def cli(ctx, config):
    """Редактор настроек CTL Sync (settings.json)"""
    ctx.ensure_object(dict)
    if config:
        ctx.obj['config_file'] = config
    else:
        ctx.obj['config_file'] = Path('settings.json')


@cli.command()
@click.pass_context
def show(ctx):
    """Показать текущие настройки"""
    config_file = ctx.obj['config_file']
    
    if not config_file.exists():
        colored_print(f"Settings file not found: {config_file}", "red")
        return
    
    config = CtlJsonConfig(config_file)
    
    colored_print("Current Settings", "cyan", bold=True)
    colored_print("=" * 50, "cyan")
    
    # Pretty print the entire config (with ASCII encoding for Windows compatibility)
    print(json.dumps(config.config, indent=2, ensure_ascii=True))


@cli.command()
@click.argument('key')
@click.pass_context
def get(ctx, key):
    """Получить значение по ключу (например: project.name)"""
    config_file = ctx.obj['config_file']
    config = CtlJsonConfig(config_file)
    
    value = config.get(key)
    if value is not None:
        if isinstance(value, (dict, list)):
            print(json.dumps(value, indent=2, ensure_ascii=False))
        else:
            print(value)
    else:
        colored_print(f"Key not found: {key}", "red")


@cli.command()
@click.argument('key')
@click.argument('value')
@click.pass_context
def set(ctx, key, value):
    """Установить значение по ключу (например: project.name "My Project")"""
    config_file = ctx.obj['config_file']
    config = CtlJsonConfig(config_file)
    
    # Try to parse JSON values
    try:
        # Try parsing as JSON first
        parsed_value = json.loads(value)
    except json.JSONDecodeError:
        # If not JSON, treat as string
        parsed_value = value
    
    config.set(key, parsed_value)
    config.save()
    
    colored_print(f"Set {key} = {parsed_value}", "green")


@cli.command()
@click.pass_context
def create_default(ctx):
    """Создать файл настроек по умолчанию"""
    config_file = ctx.obj['config_file']
    
    if config_file.exists():
        if not click.confirm(f"File {config_file} exists. Overwrite?"):
            return
    
    config = CtlJsonConfig(config_file)
    colored_print(f"Created default settings file: {config_file}", "green")


@cli.command()
@click.pass_context  
def validate(ctx):
    """Проверить корректность настроек"""
    config_file = ctx.obj['config_file']
    
    if not config_file.exists():
        colored_print(f"Settings file not found: {config_file}", "red")
        return
    
    try:
        config = CtlJsonConfig(config_file)
        colored_print("Settings file is valid!", "green")
        
        # Basic validation checks
        required_sections = ['project', 'scanning', 'parsing']
        for section in required_sections:
            if section not in config.config:
                colored_print(f"Warning: Missing section '{section}'", "yellow")
        
        # Check project settings
        if not config.get_crates_dir():
            colored_print("Warning: No crates directory specified", "yellow")
        
        if not config.get_claude_md_file():
            colored_print("Warning: No CLAUDE.md file specified", "yellow")
        
        colored_print("Validation complete", "blue")
        
    except Exception as e:
        colored_print(f"Settings validation failed: {e}", "red")


@cli.command()
@click.pass_context
def interactive(ctx):
    """Интерактивный режим настройки"""
    config_file = ctx.obj['config_file']
    config = CtlJsonConfig(config_file)
    
    colored_print("Interactive Settings Editor", "cyan", bold=True)
    colored_print("=" * 50, "cyan")
    
    # Common settings to configure
    settings_menu = [
        ("project.name", "Project name", str),
        ("project.description", "Project description", str),
        ("project.crates_dir", "Crates directory", str),
        ("project.claude_md_file", "CLAUDE.md filename", str),
        ("scanning.exclude_dirs", "Directories to exclude", list),
        ("scanning.file_extensions", "File extensions to scan", list),
        ("parsing.ctl2.enabled", "Enable CTL v2.0 parsing", bool),
        ("parsing.ctl3.enabled", "Enable CTL v3.0 parsing", bool),
        ("features.colored_output", "Enable colored output", bool),
        ("features.statistics", "Enable statistics", bool),
    ]
    
    colored_print("Configure common settings:", "blue")
    
    for key, description, value_type in settings_menu:
        current_value = config.get(key)
        
        if value_type == bool:
            new_value = click.confirm(f"{description} (current: {current_value})", 
                                     default=current_value)
        elif value_type == list:
            colored_print(f"{description} (current: {current_value})", "white")
            new_value_str = click.prompt("Enter comma-separated values (or press Enter to keep current)", 
                                        default="", show_default=False)
            if new_value_str.strip():
                new_value = [item.strip() for item in new_value_str.split(',')]
            else:
                new_value = current_value
        else:
            new_value = click.prompt(f"{description}", default=current_value)
        
        if new_value != current_value:
            config.set(key, new_value)
            colored_print(f"  Updated {key}", "green")
    
    config.save()
    colored_print("Settings saved!", "green", bold=True)


@cli.command()  
@click.argument('pattern', required=False)
@click.pass_context
def search(ctx, pattern):
    """Поиск настроек по ключевому слову"""
    config_file = ctx.obj['config_file']
    config = CtlJsonConfig(config_file)
    
    if not pattern:
        pattern = click.prompt("Enter search pattern")
    
    pattern = pattern.lower()
    found_keys = []
    
    def search_dict(d, prefix=""):
        for key, value in d.items():
            full_key = f"{prefix}.{key}" if prefix else key
            
            if pattern in full_key.lower() or (isinstance(value, str) and pattern in value.lower()):
                found_keys.append((full_key, value))
            
            if isinstance(value, dict):
                search_dict(value, full_key)
    
    search_dict(config.config)
    
    if found_keys:
        colored_print(f"Found {len(found_keys)} matches for '{pattern}':", "blue")
        for key, value in found_keys:
            if isinstance(value, (dict, list)):
                value_str = json.dumps(value, ensure_ascii=False)[:50] + "..."
            else:
                value_str = str(value)
            colored_print(f"  {key}: {value_str}", "white")
    else:
        colored_print(f"No matches found for '{pattern}'", "yellow")


if __name__ == '__main__':
    cli()