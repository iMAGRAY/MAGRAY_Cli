#!/usr/bin/env python3
"""
Automatic CTL Metrics Generator
Generates CTL tasks and metrics based on code analysis
"""

import os
import re
import json
import glob
import subprocess
from datetime import datetime
from typing import Dict, List, Optional

class CtlMetricsGenerator:
    def __init__(self, project_root: str):
        self.project_root = project_root
        self.generated_items = []

    def analyze_codebase(self) -> Dict:
        """Analyze codebase and generate metrics"""
        stats = {
            'rust_files': 0,
            'lines_of_code': 0,
            'test_files': 0,
            'components_with_ctl': 0,
            'todo_comments': 0,
            'fixme_comments': 0,
            'performance_issues': 0,
        }
        
        rust_files = glob.glob(f"{self.project_root}/crates/**/*.rs", recursive=True)
        
        for file_path in rust_files:
            if 'target' in file_path:
                continue
                
            stats['rust_files'] += 1
            
            if 'test' in file_path or file_path.endswith('_test.rs'):
                stats['test_files'] += 1
            
            try:
                with open(file_path, 'r', encoding='utf-8') as f:
                    content = f.read()
                    
                stats['lines_of_code'] += len([l for l in content.split('\n') if l.strip()])
                
                # Count CTL annotations
                if '@component:' in content and '{' in content:
                    stats['components_with_ctl'] += 1
                
                # Count issues
                stats['todo_comments'] += len(re.findall(r'//.*TODO', content, re.IGNORECASE))
                stats['fixme_comments'] += len(re.findall(r'//.*FIXME', content, re.IGNORECASE))
                
                # Performance issues patterns
                if re.search(r'\.iter\(\).*\.find\(|for.*in.*\.iter\(\)', content):
                    stats['performance_issues'] += 1
                    
            except Exception as e:
                print(f"Error analyzing {file_path}: {e}")
        
        return stats

    def generate_project_metrics(self, stats: Dict) -> List[Dict]:
        """Generate project-level metrics"""
        metrics = []
        
        # Code coverage metric
        test_coverage = min(95, (stats['test_files'] / max(1, stats['rust_files'])) * 100)
        metrics.append({
            "k": "M",
            "id": "test_coverage",
            "t": "Test coverage",
            "m": {"cur": int(test_coverage), "tgt": 80, "u": "%"},
            "f": ["testing", "quality"]
        })
        
        # CTL adoption
        ctl_adoption = (stats['components_with_ctl'] / max(1, stats['rust_files'])) * 100
        metrics.append({
            "k": "M", 
            "id": "ctl_adoption",
            "t": "CTL annotation coverage",
            "m": {"cur": int(ctl_adoption), "tgt": 60, "u": "%"},
            "f": ["documentation", "ctl"]
        })
        
        # Technical debt
        debt_score = min(100, (stats['todo_comments'] + stats['fixme_comments']) / 10)
        metrics.append({
            "k": "M",
            "id": "tech_debt",
            "t": "Technical debt level", 
            "m": {"cur": int(debt_score), "tgt": 20, "u": "score"},
            "f": ["debt", "maintenance"]
        })
        
        # Performance issues
        metrics.append({
            "k": "M",
            "id": "perf_issues",
            "t": "Performance bottlenecks",
            "m": {"cur": stats['performance_issues'], "tgt": 5, "u": "count"},
            "f": ["performance", "optimization"]
        })
        
        return metrics

    def generate_critical_tasks(self, stats: Dict) -> List[Dict]:
        """Generate critical tasks based on analysis"""
        tasks = []
        
        # High priority: Vector search optimization
        if stats['performance_issues'] > 10:
            tasks.append({
                "k": "T",
                "id": "fix_vector_search",
                "t": "Optimize O(n) vector search to HNSW",
                "p": 5,
                "e": "P1W", 
                "r": "100x_speedup",
                "f": ["critical", "performance"]
            })
        
        # Test coverage improvement
        if stats['test_files'] < stats['rust_files'] * 0.7:
            tasks.append({
                "k": "T",
                "id": "improve_coverage",
                "t": "Increase test coverage to 80%",
                "p": 4,
                "e": "P1W",
                "r": "production_ready",
                "f": ["testing", "quality"]
            })
        
        # CTL adoption
        if stats['components_with_ctl'] < stats['rust_files'] * 0.5:
            tasks.append({
                "k": "T", 
                "id": "add_ctl_annotations",
                "t": "Add CTL annotations to all components",
                "p": 3,
                "e": "P3D",
                "r": "full_coverage",
                "f": ["documentation", "ctl"]
            })
        
        # Technical debt cleanup
        if stats['todo_comments'] + stats['fixme_comments'] > 50:
            tasks.append({
                "k": "T",
                "id": "cleanup_debt",
                "t": "Resolve TODO/FIXME comments",
                "p": 3,
                "e": "P1W",
                "r": "clean_codebase",
                "f": ["maintenance", "debt"]
            })
        
        return tasks

    def generate_bugs_from_patterns(self) -> List[Dict]:
        """Generate bugs based on code patterns"""
        bugs = []
        
        # Analyze for common issues
        rust_files = glob.glob(f"{self.project_root}/crates/**/*.rs", recursive=True)
        
        memory_leaks = 0
        async_issues = 0
        error_handling = 0
        
        for file_path in rust_files:
            if 'target' in file_path:
                continue
                
            try:
                with open(file_path, 'r', encoding='utf-8') as f:
                    content = f.read()
                
                # Memory leak patterns
                if re.search(r'Box::leak|mem::forget|Rc::leak', content):
                    memory_leaks += 1
                
                # Async issues
                if re.search(r'\.await.*\.await|async.*block_on', content):
                    async_issues += 1
                
                # Poor error handling
                if re.search(r'\.unwrap\(\)|\.expect\(', content) and 'test' not in file_path:
                    error_handling += 1
                    
            except Exception:
                continue
        
        if memory_leaks > 0:
            bugs.append({
                "k": "B",
                "id": "memory_leaks",
                "t": f"Potential memory leaks ({memory_leaks} files)",
                "p": 4,
                "f": ["memory", "leak"]
            })
        
        if error_handling > 10:
            bugs.append({
                "k": "B", 
                "id": "poor_error_handling",
                "t": "Excessive unwrap() usage",
                "p": 3,
                "f": ["error_handling", "stability"]
            })
        
        return bugs

    def generate_features_roadmap(self) -> List[Dict]:
        """Generate feature roadmap"""
        features = [
            {
                "k": "F",
                "id": "gpu_acceleration", 
                "t": "CUDA/OpenCL GPU acceleration",
                "p": 4,
                "e": "P2W",
                "d": ["vector_index", "embedding_cache"],
                "r": "10x_inference",
                "f": ["gpu", "performance"]
            },
            {
                "k": "F",
                "id": "distributed_memory",
                "t": "Distributed memory system",
                "p": 3,
                "e": "P3W", 
                "d": ["vector_store", "promotion_engine"],
                "r": "scalable_memory",
                "f": ["distributed", "scaling"]
            },
            {
                "k": "F",
                "id": "model_quantization",
                "t": "Model quantization support",
                "p": 3,
                "e": "P1W",
                "d": ["llm_client"],
                "r": "smaller_models",
                "f": ["optimization", "memory"]
            }
        ]
        
        return features

    def generate_all(self) -> List[Dict]:
        """Generate all CTL items"""
        print("Analyzing codebase...")
        stats = self.analyze_codebase()
        
        print(f"Found: {stats['rust_files']} Rust files, {stats['lines_of_code']} LOC")
        print(f"Tests: {stats['test_files']}, CTL components: {stats['components_with_ctl']}")
        print(f"Issues: {stats['todo_comments']} TODOs, {stats['fixme_comments']} FIXMEs")
        
        all_items = []
        
        # Generate different types
        all_items.extend(self.generate_project_metrics(stats))
        all_items.extend(self.generate_critical_tasks(stats))
        all_items.extend(self.generate_bugs_from_patterns())
        all_items.extend(self.generate_features_roadmap())
        
        return all_items

    def save_to_file(self, items: List[Dict], filename: str):
        """Save CTL items to JSONL file"""
        os.makedirs(os.path.dirname(filename), exist_ok=True)
        
        with open(filename, 'w', encoding='utf-8') as f:
            for item in items:
                f.write(json.dumps(item, separators=(',', ':')) + '\n')
        
        print(f"Saved {len(items)} CTL items to {filename}")

if __name__ == "__main__":
    import sys
    
    if len(sys.argv) > 1:
        project_root = sys.argv[1] 
    else:
        project_root = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
    
    generator = CtlMetricsGenerator(project_root)
    items = generator.generate_all()
    
    # Save to .ctl directory
    ctl_dir = os.path.join(project_root, '.ctl')
    generator.save_to_file(items, os.path.join(ctl_dir, 'auto_generated.jsonl'))
    
    print("\nGenerated CTL items:")
    for item in items:
        kind_names = {"T": "Task", "M": "Metric", "B": "Bug", "F": "Feature"}
        kind_name = kind_names.get(item['k'], item['k'])
        print(f"  {kind_name}: {item['t']} ({item['id']})")