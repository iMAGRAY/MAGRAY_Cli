#!/usr/bin/env python3
"""
CTL v2.0 CLI Tool
Command-line interface for Claude Tensor Language
"""

import json
import sys
import argparse
from datetime import datetime
from typing import List, Dict, Optional
import os
from pathlib import Path

class CTLManager:
    """Manages CTL v2.0 tasks and components"""
    
    def __init__(self, storage_dir: str = ".ctl"):
        self.storage_dir = Path(storage_dir)
        self.storage_dir.mkdir(exist_ok=True)
        
        self.files = {
            'tasks': self.storage_dir / 'tasks.jsonl',
            'completed': self.storage_dir / 'completed.jsonl',
            'metrics': self.storage_dir / 'metrics.jsonl',
            'architecture': self.storage_dir / 'architecture.jsonl'
        }
        
        # Create files if they don't exist
        for file in self.files.values():
            file.touch(exist_ok=True)
    
    def add(self, item: Dict) -> None:
        """Add a new CTL item with validation"""
        # Validate required fields
        required_fields = ['k', 'id', 't']
        for field in required_fields:
            if field not in item:
                raise ValueError(f"Required field '{field}' is missing")
        
        # Validate field constraints
        if len(item['id']) > 32:
            raise ValueError("ID must be <= 32 characters")
        if len(item['t']) > 40:
            raise ValueError("Title must be <= 40 characters")
        if 'p' in item and not (1 <= item['p'] <= 5):
            raise ValueError("Priority must be between 1 and 5")
        
        # Check for duplicate ID
        if self.exists(item['id']):
            raise ValueError(f"Item with ID '{item['id']}' already exists")
        
        # Ensure consistent key order
        ordered_item = {}
        key_order = ['k', 'id', 't', 'p', 'e', 'd', 'r', 'm', 'f']
        for key in key_order:
            if key in item:
                ordered_item[key] = item[key]
        
        # Add remaining x_ fields
        for key, value in item.items():
            if key.startswith('x_'):
                ordered_item[key] = value
        
        # Determine which file to use based on kind
        if ordered_item['k'] == 'T' and ordered_item.get('x_completed'):
            file = self.files['completed']
        elif ordered_item['k'] == 'M':
            file = self.files['metrics']
        elif ordered_item['k'] in ['A', 'C']:
            file = self.files['architecture']
        else:
            file = self.files['tasks']
        
        # Append to file
        with open(file, 'a') as f:
            f.write(json.dumps(ordered_item, ensure_ascii=False, separators=(',', ':')) + '\n')
    
    def exists(self, item_id: str) -> bool:
        """Check if an item with given ID exists"""
        for file in self.files.values():
            if file.exists():
                with open(file, 'r') as f:
                    for line in f:
                        if line.strip():
                            try:
                                item = json.loads(line)
                                if item.get('id') == item_id:
                                    return True
                            except json.JSONDecodeError:
                                pass
        return False
    
    def query(self, kind: Optional[str] = None, priority: Optional[int] = None,
              depends_on: Optional[str] = None, flags: Optional[List[str]] = None) -> List[Dict]:
        """Query CTL items with filters"""
        results = []
        
        # Read from all files
        for file in self.files.values():
            if file.exists():
                with open(file, 'r') as f:
                    for line in f:
                        if line.strip():
                            try:
                                item = json.loads(line)
                                
                                # Apply filters
                                if kind and item.get('k') != kind:
                                    continue
                                if priority and item.get('p') != priority:
                                    continue
                                if depends_on and depends_on not in item.get('d', []):
                                    continue
                                if flags:
                                    item_flags = item.get('f', [])
                                    if not any(f in item_flags for f in flags):
                                        continue
                                
                                results.append(item)
                            except json.JSONDecodeError:
                                pass
        
        return results
    
    def complete(self, task_id: str) -> bool:
        """Mark a task as completed"""
        # Find task
        tasks = self.query(kind='T')
        task = None
        
        for t in tasks:
            if t['id'] == task_id:
                task = t
                break
        
        if not task:
            return False
        
        # Mark as completed
        task['x_completed'] = datetime.now().isoformat()
        task['r'] = task.get('r', 'completed')
        
        # Move to completed file
        self.remove_from_file(self.files['tasks'], task_id)
        self.add(task)
        
        return True
    
    def remove_from_file(self, file: Path, task_id: str) -> None:
        """Remove an item from a file"""
        if not file.exists():
            return
        
        lines = []
        with open(file, 'r') as f:
            for line in f:
                if line.strip():
                    try:
                        item = json.loads(line)
                        if item.get('id') != task_id:
                            lines.append(line)
                    except json.JSONDecodeError:
                        lines.append(line)
        
        with open(file, 'w') as f:
            f.writelines(lines)
    
    def update_metric(self, metric_id: str, current: float) -> bool:
        """Update a metric's current value"""
        metrics = self.query(kind='M')
        
        for m in metrics:
            if m['id'] == metric_id:
                m['m']['cur'] = current
                m['x_updated'] = datetime.now().isoformat()
                
                # Rewrite metrics file
                self.rewrite_metrics(metrics)
                return True
        
        return False
    
    def rewrite_metrics(self, metrics: List[Dict]) -> None:
        """Rewrite the metrics file"""
        with open(self.files['metrics'], 'w') as f:
            for m in metrics:
                f.write(json.dumps(m, ensure_ascii=False, separators=(',', ':')) + '\n')
    
    def today(self) -> List[Dict]:
        """Get today's high-priority tasks"""
        tasks = self.query(kind='T')
        
        # Filter for high priority (4-5)
        high_priority = [t for t in tasks if t.get('p', 0) >= 4]
        
        # Sort by priority (descending)
        high_priority.sort(key=lambda x: x.get('p', 0), reverse=True)
        
        return high_priority
    
    def report(self, format: str = 'text') -> str:
        """Generate a report"""
        tasks = self.query(kind='T')
        completed = self.query()
        completed = [t for t in completed if t.get('x_completed')]
        metrics = self.query(kind='M')
        
        if format == 'markdown':
            report = f"# CTL Status Report\n\n"
            report += f"*Generated: {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}*\n\n"
            
            # Tasks summary
            report += f"## Tasks\n\n"
            report += f"- Active: {len(tasks)}\n"
            report += f"- Completed: {len(completed)}\n"
            
            # Current tasks
            if tasks:
                report += f"\n### Active Tasks\n\n"
                for t in sorted(tasks, key=lambda x: x.get('p', 0), reverse=True):
                    priority = '🔴' * t.get('p', 0)
                    report += f"- {priority} **{t['t']}** (`{t['id']}`)\n"
            
            # Metrics
            if metrics:
                report += f"\n## Metrics\n\n"
                for m in metrics:
                    cur = m['m']['cur']
                    tgt = m['m']['tgt']
                    unit = m['m']['u']
                    progress = (cur / tgt * 100) if tgt > 0 else 0
                    
                    report += f"- **{m['t']}**: {cur}{unit} / {tgt}{unit} ({progress:.0f}%)\n"
            
            return report
        
        else:  # text format
            report = []
            report.append(f"Tasks: {len(tasks)} active, {len(completed)} completed")
            
            if tasks:
                report.append("\nActive tasks:")
                for t in sorted(tasks, key=lambda x: x.get('p', 0), reverse=True):
                    report.append(f"  [{t.get('p', '')}] {t['id']}: {t['t']}")
            
            return '\n'.join(report)
    
    def visualize(self, format: str = 'mermaid') -> str:
        """Generate visualization of dependencies"""
        all_items = []
        for kind in ['T', 'F', 'B', 'E', 'C', 'A']:
            all_items.extend(self.query(kind=kind))
        
        if format == 'mermaid':
            graph = ["```mermaid", "graph TD"]
            
            # Style definitions
            graph.append("    classDef task fill:#e3f2fd,stroke:#1976d2,stroke-width:2px")
            graph.append("    classDef bug fill:#ffebee,stroke:#d32f2f,stroke-width:2px")
            graph.append("    classDef feature fill:#f3e5f5,stroke:#7b1fa2,stroke-width:2px")
            graph.append("    classDef component fill:#e8f5e9,stroke:#388e3c,stroke-width:2px")
            graph.append("    classDef epic fill:#fff3e0,stroke:#f57c00,stroke-width:2px")
            
            # Add nodes with styling
            for item in all_items:
                node_id = item['id'].replace('-', '_')
                label = f"{item['t']}"
                
                # Add priority indicator
                if 'p' in item:
                    priority_stars = '★' * item['p']
                    label = f"{priority_stars} {label}"
                
                # Format node
                if item['k'] == 'T':
                    graph.append(f"    {node_id}[{label}]:::task")
                elif item['k'] == 'B':
                    graph.append(f"    {node_id}[{label}]:::bug")
                elif item['k'] == 'F':
                    graph.append(f"    {node_id}[{label}]:::feature")
                elif item['k'] == 'C':
                    graph.append(f"    {node_id}[{label}]:::component")
                elif item['k'] == 'E':
                    graph.append(f"    {node_id}[{label}]:::epic")
                else:
                    graph.append(f"    {node_id}[{label}]")
            
            # Add dependencies
            for item in all_items:
                if 'd' in item:
                    for dep in item['d']:
                        dep_id = dep.replace('-', '_')
                        item_id = item['id'].replace('-', '_')
                        graph.append(f"    {dep_id} --> {item_id}")
            
            graph.append("```")
            return '\n'.join(graph)
        
        return ""


def parse_json_or_args(args_str: str) -> Dict:
    """Parse either JSON or simple key=value arguments"""
    # Try JSON first
    try:
        return json.loads(args_str)
    except json.JSONDecodeError:
        pass
    
    # Parse as key=value pairs
    result = {}
    parts = args_str.split()
    
    for part in parts:
        if '=' in part:
            key, value = part.split('=', 1)
            
            # Handle special keys
            if key == 'k':
                result['k'] = value.upper()
            elif key == 'p':
                result['p'] = int(value)
            elif key == 'd':
                result['d'] = value.split(',')
            elif key == 'f':
                result['f'] = value.split(',')
            else:
                result[key] = value
    
    return result


def main():
    parser = argparse.ArgumentParser(description='CTL v2.0 CLI Tool')
    subparsers = parser.add_subparsers(dest='command', help='Commands')
    
    # Add command
    add_parser = subparsers.add_parser('add', help='Add a new CTL item')
    add_parser.add_argument('item', help='JSON object or key=value pairs')
    
    # Query command
    query_parser = subparsers.add_parser('query', help='Query CTL items')
    query_parser.add_argument('--kind', '-k', help='Filter by kind (T/A/B/F/M/etc)')
    query_parser.add_argument('--priority', '-p', type=int, help='Filter by priority')
    query_parser.add_argument('--depends-on', '-d', help='Filter by dependency')
    query_parser.add_argument('--flags', '-f', nargs='+', help='Filter by flags')
    
    # Complete command
    complete_parser = subparsers.add_parser('complete', help='Mark task as completed')
    complete_parser.add_argument('task_id', help='Task ID to complete')
    
    # Metric command
    metric_parser = subparsers.add_parser('metric', help='Update metric')
    metric_parser.add_argument('action', choices=['update'], help='Action to perform')
    metric_parser.add_argument('metric_id', help='Metric ID')
    metric_parser.add_argument('--current', '-c', type=float, required=True, help='Current value')
    
    # Today command
    subparsers.add_parser('today', help='Show today\'s high-priority tasks')
    
    # Report command
    report_parser = subparsers.add_parser('report', help='Generate report')
    report_parser.add_argument('--format', '-f', choices=['text', 'markdown'], 
                               default='text', help='Output format')
    
    # Graph command
    graph_parser = subparsers.add_parser('graph', help='Visualize dependencies')
    graph_parser.add_argument('--format', '-f', choices=['mermaid'], 
                              default='mermaid', help='Output format')
    
    args = parser.parse_args()
    
    # Initialize manager
    mgr = CTLManager()
    
    # Execute command
    if args.command == 'add':
        item = parse_json_or_args(args.item)
        mgr.add(item)
        print(f"Added: {item['id']}")
    
    elif args.command == 'query':
        results = mgr.query(
            kind=args.kind,
            priority=args.priority,
            depends_on=args.depends_on,
            flags=args.flags
        )
        
        for item in results:
            print(json.dumps(item, ensure_ascii=False, separators=(',', ':')))
    
    elif args.command == 'complete':
        if mgr.complete(args.task_id):
            print(f"Completed: {args.task_id}")
        else:
            print(f"Task not found: {args.task_id}")
            sys.exit(1)
    
    elif args.command == 'metric':
        if args.action == 'update':
            if mgr.update_metric(args.metric_id, args.current):
                print(f"Updated metric: {args.metric_id} = {args.current}")
            else:
                print(f"Metric not found: {args.metric_id}")
                sys.exit(1)
    
    elif args.command == 'today':
        tasks = mgr.today()
        
        if tasks:
            for t in tasks:
                priority = '🔴' * t.get('p', 0)
                effort = t.get('e', 'unknown')
                print(f"{priority} {t['id']}: {t['t']} ({effort})")
        else:
            print("No high-priority tasks for today")
    
    elif args.command == 'report':
        report = mgr.report(format=args.format)
        print(report)
    
    elif args.command == 'graph':
        graph = mgr.visualize(format=args.format)
        print(graph)
    
    else:
        parser.print_help()


if __name__ == '__main__':
    main()