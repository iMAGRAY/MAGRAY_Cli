"""
CTL Schema Validation

Provides schema validation for CTL v2.0 and v3.0 formats with detailed error reporting.
"""

import json
from typing import Any, Dict, List, Optional
from jsonschema import Draft7Validator, ValidationError
from pydantic import BaseModel, Field, validator
try:
    from pydantic import ValidationError as PydanticValidationError
except ImportError:
    from pydantic_core import ValidationError as PydanticValidationError


class CtlMaturity(BaseModel):
    """CTL maturity tensor ⟨cur, tgt, unit⟩"""
    cur: float = Field(..., ge=0, le=100, description="Current maturity percentage")
    tgt: float = Field(..., ge=0, le=100, description="Target maturity percentage") 
    u: str = Field(default="%", max_length=10, description="Unit of measurement")


class CtlComponent(BaseModel):
    """CTL v2.0 Component structure"""
    k: str = Field(..., pattern=r'^[TABFMSRPDC]$', description="Component kind")
    id: str = Field(..., pattern=r'^[a-z0-9_]{1,32}$', description="Component ID")
    t: str = Field(..., max_length=40, description="Component description")
    p: Optional[int] = Field(None, ge=1, le=5, description="Priority level")
    e: Optional[str] = Field(None, pattern=r'^P(\d+D)?(T(\d+H)?(\d+M)?)?$|^PT\d+[HMS]$', description="Effort estimate")
    d: Optional[List[str]] = Field(None, max_items=10, description="Dependencies")
    r: Optional[str] = Field(None, max_length=20, description="Responsible person")
    m: Optional[CtlMaturity] = Field(None, description="Maturity metrics")
    f: Optional[List[str]] = Field(None, max_items=10, description="Feature flags")
    x_file: Optional[str] = Field(None, description="File location")
    ctl3_tensor: Optional[str] = Field(None, description="CTL v3.0 tensor representation")


class CtlSchema:
    """CTL Schema validation and management"""
    
    def __init__(self):
        self.ctl2_schema = self._build_ctl2_schema()
        self.ctl2_validator = Draft7Validator(self.ctl2_schema)
        
    def _build_ctl2_schema(self) -> Dict[str, Any]:
        """Build CTL v2.0 JSON Schema"""
        return {
            "$schema": "http://json-schema.org/draft-07/schema#",
            "type": "object",
            "required": ["k", "id", "t"],
            "properties": {
                "k": {
                    "type": "string",
                    "enum": ["T", "A", "B", "F", "M", "S", "R", "P", "D", "C", "E"]
                },
                "id": {
                    "type": "string",
                    "pattern": "^[a-z0-9_]{1,32}$"
                },
                "t": {
                    "type": "string",
                    "maxLength": 40
                },
                "p": {
                    "type": "integer",
                    "minimum": 1,
                    "maximum": 5
                },
                "e": {
                    "type": "string",
                    "pattern": "^P(\\d+D)?(T(\\d+H)?(\\d+M)?)?$|^PT\\d+[HMS]$"
                },
                "d": {
                    "type": "array",
                    "items": {"type": "string"},
                    "maxItems": 10
                },
                "r": {
                    "type": "string",
                    "maxLength": 20
                },
                "m": {
                    "type": "object",
                    "required": ["cur", "tgt", "u"],
                    "properties": {
                        "cur": {"type": "number"},
                        "tgt": {"type": "number"},
                        "u": {"type": "string", "maxLength": 10}
                    }
                },
                "f": {
                    "type": "array",
                    "items": {"type": "string"},
                    "maxItems": 10
                }
            },
            "additionalProperties": {
                "oneOf": [
                    {"type": "string"},
                    {"type": "number"},
                    {"type": "boolean"},
                    {"type": "array"},
                    {"type": "object"}
                ]
            }
        }
    
    def validate_ctl2(self, component: Dict[str, Any]) -> tuple[bool, List[str]]:
        """
        Validate CTL v2.0 component
        
        Returns:
            (is_valid, error_messages)
        """
        errors = []
        
        try:
            # Use pydantic for more detailed validation
            CtlComponent(**component)
            
            # Additional JSON Schema validation
            validation_errors = list(self.ctl2_validator.iter_errors(component))
            if validation_errors:
                for error in validation_errors:
                    errors.append(f"Schema validation: {error.message}")
            
            return len(errors) == 0, errors
            
        except PydanticValidationError as e:
            for error in e.errors():
                field = ".".join(str(x) for x in error["loc"])
                msg = error["msg"]
                errors.append(f"Field '{field}': {msg}")
            
            return False, errors
    
    def validate_ctl3_tensor(self, tensor_str: str) -> tuple[bool, List[str]]:
        """
        Validate CTL v3.0 tensor format
        
        Args:
            tensor_str: Raw tensor string like "Ⱦ[id:type] := {operations}"
            
        Returns:
            (is_valid, error_messages)
        """
        errors = []
        
        # Basic tensor format validation
        if not (tensor_str.strip().startswith('Ⱦ[') or tensor_str.strip().startswith('D[')):
            errors.append("CTL v3.0: Must start with tensor symbol 'Ⱦ[' or 'D['")
        
        if ':=' not in tensor_str:
            errors.append("CTL v3.0: Missing tensor assignment operator ':='")
        
        if not ('{' in tensor_str and '}' in tensor_str):
            errors.append("CTL v3.0: Missing tensor operation braces '{...}'")
        
        # Validate tensor operators (both Unicode and ASCII)
        valid_operators = ['⊗', '⊕', '⊙', '⊡', '∇', '∂', '∴', '∵', '≡', '⟹', '⟷',
                          'compose', 'parallel', 'grad', 'partial', 'implies']
        operations_part = tensor_str.split(':=')[1] if ':=' in tensor_str else ""
        
        found_operators = [op for op in valid_operators if op in operations_part]
        if not found_operators and operations_part.strip() != '{}':
            errors.append("CTL v3.0: No valid tensor operators found")
        
        return len(errors) == 0, errors
    
    def get_component_kinds(self) -> Dict[str, str]:
        """Get mapping of component kind codes to descriptions"""
        return {
            'T': 'Test',
            'A': 'Agent', 
            'B': 'Batch',
            'F': 'Function',
            'M': 'Module',
            'S': 'Service',
            'R': 'Resource',
            'P': 'Process',
            'D': 'Data',
            'C': 'Component',
            'E': 'Error'
        }
    
    def infer_kind_from_type(self, type_str: str) -> str:
        """Infer component kind from type string (CTL v3.0 conversion)"""
        type_lower = type_str.lower()
        
        if 'test' in type_lower:
            return 'T'
        elif 'agent' in type_lower:
            return 'A'
        elif 'batch' in type_lower:
            return 'B'
        elif 'function' in type_lower:
            return 'F'
        elif 'module' in type_lower:
            return 'M'
        elif 'service' in type_lower:
            return 'S'
        elif 'resource' in type_lower:
            return 'R'
        elif 'process' in type_lower:
            return 'P'
        elif 'data' in type_lower:
            return 'D'
        elif 'error' in type_lower:
            return 'E'
        else:
            return 'C'  # Component by default