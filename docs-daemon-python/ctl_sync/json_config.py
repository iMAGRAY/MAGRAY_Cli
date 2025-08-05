"""
JSON Configuration Management

Centralized configuration system for CTL sync daemon with JSON settings support.
"""

# @component: {"k":"C","id":"json_config_manager","t":"JSON configuration system","m":{"cur":95,"tgt":100,"u":"%"},"f":["python","config","json","management","settings"]}

import json
import os
from pathlib import Path
from typing import Dict, Any, List, Optional


class CtlJsonConfig:
    """CTL Sync configuration manager with JSON settings"""
    
    def __init__(self, config_file: Path = None):
        self.config_file = config_file or Path("settings.json")
        self.config = self._load_default_config()
        
        if self.config_file.exists():
            self._load_config_file()
        else:
            # Create default settings file if it doesn't exist
            self._create_default_settings()
    
    def _load_default_config(self) -> Dict[str, Any]:
        """Load minimal default configuration as fallback"""
        return {
            "project": {
                "name": "MAGRAY_CLI",
                "crates_dir": "crates",
                "claude_md_file": "CLAUDE.md"
            },
            "scanning": {
                "exclude_dirs": ["target", ".git", "__pycache__"],
                "file_extensions": [".rs", ".toml"],
                "max_file_size_mb": 5
            },
            "parsing": {
                "ctl2": {"enabled": True},
                "ctl3": {"enabled": True}
            },
            "validation": {
                "component": {
                    "required_fields": ["k", "id", "t"],
                    "valid_kinds": ["T", "A", "B", "F", "M", "S", "R", "P", "D", "C", "E"]
                }
            },
            "output": {
                "claude_md": {"auto_update": True}
            },
            "features": {
                "colored_output": True,
                "statistics": True
            }
        }
    
    def _create_default_settings(self) -> None:
        """Create default settings.json file"""
        default_settings = {
            "project": {
                "name": "MAGRAY_CLI",
                "description": "Production-ready Rust AI агент с многослойной памятью",
                "crates_dir": "crates",
                "claude_md_file": "CLAUDE.md",
                "file_extensions": [".rs", ".toml"]
            },
            "scanning": {
                "exclude_dirs": ["target", "node_modules", ".git", "__pycache__"],
                "file_extensions": [".rs", ".toml"],
                "max_file_size_mb": 5
            },
            "parsing": {
                "ctl2": {"enabled": True},
                "ctl3": {"enabled": True}
            },
            "features": {
                "colored_output": True,
                "statistics": True
            }
        }
        
        try:
            with open(self.config_file, 'w', encoding='utf-8') as f:
                json.dump(default_settings, f, indent=2, ensure_ascii=False)
        except Exception as e:
            print(f"Warning: Could not create default settings file: {e}")
    
    def _load_config_file(self) -> None:
        """Load configuration from JSON file"""
        try:
            with open(self.config_file, 'r', encoding='utf-8') as f:
                file_config = json.load(f)
            
            self._merge_config(file_config)
            
        except Exception as e:
            print(f"Warning: Failed to load config file {self.config_file}: {e}")
    
    def _merge_config(self, file_config: Dict[str, Any]) -> None:
        """Merge file configuration with defaults"""
        def merge_dict(base: Dict, overlay: Dict) -> Dict:
            result = base.copy()
            for key, value in overlay.items():
                if key in result and isinstance(result[key], dict) and isinstance(value, dict):
                    result[key] = merge_dict(result[key], value)
                else:
                    result[key] = value
            return result
        
        self.config = merge_dict(self.config, file_config)
    
    def get(self, key_path: str, default: Any = None) -> Any:
        """Get configuration value by dot-separated path"""
        keys = key_path.split('.')
        value = self.config
        
        for key in keys:
            if isinstance(value, dict) and key in value:
                value = value[key]
            else:
                return default
        
        return value
    
    def set(self, key_path: str, value: Any) -> None:
        """Set configuration value by dot-separated path"""
        keys = key_path.split('.')
        current = self.config
        
        for key in keys[:-1]:
            if key not in current:
                current[key] = {}
            current = current[key]
        
        current[keys[-1]] = value
    
    def save(self) -> None:
        """Save current configuration to file"""
        try:
            with open(self.config_file, 'w', encoding='utf-8') as f:
                json.dump(self.config, f, indent=2, ensure_ascii=False)
        except Exception as e:
            print(f"Error saving configuration: {e}")
    
    def reload(self) -> None:
        """Reload configuration from file"""
        if self.config_file.exists():
            self._load_config_file()
    
    # Convenience methods for common settings
    def get_project_name(self) -> str:
        """Get project name"""
        return self.get("project.name", "Unknown Project")
    
    def get_crates_dir(self) -> str:
        """Get crates directory name"""
        return self.get("project.crates_dir", "crates")
    
    def get_claude_md_file(self) -> str:
        """Get CLAUDE.md filename"""
        return self.get("project.claude_md_file", "CLAUDE.md")
    
    def get_file_extensions(self) -> List[str]:
        """Get list of file extensions to scan"""
        return self.get("scanning.file_extensions", [".rs"])
    
    def get_exclude_dirs(self) -> List[str]:
        """Get list of directories to exclude"""
        return self.get("scanning.exclude_dirs", [])
    
    def get_exclude_files(self) -> List[str]:
        """Get list of file patterns to exclude"""
        return self.get("scanning.exclude_files", [])
    
    def get_max_file_size_mb(self) -> int:
        """Get maximum file size in MB"""
        return self.get("scanning.max_file_size_mb", 5)
    
    def is_ctl2_enabled(self) -> bool:
        """Check if CTL v2.0 parsing is enabled"""
        return self.get("parsing.ctl2.enabled", True)
    
    def is_ctl3_enabled(self) -> bool:
        """Check if CTL v3.0 parsing is enabled"""
        return self.get("parsing.ctl3.enabled", True)
    
    def get_ctl2_patterns(self) -> List[str]:
        """Get CTL v2.0 regex patterns"""
        return self.get("parsing.ctl2.patterns", ["//\\s*@component:\\s*(\\{.*\\})"])
    
    def get_ctl3_patterns(self) -> List[str]:
        """Get CTL v3.0 regex patterns"""
        return self.get("parsing.ctl3.patterns", ["//\\s*@ctl3:\\s*([ⱧD]\\[.*?\\]\\s*:=\\s*\\{[^}]*\\})"])
    
    def get_tensor_symbols(self) -> Dict[str, List[str]]:
        """Get tensor symbol mappings"""
        return self.get("parsing.ctl3.tensor_symbols", {
            "unicode": ["⊗", "⊕", "∇"],
            "ascii": ["compose", "parallel", "grad"]
        })
    
    def should_auto_update_claude_md(self) -> bool:
        """Check if CLAUDE.md should be auto-updated"""
        return self.get("output.claude_md.auto_update", True)
    
    def should_backup_before_update(self) -> bool:
        """Check if backup should be made before updating"""
        return self.get("output.claude_md.backup_before_update", True)
    
    def get_section_marker(self) -> str:
        """Get CLAUDE.md section marker"""
        return self.get("output.claude_md.section_marker", "# AUTO-GENERATED ARCHITECTURE")
    
    def get_valid_kinds(self) -> List[str]:
        """Get valid component kinds"""
        return self.get("validation.component.valid_kinds", ["T", "A", "B", "F", "M", "S", "R", "P", "D", "C", "E"])
    
    def get_required_fields(self) -> List[str]:
        """Get required component fields"""
        return self.get("validation.component.required_fields", ["k", "id", "t"])
    
    def get_kind_mapping(self) -> Dict[str, str]:
        """Get component kind to name mapping"""
        return self.get("kind_mapping", {
            'T': 'Test', 'A': 'Agent', 'B': 'Batch', 'F': 'Function',
            'M': 'Module', 'S': 'Service', 'R': 'Resource', 'P': 'Process',
            'D': 'Data', 'C': 'Component', 'E': 'Error'
        })
    
    def get_auto_inference_rules(self) -> Dict[str, str]:
        """Get auto-inference rules for component kinds"""
        return self.get("auto_inference.rules", {
            "test": "T", "agent": "A", "batch": "B", "function": "F",
            "module": "M", "service": "S", "component": "C"
        })
    
    def get_keyword_flags(self) -> Dict[str, List[str]]:
        """Get keyword to flag mappings"""
        return self.get("auto_inference.keywords", {
            "gpu": ["gpu", "cuda"],
            "ai": ["ai", "ml", "neural"],
            "async": ["async", "await"]
        })
    
    def is_colored_output_enabled(self) -> bool:
        """Check if colored output is enabled"""
        return self.get("features.colored_output", True)
    
    def is_statistics_enabled(self) -> bool:
        """Check if statistics are enabled"""
        return self.get("features.statistics", True)
    
    def get_debounce_seconds(self) -> float:
        """Get file watching debounce time"""
        return self.get("watching.debounce_seconds", 2.0)
    
    def get_log_level(self) -> str:
        """Get logging level"""
        return self.get("logging.level", "INFO")
    
    def get_log_format(self) -> str:
        """Get logging format"""
        return self.get("logging.format", "%(asctime)s - %(name)s - %(levelname)s - %(message)s")