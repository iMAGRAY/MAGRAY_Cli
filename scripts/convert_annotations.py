#!/usr/bin/env python3
"""
CTL Annotation Converter
Converts old-style component annotations to CTL v2.0 JSON format
"""

import os
import re
import json
import glob
from typing import Dict, List, Optional

class AnnotationConverter:
    def __init__(self):
        self.status_to_percent = {
            "WORKING": 80,
            "BROKEN": 20,
            "MOCKED": 40,
            "PARTIAL": 60,
            "COMPLETE": 95
        }
        
        self.priority_map = {
            "high": 4,
            "medium": 3,
            "low": 2,
            "critical": 5
        }

    def extract_old_annotations(self, content: str) -> List[Dict]:
        """Extract old-style annotations from file content"""
        annotations = []
        
        # Pattern for old-style annotations
        patterns = [
            # // @component: ComponentName
            r'//\s*@component:\s*([A-Za-z_][A-Za-z0-9_]*)',
            # // @status: WORKING
            r'//\s*@status:\s*([A-Za-z_]+)',
            # // @performance: O(n) description
            r'//\s*@performance:\s*(.+)',
            # // @production_ready: 85%
            r'//\s*@production_ready:\s*(\d+)%',
            # // @dependencies: dep1(✅), dep2(⚠️)
            r'//\s*@dependencies:\s*(.+)',
            # // @issues: description
            r'//\s*@issues:\s*(.+)',
        ]
        
        component_blocks = []
        current_block = {}
        
        for i, line in enumerate(content.split('\n')):
            line = line.strip()
            
            # Check for component start
            comp_match = re.search(r'//\s*@component:\s*([A-Za-z_][A-Za-z0-9_]*)', line)
            if comp_match:
                if current_block:
                    component_blocks.append(current_block)
                current_block = {
                    'name': comp_match.group(1),
                    'line': i + 1,
                    'attributes': {}
                }
                continue
            
            # Collect attributes for current component
            if current_block:
                for pattern_name, pattern in [
                    ('status', r'//\s*@status:\s*([A-Za-z_]+)'),
                    ('performance', r'//\s*@performance:\s*(.+)'),
                    ('production_ready', r'//\s*@production_ready:\s*(\d+)%'),
                    ('dependencies', r'//\s*@dependencies:\s*(.+)'),
                    ('issues', r'//\s*@issues:\s*(.+)'),
                    ('tests', r'//\s*@tests:\s*(.+)'),
                    ('upgrade_path', r'//\s*@upgrade_path:\s*(.+)'),
                ]:
                    match = re.search(pattern, line)
                    if match:
                        current_block['attributes'][pattern_name] = match.group(1)
                        continue
                
                # If we hit a non-annotation line, finish current block
                if not line.startswith('//') or '@' not in line:
                    if current_block:
                        component_blocks.append(current_block)
                        current_block = {}
        
        # Don't forget the last block
        if current_block:
            component_blocks.append(current_block)
            
        return component_blocks

    def convert_to_ctl(self, old_annotation: Dict, file_path: str) -> Optional[str]:
        """Convert old annotation to CTL v2.0 JSON"""
        name = old_annotation['name']
        attrs = old_annotation['attributes']
        
        # Generate ID from name
        component_id = re.sub(r'([A-Z])', r'_\1', name).lower().strip('_')
        
        # Build CTL object
        ctl = {
            "k": "C",
            "id": component_id,
            "t": name.replace('_', ' ').title()
        }
        
        # Add metrics if we have status or production_ready
        if 'status' in attrs or 'production_ready' in attrs:
            current = self.status_to_percent.get(attrs.get('status', ''), 50)
            if 'production_ready' in attrs:
                current = int(attrs['production_ready'])
            
            ctl["m"] = {
                "cur": current,
                "tgt": min(100, current + 20),
                "u": "%"
            }
        
        # Add dependencies
        if 'dependencies' in attrs:
            deps_text = attrs['dependencies']
            # Extract dependency names (very basic parsing)
            deps = re.findall(r'([A-Za-z_][A-Za-z0-9_]*)', deps_text)
            if deps:
                ctl["d"] = [dep.lower() for dep in deps[:3]]  # Limit to 3
        
        # Add flags based on attributes
        flags = []
        if 'performance' in attrs and 'O(n)' in attrs['performance']:
            flags.append("O(n)")
        if 'status' in attrs:
            status = attrs['status'].lower()
            if status in ['working', 'complete']:
                flags.append("stable")
            elif status in ['broken', 'mocked']:
                flags.append("unstable")
        if 'tests' in attrs and '❌' in attrs['tests']:
            flags.append("no_tests")
        elif 'tests' in attrs and '✅' in attrs['tests']:
            flags.append("tested")
            
        if flags:
            ctl["f"] = flags
        
        return json.dumps(ctl, separators=(',', ':'))

    def process_file(self, file_path: str) -> bool:
        """Process a single Rust file"""
        try:
            with open(file_path, 'r', encoding='utf-8') as f:
                content = f.read()
            
            old_annotations = self.extract_old_annotations(content)
            if not old_annotations:
                return False
            
            print(f"Converting {len(old_annotations)} annotations in {file_path}")
            
            # Convert each annotation
            lines = content.split('\n')
            changes_made = False
            
            for annotation in old_annotations:
                # Find the annotation block and replace it
                start_line = annotation['line'] - 1
                
                # Find the end of the annotation block
                end_line = start_line
                while end_line < len(lines) and (lines[end_line].strip().startswith('//') and '@' in lines[end_line]):
                    end_line += 1
                
                # Generate CTL JSON
                ctl_json = self.convert_to_ctl(annotation, file_path)
                if ctl_json:
                    # Replace the old annotation block
                    new_annotation = f"// @component: {ctl_json}"
                    lines[start_line:end_line] = [new_annotation]
                    changes_made = True
                    print(f"  Converted: {annotation['name']} -> {ctl_json}")
            
            if changes_made:
                # Write back to file
                with open(file_path, 'w', encoding='utf-8') as f:
                    f.write('\n'.join(lines))
                return True
            
            return False
            
        except Exception as e:
            print(f"Error processing {file_path}: {e}")
            return False

    def convert_project(self, project_root: str):
        """Convert all Rust files in the project"""
        rust_files = glob.glob(f"{project_root}/crates/**/*.rs", recursive=True)
        
        print(f"Found {len(rust_files)} Rust files")
        
        converted_count = 0
        for file_path in rust_files:
            # Skip target directories
            if 'target' in file_path or 'examples' in file_path:
                continue
                
            if self.process_file(file_path):
                converted_count += 1
        
        print(f"\nConversion complete: {converted_count} files modified")

if __name__ == "__main__":
    import sys
    
    if len(sys.argv) > 1:
        project_root = sys.argv[1]
    else:
        project_root = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
    
    converter = AnnotationConverter()
    converter.convert_project(project_root)