#!/usr/bin/env python3
"""
CTL v1.0 to v2.0 Converter
Converts legacy Claude Tensor Language format to new JSON format
"""

import re
import json
import sys
from datetime import datetime
from typing import Dict, List, Optional, Tuple

class CTLConverter:
    """Converts CTL v1.0 format to v2.0 JSON format"""
    
    # Priority mapping
    PRIORITY_MAP = {
        'low': 1,
        'med': 2,
        'medium': 2,
        'high': 4,
        'critical': 5,
        'urgent': 5
    }
    
    # Time conversion patterns
    TIME_PATTERNS = {
        r'(\d+)h': lambda m: f'PT{m.group(1)}H',
        r'(\d+)d': lambda m: f'P{m.group(1)}D',
        r'(\d+)w': lambda m: f'P{int(m.group(1))*7}D',
        r'(\d+)min': lambda m: f'PT{m.group(1)}M',
    }
    
    def __init__(self):
        # Regex patterns for parsing v1.0 format
        self.task_pattern = re.compile(
            r'#([TSABFRPDMCE])(\d+)?\s+(\w+)\s*(?:\[([^\]]+)\])?\s*(?:→\s*(.+))?'
        )
        self.metric_pattern = re.compile(
            r'#M(\d+)?\s+(\w+):\s*(\d+)(\w*)\s*→\s*(\d+)(\w*)\s*(?:\[([^\]]+)\])?\s*(?:\|\s*(.+))?'
        )
        self.dependency_pattern = re.compile(r'#([TSABFRPDMCE]?\d+)')
        
    def convert_time(self, time_str: str) -> str:
        """Convert time format to ISO 8601 duration"""
        time_str = time_str.strip()
        
        for pattern, converter in self.TIME_PATTERNS.items():
            match = re.match(pattern, time_str, re.IGNORECASE)
            if match:
                return converter(match)
        
        # Default to P1D if cannot parse
        return 'P1D'
    
    def parse_attributes(self, attr_str: str) -> Tuple[Optional[int], Optional[str], List[str]]:
        """Parse [priority,time,deps] attributes"""
        if not attr_str:
            return None, None, []
        
        parts = [p.strip() for p in attr_str.split(',')]
        priority = None
        effort = None
        deps = []
        
        for part in parts:
            # Check if it's a priority
            if part.lower() in self.PRIORITY_MAP:
                priority = self.PRIORITY_MAP[part.lower()]
            # Check if it's a time duration
            elif re.match(r'\d+[hdwm]', part, re.IGNORECASE):
                effort = self.convert_time(part)
            # Check if it's a dependency reference
            elif self.dependency_pattern.match(part):
                dep_match = self.dependency_pattern.match(part)
                deps.append(dep_match.group(0).replace('#', '').lower())
            else:
                # Try to parse as dependency name
                deps.append(part.replace('#', '').lower())
        
        return priority, effort, deps
    
    def convert_line(self, line: str) -> Optional[Dict]:
        """Convert a single line from v1.0 to v2.0 format"""
        line = line.strip()
        if not line or line.startswith('//') or line.startswith('---'):
            return None
        
        # Try to match metric pattern first
        metric_match = self.metric_pattern.match(line)
        if metric_match:
            groups = metric_match.groups()
            metric_id = f"m{groups[0]}" if groups[0] else groups[1]
            
            result = {
                "k": "M",
                "id": metric_id,
                "t": groups[1].replace('_', ' ').title(),
                "m": {
                    "cur": int(groups[2]),
                    "tgt": int(groups[4]),
                    "u": groups[3] or groups[5] or "unit"
                }
            }
            
            if groups[6]:  # trend
                result["x_trend"] = groups[6]
            if groups[7]:  # alert threshold
                result["x_alert"] = groups[7]
                
            return result
        
        # Try to match task/architecture/etc pattern
        task_match = self.task_pattern.match(line)
        if task_match:
            groups = task_match.groups()
            kind = groups[0]
            number = groups[1] or ""
            name = groups[2]
            attributes = groups[3]
            result_str = groups[4]
            
            # Build ID
            task_id = f"{kind.lower()}{number}" if number else name.lower()
            
            # Parse attributes
            priority, effort, deps = self.parse_attributes(attributes)
            
            # Build JSON object
            result = {
                "k": kind,
                "id": task_id,
                "t": name.replace('_', ' ').title()[:40]  # Max 40 chars
            }
            
            # Add optional fields
            if priority:
                result["p"] = priority
            if effort:
                result["e"] = effort
            if deps:
                result["d"] = deps
            if result_str:
                result["r"] = result_str.strip()
            
            return result
        
        return None
    
    def convert_file(self, input_file: str, output_file: str = None) -> List[Dict]:
        """Convert entire file from v1.0 to v2.0 format"""
        results = []
        
        with open(input_file, 'r', encoding='utf-8') as f:
            for line in f:
                converted = self.convert_line(line)
                if converted:
                    results.append(converted)
        
        if output_file:
            with open(output_file, 'w', encoding='utf-8') as f:
                for item in results:
                    f.write(json.dumps(item, ensure_ascii=False, separators=(',', ':')) + '\n')
        
        return results
    
    def convert_text(self, text: str) -> List[Dict]:
        """Convert text block from v1.0 to v2.0 format"""
        results = []
        
        for line in text.split('\n'):
            converted = self.convert_line(line)
            if converted:
                results.append(converted)
        
        return results


def main():
    """CLI interface for CTL converter"""
    if len(sys.argv) < 2:
        print("Usage: python ctl_converter.py <input_file> [output_file]")
        print("       python ctl_converter.py -")  # Read from stdin
        sys.exit(1)
    
    converter = CTLConverter()
    
    if sys.argv[1] == '-':
        # Read from stdin
        text = sys.stdin.read()
        results = converter.convert_text(text)
        for item in results:
            print(json.dumps(item, ensure_ascii=False, separators=(',', ':')))
    else:
        # Read from file
        input_file = sys.argv[1]
        output_file = sys.argv[2] if len(sys.argv) > 2 else None
        
        results = converter.convert_file(input_file, output_file)
        
        if not output_file:
            # Print to stdout
            for item in results:
                print(json.dumps(item, ensure_ascii=False, separators=(',', ':')))
        else:
            print(f"Converted {len(results)} items to {output_file}")


if __name__ == "__main__":
    main()