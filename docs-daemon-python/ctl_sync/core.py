"""
CTL Sync Core

Main synchronization logic with file monitoring and CLAUDE.md updates.
"""

# @component: {"k":"C","id":"ctl_sync_core","t":"Python CTL sync engine","m":{"cur":90,"tgt":100,"u":"%"},"f":["python","ctl","sync","parsing","claude_md"]}

import hashlib
import json
import os
from datetime import datetime, timezone
from pathlib import Path
from typing import Any, Dict, List, Optional, Tuple

from .parsers import Ctl2Parser, Ctl3Parser, Ctl3EnhancedParser
from .schema import CtlSchema
from .json_config import CtlJsonConfig


class FileCache:
    """File hash cache for change detection"""
    
    def __init__(self, cache_path: Path):
        self.cache_path = cache_path
        self.hashes: Dict[str, str] = {}
        self.load()
    
    def load(self) -> None:
        """Load cache from disk"""
        if self.cache_path.exists():
            try:
                with open(self.cache_path, 'r', encoding='utf-8') as f:
                    data = json.load(f)
                    self.hashes = data.get('hashes', {})
            except (json.JSONDecodeError, IOError) as e:
                print(f"Warning: Failed to load cache: {e}")
                self.hashes = {}
    
    def save(self) -> None:
        """Save cache to disk"""
        try:
            cache_data = {'hashes': self.hashes}
            with open(self.cache_path, 'w', encoding='utf-8') as f:
                json.dump(cache_data, f, indent=2, ensure_ascii=False)
        except IOError as e:
            print(f"Warning: Failed to save cache: {e}")
    
    def get_hash(self, file_path: Path) -> str:
        """Calculate SHA256 hash of file"""
        try:
            with open(file_path, 'rb') as f:
                content = f.read()
            return hashlib.sha256(content).hexdigest()
        except IOError:
            return ""
    
    def is_changed(self, file_path: Path, relative_path: str) -> bool:
        """Check if file has changed since last scan"""
        current_hash = self.get_hash(file_path)
        previous_hash = self.hashes.get(relative_path, "")
        
        if current_hash != previous_hash:
            self.hashes[relative_path] = current_hash
            return True
        
        return False


class CtlSync:
    """Main CTL synchronization class"""
    
    def __init__(self, base_path: Optional[Path] = None):
        self.base_path = base_path or Path.cwd()
        
        # Initialize JSON configuration
        settings_path = self.base_path / "settings.json"
        self.config = CtlJsonConfig(settings_path)
        
        self.setup_paths()
        
        # Initialize components
        self.cache = FileCache(self.cache_path)
        self.schema = CtlSchema()
        self.ctl2_parser = Ctl2Parser()
        
        # Use enhanced CTL v3.0 parser if enabled in config
        use_enhanced = self.config.get('parsing.ctl3.use_enhanced', False)
        if use_enhanced:
            self.ctl3_parser = Ctl3EnhancedParser()
            print("  - Using Enhanced CTL v3.0 parser with extended semantics")
        else:
            self.ctl3_parser = Ctl3Parser()
        
        print("CTL v3.0 Tensor Sync Daemon (Python)")
        print("=====================================")
        print("Supports:")
        print("  - CTL v2.0 JSON format: // @component: {...}")
        print("  - CTL v3.0 Tensor format: // @ctl3: D[id:type] := {...}")
        print("  - Tensor operators: compose,grad,parallel with auto-parsing")
        print()
    
    def setup_paths(self) -> None:
        """Setup file paths based on configuration and working directory"""
        # Check if running from docs-daemon-python subdirectory
        if self.base_path.name == "docs-daemon-python":
            project_root = self.base_path.parent
        else:
            project_root = self.base_path
        
        # Use configuration for paths
        self.crates_path = project_root / self.config.get_crates_dir()
        self.claude_path = project_root / self.config.get_claude_md_file()
        self.cache_path = self.base_path / "cache.json"
        
        print(f"Scanning directory: {self.crates_path}")
        print(f"Updating file: {self.claude_path}")
    
    def scan_crates(self) -> Tuple[bool, List[Dict[str, Any]]]:
        """
        Scan crates directory for CTL annotations
        
        Returns:
            (has_changes, all_components)
        """
        all_components = []
        has_changes = False
        file_count = 0
        
        if not self.crates_path.exists():
            print(f"Warning: Crates directory not found: {self.crates_path}")
            return False, []
        
        # Walk through all Rust files
        for rust_file in self.crates_path.rglob("*.rs"):
            # Skip target directory
            if "target" in rust_file.parts:
                continue
            
            file_count += 1
            relative_path = rust_file.relative_to(self.crates_path).as_posix()
            
            # Check for changes
            if self.cache.is_changed(rust_file, relative_path):
                print(f"  Changed: {relative_path}")
                has_changes = True
            
            # Extract components from file
            try:
                with open(rust_file, 'r', encoding='utf-8') as f:
                    content = f.read()
                
                file_components = self.extract_components(content, relative_path)
                all_components.extend(file_components)
                
            except (IOError, UnicodeDecodeError) as e:
                print(f"  Warning: Failed to read {relative_path}: {e}")
        
        # Sort components by kind and id
        all_components.sort(key=lambda c: (c.get('k', ''), c.get('id', '')))
        
        status = "some" if has_changes else "none"
        print(f"Scanned {file_count} files, {status} changed")
        
        return has_changes, all_components
    
    def extract_components(self, content: str, file_path: str) -> List[Dict[str, Any]]:
        """
        Extract CTL components from file content using all parsers
        
        Args:
            content: File content
            file_path: Relative file path
            
        Returns:
            List of validated components
        """
        components = []
        
        # Parse CTL v2.0 annotations
        ctl2_components = self.ctl2_parser.extract_from_file(content, file_path)
        components.extend(ctl2_components)
        
        # Parse CTL v3.0 annotations
        ctl3_components = self.ctl3_parser.extract_from_file(content, file_path)
        components.extend(ctl3_components)
        
        # Validate all components
        validated_components = []
        for component in components:
            if self.validate_component(component, file_path):
                validated_components.append(component)
        
        return validated_components
    
    def validate_component(self, component: Dict[str, Any], file_path: str) -> bool:
        """
        Validate component against CTL schema
        
        Args:
            component: Component dictionary
            file_path: File path for error reporting
            
        Returns:
            True if valid, False otherwise
        """
        is_valid, errors = self.schema.validate_ctl2(component)
        
        if not is_valid:
            print(f"        Schema validation failed for {file_path}:")
            for error in errors:
                print(f"          - {error}")
            return False
        
        return True
    
    def update_claude_md(self, components: List[Dict[str, Any]]) -> None:
        """
        Update CLAUDE.md with new component data
        
        Args:
            components: List of components to write
        """
        print(f"Updating CLAUDE.md with {len(components)} components")
        
        if not self.claude_path.exists():
            print(f"Warning: CLAUDE.md not found at {self.claude_path}")
            return
        
        try:
            with open(self.claude_path, 'r', encoding='utf-8') as f:
                content = f.read()
        except IOError as e:
            print(f"Error reading CLAUDE.md: {e}")
            return
        
        # Generate new architecture section
        timestamp = datetime.now(timezone.utc).strftime("%Y-%m-%d %H:%M:%S UTC")
        new_section = self.build_architecture_section(components, timestamp)
        
        # Replace existing section
        updated_content = self.replace_architecture_section(content, new_section)
        
        # Write back to file
        try:
            with open(self.claude_path, 'w', encoding='utf-8') as f:
                f.write(updated_content)
        except IOError as e:
            print(f"Error writing CLAUDE.md: {e}")
    
    def build_architecture_section(self, components: List[Dict[str, Any]], timestamp: str) -> str:
        """Build the AUTO-GENERATED ARCHITECTURE section"""
        section_lines = [
            "# AUTO-GENERATED ARCHITECTURE",
            "",
            f"*Last updated: {timestamp}*",
            "",
            "## Components (CTL v2.0/v3.0 Mixed Format)",
            "",
            "```json"
        ]
        
        # Add each component as a JSON line
        for component in components:
            json_line = json.dumps(component, ensure_ascii=False, separators=(',', ':'))
            section_lines.append(json_line)
        
        section_lines.append("```")
        
        return "\n".join(section_lines)
    
    def replace_architecture_section(self, content: str, new_section: str) -> str:
        """Replace the AUTO-GENERATED ARCHITECTURE section in CLAUDE.md"""
        start_marker = "# AUTO-GENERATED ARCHITECTURE"
        end_marker = "# AUTO-GENERATED COMPONENT STATUS"
        
        start_pos = content.find(start_marker)
        if start_pos == -1:
            # Section doesn't exist, append it
            return content + "\n\n" + new_section
        
        # Find end position
        end_pos = content.find(end_marker, start_pos)
        if end_pos == -1:
            end_pos = len(content)
        
        # Replace the section
        before = content[:start_pos]
        after = content[end_pos:]
        
        return before + new_section + "\n\n" + after
    
    def sync_once(self) -> None:
        """Perform one-time synchronization"""
        print("Scanning for CTL changes...")
        
        has_changes, components = self.scan_crates()
        
        if has_changes:
            self.update_claude_md(components)
            self.cache.save()
            print("Update complete")
        else:
            print("No changes detected")
    
    def get_stats(self) -> Dict[str, Any]:
        """Get current synchronization statistics"""
        _, components = self.scan_crates()
        
        # Count by kind
        kind_counts = {}
        for component in components:
            kind = component.get('k', 'Unknown')
            kind_counts[kind] = kind_counts.get(kind, 0) + 1
        
        # Count by format
        ctl2_count = sum(1 for c in components if 'ctl3_tensor' not in c)
        ctl3_count = sum(1 for c in components if 'ctl3_tensor' in c)
        
        return {
            'total_components': len(components),
            'ctl2_components': ctl2_count,
            'ctl3_components': ctl3_count,
            'kind_distribution': kind_counts,
            'last_scan': datetime.now(timezone.utc).isoformat()
        }