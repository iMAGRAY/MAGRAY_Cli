"""
Configuration Management

Flexible configuration system for CTL sync daemon.
"""

import os
import yaml
from pathlib import Path
from typing import Any, Dict, List, Optional
from pydantic import BaseModel, Field


class ParserConfig(BaseModel):
    """Parser-specific configuration"""
    enabled: bool = True
    patterns: Dict[str, str] = Field(default_factory=dict)
    flags: List[str] = Field(default_factory=list)


class CtlSyncConfig(BaseModel):
    """Main CTL sync configuration"""
    
    # Paths
    crates_path: str = "crates"
    claude_path: str = "CLAUDE.md"
    cache_path: str = "cache.json"
    
    # File filtering
    include_patterns: List[str] = Field(default_factory=lambda: ["**/*.rs"])
    exclude_patterns: List[str] = Field(default_factory=lambda: ["**/target/**", "**/.git/**"])
    
    # Parsers
    ctl2_parser: ParserConfig = Field(default_factory=ParserConfig)
    ctl3_parser: ParserConfig = Field(default_factory=ParserConfig)
    
    # Watching
    watch_debounce_seconds: float = 0.5
    watch_recursive: bool = True
    
    # Validation
    strict_validation: bool = True
    schema_validation: bool = True
    
    # Output
    pretty_json: bool = False
    colored_output: bool = True
    verbose_logging: bool = False
    
    # Performance
    max_file_size_mb: int = 10
    parallel_processing: bool = False
    cache_enabled: bool = True


class ConfigManager:
    """Configuration manager with multiple sources"""
    
    DEFAULT_CONFIG_NAMES = [
        'ctl-sync.yaml',
        'ctl-sync.yml', 
        '.ctl-sync.yaml',
        '.ctl-sync.yml'
    ]
    
    def __init__(self, config_path: Optional[Path] = None):
        self.config_path = config_path
        self.config = self.load_config()
    
    def load_config(self) -> CtlSyncConfig:
        """Load configuration from file, environment, and defaults"""
        
        # Start with default config
        config_dict = {}
        
        # Try to load from file
        config_file = self.find_config_file()
        if config_file:
            config_dict.update(self.load_from_file(config_file))
        
        # Override with environment variables
        config_dict.update(self.load_from_env())
        
        # Create and validate config
        return CtlSyncConfig(**config_dict)
    
    def find_config_file(self) -> Optional[Path]:
        """Find configuration file"""
        if self.config_path and self.config_path.exists():
            return self.config_path
        
        # Look in current directory and parents
        current = Path.cwd()
        while current != current.parent:
            for config_name in self.DEFAULT_CONFIG_NAMES:
                config_file = current / config_name
                if config_file.exists():
                    return config_file
            current = current.parent
        
        return None
    
    def load_from_file(self, config_file: Path) -> Dict[str, Any]:
        """Load configuration from YAML file"""
        try:
            with open(config_file, 'r', encoding='utf-8') as f:
                config_data = yaml.safe_load(f) or {}
            return config_data
        except (yaml.YAMLError, IOError) as e:
            print(f"Warning: Failed to load config from {config_file}: {e}")
            return {}
    
    def load_from_env(self) -> Dict[str, Any]:
        """Load configuration from environment variables"""
        config = {}
        
        # Environment variable mapping
        env_mapping = {
            'CTL_SYNC_CRATES_PATH': 'crates_path',
            'CTL_SYNC_CLAUDE_PATH': 'claude_path', 
            'CTL_SYNC_CACHE_PATH': 'cache_path',
            'CTL_SYNC_STRICT_VALIDATION': 'strict_validation',
            'CTL_SYNC_VERBOSE': 'verbose_logging',
            'CTL_SYNC_COLORED_OUTPUT': 'colored_output',
            'CTL_SYNC_DEBOUNCE_SECONDS': 'watch_debounce_seconds',
        }
        
        for env_key, config_key in env_mapping.items():
            value = os.getenv(env_key)
            if value is not None:
                # Type conversion
                if config_key in ['strict_validation', 'verbose_logging', 'colored_output']:
                    config[config_key] = value.lower() in ('true', '1', 'yes', 'on')
                elif config_key == 'watch_debounce_seconds':
                    try:
                        config[config_key] = float(value)
                    except ValueError:
                        pass
                else:
                    config[config_key] = value
        
        return config
    
    def save_config(self, config_path: Optional[Path] = None) -> None:
        """Save current configuration to file"""
        if config_path is None:
            config_path = Path.cwd() / "ctl-sync.yaml"
        
        config_dict = self.config.dict()
        
        try:
            with open(config_path, 'w', encoding='utf-8') as f:
                yaml.dump(config_dict, f, default_flow_style=False, indent=2)
            print(f"Configuration saved to {config_path}")
        except IOError as e:
            print(f"Failed to save configuration: {e}")
    
    def get_effective_paths(self, base_path: Path) -> Dict[str, Path]:
        """Get effective paths resolved against base path"""
        return {
            'crates': base_path / self.config.crates_path,
            'claude': base_path / self.config.claude_path,
            'cache': base_path / self.config.cache_path,
        }


def create_default_config() -> str:
    """Create default configuration YAML content"""
    return """# CTL Sync Daemon Configuration

# Paths (relative to project root)
crates_path: "crates"
claude_path: "CLAUDE.md"
cache_path: "cache.json"

# File filtering
include_patterns:
  - "**/*.rs"
exclude_patterns:
  - "**/target/**"
  - "**/.git/**"
  - "**/.*"

# Parser settings
ctl2_parser:
  enabled: true
  patterns:
    component: '//\\s*@component:\\s*(\\{.*\\})'
  flags: []

ctl3_parser:
  enabled: true
  patterns:
    tensor: '//\\s*@ctl3:\\s*(Ⱦ\\[.*?\\]\\s*:=\\s*\\{[^}]*\\})'
  flags: []

# File watching
watch_debounce_seconds: 0.5
watch_recursive: true

# Validation
strict_validation: true
schema_validation: true

# Output
pretty_json: false
colored_output: true
verbose_logging: false

# Performance
max_file_size_mb: 10
parallel_processing: false
cache_enabled: true
"""