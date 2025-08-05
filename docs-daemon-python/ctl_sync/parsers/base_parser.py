"""
Base Parser for CTL Formats

Abstract base class for CTL parsers with common functionality.
"""

import re
from abc import ABC, abstractmethod
from typing import Any, Dict, List, Optional, Tuple


class BaseParser(ABC):
    """Abstract base parser for CTL formats"""
    
    def __init__(self):
        self.setup_patterns()
    
    @abstractmethod
    def setup_patterns(self) -> None:
        """Setup regex patterns for parsing"""
        pass
    
    @abstractmethod
    def parse_line(self, line: str, line_no: int, file_path: str) -> Optional[Dict[str, Any]]:
        """
        Parse a single line for CTL annotations
        
        Args:
            line: Source code line
            line_no: Line number (1-based)
            file_path: Relative file path
            
        Returns:
            Parsed component dictionary or None
        """
        pass
    
    def extract_from_file(self, content: str, file_path: str) -> List[Dict[str, Any]]:
        """
        Extract all CTL components from file content
        
        Args:
            content: File content
            file_path: Relative file path
            
        Returns:
            List of parsed components
        """
        components = []
        
        for line_no, line in enumerate(content.splitlines(), 1):
            component = self.parse_line(line, line_no, file_path)
            if component:
                components.append(component)
        
        return components
    
    def normalize_file_path(self, file_path: str) -> str:
        """Normalize file path for cross-platform compatibility"""
        return file_path.replace('\\', '/')
    
    def add_file_location(self, component: Dict[str, Any], file_path: str, line_no: int) -> None:
        """Add file location metadata to component"""
        component['x_file'] = f"{self.normalize_file_path(file_path)}:{line_no}"