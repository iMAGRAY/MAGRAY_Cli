"""
Tests for CTL Schema Validation

Unit tests for schema validation functionality.
"""

import unittest
from ctl_sync.schema import CtlSchema


class TestCtlSchema(unittest.TestCase):
    """Test CTL schema validation"""
    
    def setUp(self):
        self.schema = CtlSchema()
    
    def test_validate_valid_ctl2_component(self):
        """Test validation of valid CTL v2.0 component"""
        component = {
            'k': 'C',
            'id': 'test_component',
            't': 'Test component',
            'm': {'cur': 85, 'tgt': 95, 'u': '%'},
            'f': ['test', 'example']
        }
        
        is_valid, errors = self.schema.validate_ctl2(component)
        
        self.assertTrue(is_valid)
        self.assertEqual(len(errors), 0)
    
    def test_validate_invalid_ctl2_component(self):
        """Test validation of invalid CTL v2.0 component"""
        component = {
            'k': 'INVALID',  # Invalid kind
            'id': 'test-component-with-dashes',  # Invalid ID format
            't': 'A' * 50,  # Too long description
        }
        
        is_valid, errors = self.schema.validate_ctl2(component)
        
        self.assertFalse(is_valid)
        self.assertGreater(len(errors), 0)
    
    def test_validate_missing_required_fields(self):
        """Test validation with missing required fields"""
        component = {
            'k': 'C',
            # Missing 'id' and 't'
        }
        
        is_valid, errors = self.schema.validate_ctl2(component)
        
        self.assertFalse(is_valid)
        self.assertGreater(len(errors), 0)
    
    def test_infer_kind_from_type(self):
        """Test component kind inference"""
        test_cases = [
            ('TestService', 'T'),
            ('AgentManager', 'A'),
            ('BatchProcessor', 'B'),
            ('HelperFunction', 'F'),
            ('DataModule', 'M'),
            ('UserService', 'S'),
            ('SystemResource', 'R'),
            ('BackgroundProcess', 'P'),
            ('UserData', 'D'),
            ('ValidationError', 'E'),
            ('GenericComponent', 'C'),
        ]
        
        for type_str, expected_kind in test_cases:
            with self.subTest(type_str=type_str):
                actual_kind = self.schema.infer_kind_from_type(type_str)
                self.assertEqual(actual_kind, expected_kind)
    
    def test_validate_ctl3_tensor(self):
        """Test CTL v3.0 tensor validation"""
        valid_tensors = [
            'Ⱦ[component:Service] := {∇[85→95]}',
            'Ⱦ[test:Component] := {⊗[dep1,dep2] ⊕ parallel}',
            'Ⱦ[complex:Agent] := {∇[70→90] ⊗[llm,tools] ⟹smart_routing}',
        ]
        
        for tensor in valid_tensors:
            with self.subTest(tensor=tensor):
                is_valid, errors = self.schema.validate_ctl3_tensor(tensor)
                self.assertTrue(is_valid, f"Tensor should be valid: {tensor}, errors: {errors}")
        
        invalid_tensors = [
            'invalid format',
            'missing tensor symbol',
            'Ⱦ[id:type] without assignment',
            'assignment without operations :=',
        ]
        
        for tensor in invalid_tensors:
            with self.subTest(tensor=tensor):
                is_valid, errors = self.schema.validate_ctl3_tensor(tensor)
                self.assertFalse(is_valid, f"Tensor should be invalid: {tensor}")
                self.assertGreater(len(errors), 0)
    
    def test_get_component_kinds(self):
        """Test component kind descriptions"""
        kinds = self.schema.get_component_kinds()
        
        expected_kinds = {
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
        
        self.assertEqual(kinds, expected_kinds)


if __name__ == '__main__':
    unittest.main()