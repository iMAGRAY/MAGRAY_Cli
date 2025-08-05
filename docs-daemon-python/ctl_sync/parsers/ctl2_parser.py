"""
CTL v2.0 JSON Parser

Parses CTL v2.0 JSON format annotations.
"""

import json
import re
from typing import Any, Dict, Optional

from .base_parser import BaseParser


class Ctl2Parser(BaseParser):
    """CTL v2.0 JSON format parser"""
    
    def setup_patterns(self) -> None:
        """Setup regex patterns for CTL v2.0 JSON parsing"""
        # Pattern: // @component: {"k":"C","id":"name",...}
        self.component_pattern = re.compile(r'//\s*@component:\s*(\{.*\})', re.IGNORECASE)
    
    def parse_line(self, line: str, line_no: int, file_path: str) -> Optional[Dict[str, Any]]:
        """
        Parse CTL v2.0 JSON annotation from line
        
        Args:
            line: Source code line
            line_no: Line number (1-based)
            file_path: Relative file path
            
        Returns:
            Parsed component dictionary or None
        """
        match = self.component_pattern.search(line)
        if not match:
            return None
        
        json_str = match.group(1)
        
        try:
            component = json.loads(json_str)
            
            # Ensure it's a dictionary
            if not isinstance(component, dict):
                print(f"        Warning: CTL v2.0 annotation is not an object at {file_path}:{line_no}")
                return None
            
            # Add file location
            self.add_file_location(component, file_path, line_no)
            
            print(f"      Found CTL v2.0 annotation: {json_str}")
            return component
            
        except json.JSONDecodeError as e:
            print(f"        JSON parse error at {file_path}:{line_no}: {e}")
            return None