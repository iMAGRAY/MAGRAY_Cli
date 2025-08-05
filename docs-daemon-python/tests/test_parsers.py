"""
Tests for CTL Parsers

Unit tests for CTL v2.0 and v3.0 parsers.
"""

import unittest
from ctl_sync.parsers import Ctl2Parser, Ctl3Parser


class TestCtl2Parser(unittest.TestCase):
    """Test CTL v2.0 JSON parser"""
    
    def setUp(self):
        self.parser = Ctl2Parser()
    
    def test_parse_valid_component(self):
        """Test parsing valid CTL v2.0 component"""
        line = '// @component: {"k":"C","id":"test_component","t":"Test component","m":{"cur":85,"tgt":95,"u":"%"}}'
        result = self.parser.parse_line(line, 1, "test.rs")
        
        self.assertIsNotNone(result)
        self.assertEqual(result['k'], 'C')
        self.assertEqual(result['id'], 'test_component')
        self.assertEqual(result['t'], 'Test component')
        self.assertEqual(result['x_file'], 'test.rs:1')
    
    def test_parse_invalid_json(self):
        """Test parsing invalid JSON"""
        line = '// @component: {"k":"C","id":"test_component"'  # Invalid JSON
        result = self.parser.parse_line(line, 1, "test.rs")
        
        self.assertIsNone(result)
    
    def test_no_annotation(self):
        """Test line without annotation"""
        line = 'fn main() { println!("Hello World"); }'
        result = self.parser.parse_line(line, 1, "test.rs")
        
        self.assertIsNone(result)


class TestCtl3Parser(unittest.TestCase):
    """Test CTL v3.0 tensor parser"""
    
    def setUp(self):
        self.parser = Ctl3Parser()
    
    def test_parse_valid_tensor(self):
        """Test parsing valid CTL v3.0 tensor"""
        line = '// @ctl3: D[memory_service:Service] := {grad[85->95] compose[vector_store,cache] implies performance}'
        result = self.parser.parse_line(line, 1, "test.rs")
        
        self.assertIsNotNone(result)
        self.assertEqual(result['k'], 'S')  # Service
        self.assertEqual(result['id'], 'memory_service')
        self.assertEqual(result['t'], 'Service (CTL v3.0)')
        self.assertEqual(result['x_file'], 'test.rs:1')
        
        # Check extracted metadata
        self.assertIn('m', result)
        self.assertEqual(result['m']['cur'], 85)
        self.assertEqual(result['m']['tgt'], 95)
        
        self.assertIn('d', result)
        self.assertIn('vector_store', result['d'])
        self.assertIn('cache', result['d'])
        
        self.assertIn('f', result)
        self.assertIn('tensor_composition', result['f'])
        self.assertIn('optimization', result['f'])
    
    def test_parse_minimal_tensor(self):
        """Test parsing minimal tensor without operations"""
        line = '// @ctl3: D[simple_component:Component] := {}'
        result = self.parser.parse_line(line, 1, "test.rs")
        
        self.assertIsNotNone(result)
        self.assertEqual(result['k'], 'C')
        self.assertEqual(result['id'], 'simple_component')
    
    def test_parse_invalid_tensor(self):
        """Test parsing invalid tensor format"""
        line = '// @ctl3: invalid tensor format'
        result = self.parser.parse_line(line, 1, "test.rs")
        
        self.assertIsNone(result)
    
    def test_validate_tensor(self):
        """Test tensor validation"""
        valid_tensor = 'D[test:Component] := {grad[70->90]}'
        is_valid, errors = self.parser.validate_tensor(valid_tensor)
        
        self.assertTrue(is_valid)
        self.assertEqual(len(errors), 0)
        
        invalid_tensor = 'invalid format'
        is_valid, errors = self.parser.validate_tensor(invalid_tensor)
        
        self.assertFalse(is_valid)
        self.assertGreater(len(errors), 0)


if __name__ == '__main__':
    unittest.main()