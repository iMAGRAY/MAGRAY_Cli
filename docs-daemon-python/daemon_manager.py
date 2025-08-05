#!/usr/bin/env python3
"""
Daemon Manager - Автоматическое управление CTL Sync демоном
Используется claude-md-orchestrator агентом для управления lifecycle демона
"""

# @component: {"k":"C","id":"daemon_manager","t":"Python CTL daemon lifecycle manager","m":{"cur":95,"tgt":100,"u":"%"},"f":["python","daemon","lifecycle","ctl","orchestration"]}

import json
import os
import subprocess
import sys
import time
from pathlib import Path
from typing import Dict, List, Optional

class DaemonManager:
    """Менеджер для управления CTL Sync демоном"""
    
    def __init__(self, project_root: Optional[Path] = None):
        self.project_root = project_root or Path.cwd()
        if self.project_root.name == "docs-daemon-python":
            self.project_root = self.project_root.parent
        
        self.daemon_dir = self.project_root / "docs-daemon-python"
        self.settings_file = self.daemon_dir / "settings.json"
        self.claude_md_file = self.project_root / "CLAUDE.md"
        
    def extract_ctl_patterns_from_claude_md(self) -> Dict[str, any]:
        """Извлекает CTL паттерны и правила из CLAUDE.md"""
        if not self.claude_md_file.exists():
            print(f"Warning: CLAUDE.md not found at {self.claude_md_file}")
            return {}
        
        try:
            with open(self.claude_md_file, 'r', encoding='utf-8') as f:
                content = f.read()
        except Exception as e:
            print(f"Error reading CLAUDE.md: {e}")
            return {}
        
        patterns = {
            "ctl2_patterns": [],
            "ctl3_patterns": [],
            "tensor_symbols": {
                "unicode": [],
                "ascii": []
            },
            "validation_rules": {}
        }
        
        # Извлекаем CTL v2.0 паттерны
        if "// @component:" in content:
            patterns["ctl2_patterns"] = [
                "//\\\\s*@component:\\\\s*(\\\\{.*\\\\})",
                "//\\\\s*@test_component:\\\\s*(\\\\{.*\\\\})"
            ]
        
        # Извлекаем CTL v3.0 паттерны  
        if "// @ctl3:" in content or "Ⱦ[" in content or "D[" in content:
            patterns["ctl3_patterns"] = [
                "//\\\\s*@ctl3:\\\\s*([ⱧD]\\\\[.*?\\\\]\\\\s*:=\\\\s*\\\\{[^}]*\\\\})",
                "//\\\\s*@tensor:\\\\s*([ⱧD]\\\\[.*?\\\\]\\\\s*:=\\\\s*\\\\{[^}]*\\\\})"
            ]
        
        # Извлекаем тензорные операторы из спецификации
        ctl_spec_start = content.find("# CTL v3.0 - CLAUDE TENSOR LANGUAGE SPECIFICATION")
        if ctl_spec_start != -1:
            ctl_spec_section = content[ctl_spec_start:ctl_spec_start + 10000]  # Первые 10k символов
            
            # Unicode операторы
            unicode_ops = ["⊗", "⊕", "⊙", "⊡", "∇", "∂", "∴", "∵", "≡", "⟹", "⟷", "Ⱦ"]
            for op in unicode_ops:
                if op in ctl_spec_section:
                    patterns["tensor_symbols"]["unicode"].append(op)
            
            # ASCII альтернативы
            ascii_mappings = {
                "⊗": "compose", "⊕": "parallel", "∇": "grad", "∂": "partial",
                "⟹": "implies", "Ⱦ": "D"
            }
            for unicode_op in patterns["tensor_symbols"]["unicode"]:
                if unicode_op in ascii_mappings:
                    patterns["tensor_symbols"]["ascii"].append(ascii_mappings[unicode_op])
        
        return patterns
    
    def update_daemon_settings(self, ctl_patterns: Dict[str, any]) -> bool:
        """Обновляет settings.json с новыми CTL паттернами"""
        if not self.settings_file.exists():
            print(f"Warning: Settings file not found at {self.settings_file}")
            return False
        
        try:
            with open(self.settings_file, 'r', encoding='utf-8') as f:
                settings = json.load(f)
        except Exception as e:
            print(f"Error reading settings.json: {e}")
            return False
        
        # Обновляем CTL v2.0 паттерны
        if ctl_patterns.get("ctl2_patterns"):
            if "parsing" not in settings:
                settings["parsing"] = {}
            if "ctl2" not in settings["parsing"]:
                settings["parsing"]["ctl2"] = {}
            
            settings["parsing"]["ctl2"]["patterns"] = ctl_patterns["ctl2_patterns"]
            settings["parsing"]["ctl2"]["enabled"] = True
        
        # Обновляем CTL v3.0 паттерны
        if ctl_patterns.get("ctl3_patterns"):
            if "parsing" not in settings:
                settings["parsing"] = {}
            if "ctl3" not in settings["parsing"]:
                settings["parsing"]["ctl3"] = {}
            
            settings["parsing"]["ctl3"]["patterns"] = ctl_patterns["ctl3_patterns"]
            settings["parsing"]["ctl3"]["enabled"] = True
            
            # Обновляем тензорные символы
            if ctl_patterns.get("tensor_symbols"):
                settings["parsing"]["ctl3"]["tensor_symbols"] = ctl_patterns["tensor_symbols"]
        
        # Сохраняем обновленные настройки
        try:
            with open(self.settings_file, 'w', encoding='utf-8') as f:
                json.dump(settings, f, indent=2, ensure_ascii=False)
            print("Success: Settings updated successfully")
            return True
        except Exception as e:
            print(f"Error writing settings.json: {e}")
            return False
    
    def stop_daemon(self) -> bool:
        """Останавливает запущенные процессы демона"""
        try:
            # Windows
            if os.name == 'nt':
                result = subprocess.run([
                    'taskkill', '/f', '/im', 'python.exe', '/fi', 
                    'CommandLine like *ctl-sync*watch*'
                ], capture_output=True, text=True)
                if result.returncode == 0:
                    print("Success: Daemon stopped successfully")
                    return True
                else:
                    print("Info: No daemon process found to stop")
                    return True
            
            # Linux/macOS
            else:
                result = subprocess.run([
                    'pkill', '-f', 'ctl-sync watch'
                ], capture_output=True, text=True)
                print("Success: Daemon stopped successfully")
                return True
                
        except Exception as e:
            print(f"Error stopping daemon: {e}")
            return False
    
    def start_daemon(self) -> bool:
        """Запускает демон в фоновом режиме"""
        try:
            # Переходим в директорию демона
            os.chdir(self.daemon_dir)
            
            # Windows
            if os.name == 'nt':
                # Запускаем через Python модуль напрямую в фоне
                subprocess.Popen([
                    sys.executable, '-m', 'ctl_sync.main', 'watch'
                ], creationflags=subprocess.CREATE_NO_WINDOW)
            
            # Linux/macOS  
            else:
                subprocess.Popen([
                    'ctl-sync', 'watch'
                ], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
            
            print("Success: Daemon started in background")
            return True
            
        except Exception as e:
            print(f"Error starting daemon: {e}")
            return False
    
    def restart_daemon(self) -> bool:
        """Перезапускает демон"""
        print("Restarting CTL Sync daemon...")
        
        # Останавливаем
        self.stop_daemon()
        
        # Ждем немного
        time.sleep(2)
        
        # Запускаем
        return self.start_daemon()
    
    def validate_daemon(self) -> bool:
        """Проверяет что демон работает"""
        try:
            # Проверяем что процесс запущен
            if os.name == 'nt':
                result = subprocess.run([
                    'tasklist', '/fi', 'CommandLine like *ctl_sync.main*watch*'
                ], capture_output=True, text=True)
                
                if 'python.exe' in result.stdout or 'python' in result.stdout.lower():
                    print("Success: Daemon is running")
                    return True
                else:
                    # Альтернативная проверка через wmic
                    result2 = subprocess.run([
                        'wmic', 'process', 'where', 'CommandLine like "%ctl_sync.main%"', 'get', 'ProcessId,CommandLine'
                    ], capture_output=True, text=True)
                    
                    if 'ctl_sync.main' in result2.stdout:
                        print("Success: Daemon is running")
                        return True
                    else:
                        print("Error: Daemon is not running")
                        return False
            else:
                result = subprocess.run([
                    'pgrep', '-f', 'ctl-sync watch'
                ], capture_output=True, text=True)
                
                if result.returncode == 0:
                    print("Success: Daemon is running")
                    return True
                else:
                    print("Error: Daemon is not running")
                    return False
                    
        except Exception as e:
            print(f"Error validating daemon: {e}")
            return False
    
    def full_sync_and_restart(self) -> bool:
        """Полная синхронизация CTL правил и перезапуск демона"""
        print("Full CTL sync and daemon restart...")
        
        # 1. Извлекаем CTL паттерны из CLAUDE.md
        print("Extracting CTL patterns from CLAUDE.md...")
        ctl_patterns = self.extract_ctl_patterns_from_claude_md()
        
        if not ctl_patterns:
            print("Warning: No CTL patterns found in CLAUDE.md")
            return False
        
        # 2. Обновляем настройки демона
        print("Updating daemon settings...")
        if not self.update_daemon_settings(ctl_patterns):
            print("Error: Failed to update daemon settings")
            return False
        
        # 3. Перезапускаем демон
        print("Restarting daemon...")
        if not self.restart_daemon():
            print("Error: Failed to restart daemon")
            return False
        
        # 4. Валидируем что демон работает
        time.sleep(3)  # Ждем запуска
        if not self.validate_daemon():
            print("Error: Daemon validation failed")
            return False
        
        print("Success: Full sync and restart completed successfully!")
        return True

def main():
    """CLI интерфейс для управления демоном"""
    import argparse
    
    parser = argparse.ArgumentParser(description="CTL Sync Daemon Manager")
    parser.add_argument('action', choices=['start', 'stop', 'restart', 'validate', 'sync'],
                        help='Action to perform')
    parser.add_argument('--project-root', type=Path, 
                        help='Project root directory')
    
    args = parser.parse_args()
    
    manager = DaemonManager(args.project_root)
    
    if args.action == 'start':
        success = manager.start_daemon()
    elif args.action == 'stop':
        success = manager.stop_daemon()
    elif args.action == 'restart':
        success = manager.restart_daemon()
    elif args.action == 'validate':
        success = manager.validate_daemon()
    elif args.action == 'sync':
        success = manager.full_sync_and_restart()
    
    sys.exit(0 if success else 1)

if __name__ == '__main__':
    main()