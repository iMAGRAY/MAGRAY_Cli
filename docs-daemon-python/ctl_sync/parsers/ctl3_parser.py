"""
CTL v3.0 Tensor Parser

Parses CTL v3.0 tensor format annotations with advanced tensor operation support.
"""

import re
from typing import Any, Dict, List, Optional

from .base_parser import BaseParser
from ..schema import CtlSchema


class Ctl3Parser(BaseParser):
    """CTL v3.0 Tensor format parser"""
    
    def __init__(self):
        super().__init__()
        self.schema = CtlSchema()
    
    def setup_patterns(self) -> None:
        """Setup regex patterns for CTL v3.0 tensor parsing"""
        # Pattern: // @ctl3: Ⱦ[id:type] := {tensor_operations} or D[id:type] := {operations}
        self.ctl3_pattern = re.compile(r'//\s*@ctl3:\s*([ⱧD]\[.*?\]\s*:=\s*\{[^}]*\})', re.IGNORECASE)
        
        # Tensor component pattern: Ⱦ[id:type] := {operations} or D[id:type] := {operations}
        self.tensor_pattern = re.compile(r'[ⱧD]\[([^:]+):([^\]]+)\]\s*:=\s*\{([^}]*)\}')
        
        # Tensor operation patterns (both Unicode and ASCII variants)
        self.maturity_pattern = re.compile(r'(?:∇|grad)\[(\d+)(?:→|->)(\d+)\]')
        self.dependency_pattern = re.compile(r'(?:⊗|compose)\[([^\]]+)\]')
        
        # Tensor operators for flag extraction (both Unicode and ASCII)
        self.tensor_operators = {
            '⊗': 'tensor_composition',
            'compose': 'tensor_composition',
            '⊕': 'tensor_addition', 
            'parallel': 'tensor_addition',
            '⊙': 'elementwise_product',
            '⊡': 'convolution',
            '∇': 'optimization',
            'grad': 'optimization',
            '∂': 'partial_implementation',
            'partial': 'partial_implementation',
            '∴': 'conclusion',
            '∵': 'causality',
            '≡': 'equivalence',
            '⟹': 'implication',
            'implies': 'implication',
            '⟷': 'bidirectional'
        }
    
    def parse_line(self, line: str, line_no: int, file_path: str) -> Optional[Dict[str, Any]]:
        """
        Parse CTL v3.0 tensor annotation from line
        
        Args:
            line: Source code line
            line_no: Line number (1-based)
            file_path: Relative file path
            
        Returns:
            Parsed component dictionary or None
        """
        match = self.ctl3_pattern.search(line)
        if not match:
            return None
        
        tensor_str = match.group(1)
        print(f"      Found CTL v3.0 tensor: {tensor_str}")
        
        component = self.parse_tensor(tensor_str, file_path, line_no)
        if component:
            self.add_file_location(component, file_path, line_no)
        
        return component
    
    def parse_tensor(self, tensor_str: str, file_path: str, line_no: int) -> Optional[Dict[str, Any]]:
        """
        Parse CTL v3.0 tensor string into component dictionary
        
        Args:
            tensor_str: Raw tensor string
            file_path: File path for error reporting
            line_no: Line number for error reporting
            
        Returns:
            Component dictionary or None if parsing fails
        """
        match = self.tensor_pattern.search(tensor_str)
        if not match:
            print(f"        Failed to parse CTL v3.0 tensor at {file_path}:{line_no}: {tensor_str}")
            return None
        
        id_str = match.group(1).strip()
        type_str = match.group(2).strip()
        operations = match.group(3).strip()
        
        # Build base component (CTL v2.0 compatible)
        component = {
            'k': self.schema.infer_kind_from_type(type_str),
            'id': id_str,
            't': f"{type_str} (CTL v3.0)",
            'ctl3_tensor': tensor_str
        }
        
        # Extract tensor operations
        self._extract_maturity(component, operations)
        self._extract_dependencies(component, operations)
        self._extract_flags(component, operations)
        
        return component
    
    def _extract_maturity(self, component: Dict[str, Any], operations: str) -> None:
        """Extract maturity tensor ∇[cur→tgt] from operations"""
        match = self.maturity_pattern.search(operations)
        if match:
            cur = int(match.group(1))
            tgt = int(match.group(2))
            component['m'] = {
                'cur': cur,
                'tgt': tgt,
                'u': '%'
            }
    
    def _extract_dependencies(self, component: Dict[str, Any], operations: str) -> None:
        """Extract dependencies ⊗[dep1,dep2] from operations"""
        match = self.dependency_pattern.search(operations)
        if match:
            deps_str = match.group(1)
            dependencies = [dep.strip() for dep in deps_str.split(',')]
            component['d'] = dependencies
    
    def _extract_flags(self, component: Dict[str, Any], operations: str) -> None:
        """Extract feature flags from tensor operators"""
        flags = []
        
        # Check for tensor operators
        for operator, flag in self.tensor_operators.items():
            if operator in operations:
                flags.append(flag)
        
        # Check for specific keywords
        operation_lower = operations.lower()
        if 'gpu' in operation_lower:
            flags.append('gpu')
        if any(keyword in operation_lower for keyword in ['ai', 'ml', 'neural']):
            flags.append('ai')
        if 'async' in operation_lower:
            flags.append('async')
        if 'real_time' in operation_lower:
            flags.append('real_time')
        if 'batch' in operation_lower:
            flags.append('batch')
        if 'stream' in operation_lower:
            flags.append('streaming')
        
        if flags:
            component['f'] = flags
    
    def validate_tensor(self, tensor_str: str) -> tuple[bool, List[str]]:
        """
        Validate CTL v3.0 tensor syntax
        
        Args:
            tensor_str: Raw tensor string
            
        Returns:
            (is_valid, error_messages)
        """
        return self.schema.validate_ctl3_tensor(tensor_str)