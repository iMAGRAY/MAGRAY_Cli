#!/usr/bin/env python3
"""
Advanced coverage analysis script for MAGRAY CLI project.
Provides detailed coverage metrics, gap analysis, and testing recommendations.
"""

import subprocess
import json
import os
import sys
from pathlib import Path
from dataclasses import dataclass
from typing import List, Dict, Optional
import argparse

@dataclass
class CoverageMetrics:
    """Coverage metrics for analysis"""
    total_lines: int
    covered_lines: int 
    coverage_percentage: float
    branch_coverage: float
    functions_covered: int
    functions_total: int

@dataclass 
class ModuleCoverage:
    """Coverage data for a single module"""
    name: str
    path: str
    lines_covered: int
    lines_total: int
    coverage_percent: float
    functions_covered: int
    functions_total: int
    critical: bool = False

class CoverageAnalyzer:
    """Advanced coverage analysis and reporting"""
    
    def __init__(self, project_root: Path):
        self.project_root = project_root
        self.coverage_dir = project_root / "coverage_report"
        self.critical_modules = {
            # AI core modules
            "crates/ai/src/gpu_pipeline.rs",
            "crates/ai/src/model_downloader.rs", 
            "crates/ai/src/reranker_qwen3.rs",
            "crates/ai/src/reranker_qwen3_optimized.rs",
            "crates/ai/src/tokenizer.rs",
            "crates/ai/src/reranking.rs",
            
            # CLI core
            "crates/cli/src/main.rs",
            "crates/cli/src/agent.rs",
            
            # Memory orchestration
            "crates/memory/src/orchestration/memory_orchestrator.rs",
            "crates/memory/src/orchestration/operation_executor.rs",
            "crates/memory/src/orchestration/orchestration_facade.rs",
            
            # LLM providers
            "crates/llm/src/providers/anthropic_provider.rs",
            "crates/llm/src/providers/openai_provider.rs", 
            "crates/llm/src/providers/groq_provider.rs",
            "crates/llm/src/providers/azure_provider.rs",
            "crates/llm/src/providers/local_provider.rs",
            
            # Tools execution
            "crates/tools/src/execution/pipeline.rs",
            "crates/tools/src/execution/security_enforcer.rs",
        }
        
    def run_coverage(self, target: Optional[str] = None) -> bool:
        """Run cargo tarpaulin coverage analysis"""
        print("Running coverage analysis with tarpaulin...")
        
        cmd = ["cargo", "tarpaulin", "--workspace"]
        
        if target:
            cmd.extend(["--package", target])
            
        # Add coverage options
        cmd.extend([
            "--out", "Html", "Lcov", "Json",
            "--output-dir", "coverage_report", 
            "--timeout", "300",
            "--follow-exec",
            "--run-types", "Tests",
            "--ignore-panics",
            "--verbose"
        ])
        
        try:
            result = subprocess.run(cmd, cwd=self.project_root, 
                                  capture_output=True, text=True)
            
            if result.returncode != 0:
                print(f"Coverage failed: {result.stderr}")
                return False
                
            print("Coverage analysis completed")
            return True
            
        except Exception as e:
            print(f"Coverage error: {e}")
            return False
            
    def analyze_coverage(self) -> Dict:
        """Analyze coverage results from tarpaulin output"""
        
        # Load JSON coverage report
        json_report_path = self.coverage_dir / "tarpaulin-report.json"
        
        if not json_report_path.exists():
            print("No JSON coverage report found")
            return {}
            
        try:
            with open(json_report_path) as f:
                data = json.load(f)
        except Exception as e:
            print(f"Failed to parse coverage report: {e}")
            return {}
            
        return self._process_coverage_data(data)
        
    def _process_coverage_data(self, data: Dict) -> Dict:
        """Process raw coverage data into analysis"""
        
        analysis = {
            "overall_metrics": self._calculate_overall_metrics(data),
            "module_coverage": self._analyze_modules(data),
            "critical_gaps": self._find_critical_gaps(data),
            "recommendations": self._generate_recommendations(data)
        }
        
        return analysis
        
    def _calculate_overall_metrics(self, data: Dict) -> CoverageMetrics:
        """Calculate overall coverage metrics"""
        
        # Extract metrics from tarpaulin JSON format
        total_lines = data.get("lines", {}).get("count", 0)
        covered_lines = data.get("lines", {}).get("covered", 0)
        
        coverage_percent = (covered_lines / total_lines * 100) if total_lines > 0 else 0
        
        # Branch coverage if available
        branches = data.get("branches", {})
        branch_coverage = 0
        if branches:
            branch_total = branches.get("count", 0)
            branch_covered = branches.get("covered", 0)
            branch_coverage = (branch_covered / branch_total * 100) if branch_total > 0 else 0
            
        # Function coverage
        functions = data.get("functions", {})
        func_total = functions.get("count", 0)
        func_covered = functions.get("covered", 0)
        
        return CoverageMetrics(
            total_lines=total_lines,
            covered_lines=covered_lines,
            coverage_percentage=coverage_percent,
            branch_coverage=branch_coverage,
            functions_covered=func_covered,
            functions_total=func_total
        )
        
    def _analyze_modules(self, data: Dict) -> List[ModuleCoverage]:
        """Analyze coverage for individual modules"""
        
        modules = []
        
        # Process file-level coverage data
        files_data = data.get("files", {})
        
        for file_path, file_data in files_data.items():
            if not file_path.endswith(".rs"):
                continue
                
            lines = file_data.get("lines", {})
            functions = file_data.get("functions", {})
            
            lines_total = lines.get("count", 0)
            lines_covered = lines.get("covered", 0)
            coverage_percent = (lines_covered / lines_total * 100) if lines_total > 0 else 0
            
            func_total = functions.get("count", 0)
            func_covered = functions.get("covered", 0)
            
            is_critical = any(critical in file_path for critical in self.critical_modules)
            
            module = ModuleCoverage(
                name=Path(file_path).stem,
                path=file_path,
                lines_covered=lines_covered,
                lines_total=lines_total,
                coverage_percent=coverage_percent,
                functions_covered=func_covered,
                functions_total=func_total,
                critical=is_critical
            )
            
            modules.append(module)
            
        return sorted(modules, key=lambda m: (not m.critical, m.coverage_percent))
        
    def _find_critical_gaps(self, data: Dict) -> List[Dict]:
        """Find critical coverage gaps that need immediate attention"""
        
        gaps = []
        
        # Check critical modules with low coverage
        modules = self._analyze_modules(data)
        
        for module in modules:
            if module.critical and module.coverage_percent < 50:
                gaps.append({
                    "type": "critical_module_uncovered",
                    "module": module.path,
                    "coverage": module.coverage_percent,
                    "priority": "HIGH",
                    "impact": "Core functionality at risk"
                })
                
        # Check for completely uncovered files
        for module in modules:
            if module.coverage_percent == 0 and module.lines_total > 50:
                gaps.append({
                    "type": "large_uncovered_module", 
                    "module": module.path,
                    "lines": module.lines_total,
                    "priority": "MEDIUM",
                    "impact": "Potential bugs undetected"
                })
                
        return gaps
        
    def _generate_recommendations(self, data: Dict) -> List[Dict]:
        """Generate specific testing recommendations"""
        
        recommendations = []
        
        overall = self._calculate_overall_metrics(data)
        
        # Overall coverage recommendations
        if overall.coverage_percentage < 80:
            gap = 80 - overall.coverage_percentage
            recommendations.append({
                "type": "overall_coverage",
                "message": f"Need {gap:.1f}% more coverage to reach 80% target",
                "priority": "HIGH",
                "actions": [
                    "Focus on critical untested modules",
                    "Add integration tests for core workflows", 
                    "Implement property-based tests for algorithms"
                ]
            })
            
        # Branch coverage
        if overall.branch_coverage < 70:
            recommendations.append({
                "type": "branch_coverage", 
                "message": f"Branch coverage at {overall.branch_coverage:.1f}% - add error path tests",
                "priority": "MEDIUM",
                "actions": [
                    "Test error handling paths",
                    "Add edge case scenarios",
                    "Test fallback mechanisms"
                ]
            })
            
        # Critical module specific recommendations
        modules = self._analyze_modules(data)
        critical_uncovered = [m for m in modules if m.critical and m.coverage_percent < 60]
        
        if critical_uncovered:
            recommendations.append({
                "type": "critical_modules",
                "message": f"{len(critical_uncovered)} critical modules under 60% coverage",
                "priority": "CRITICAL",
                "modules": [m.path for m in critical_uncovered],
                "actions": [
                    "Create unit tests for core functions",
                    "Add mock-based integration tests", 
                    "Test failure scenarios"
                ]
            })
            
        return recommendations
        
    def print_detailed_report(self, analysis: Dict):
        """Print comprehensive coverage analysis report"""
        
        print("\n" + "="*80)
        print("MAGRAY CLI COVERAGE ANALYSIS REPORT")
        print("="*80)
        
        # Overall metrics
        metrics = analysis["overall_metrics"]
        print(f"\nOVERALL COVERAGE:")
        print(f"  Lines: {metrics.covered_lines}/{metrics.total_lines} ({metrics.coverage_percentage:.1f}%)")
        print(f"  Branches: {metrics.branch_coverage:.1f}%")
        print(f"  Functions: {metrics.functions_covered}/{metrics.functions_total}")
        
        # Coverage target progress
        target = 80.0
        gap = target - metrics.coverage_percentage
        if gap > 0:
            print(f"  Gap to 80% target: {gap:.1f}%")
            lines_needed = int(gap / 100 * metrics.total_lines)
            print(f"  Estimated lines to test: ~{lines_needed}")
        else:
            print(f"  Target 80% coverage achieved!")
            
        # Critical gaps
        gaps = analysis["critical_gaps"]
        if gaps:
            print(f"\nCRITICAL COVERAGE GAPS ({len(gaps)}):")
            for gap in gaps[:10]:  # Top 10 critical gaps
                print(f"  MISSING: {gap['module']} - {gap.get('coverage', 0):.1f}% covered")
                
        # Module breakdown
        modules = analysis["module_coverage"] 
        critical_modules = [m for m in modules if m.critical]
        
        if critical_modules:
            print(f"\nCRITICAL MODULES COVERAGE:")
            for module in critical_modules[:15]:
                status = "OK" if module.coverage_percent >= 70 else "NEEDS WORK"
                print(f"  {status} {module.name}: {module.coverage_percent:.1f}% ({module.lines_covered}/{module.lines_total})")
                
        # Low coverage modules
        low_coverage = [m for m in modules if m.coverage_percent < 30 and m.lines_total > 20]
        if low_coverage:
            print(f"\nLOW COVERAGE MODULES (Top 10):")
            for module in low_coverage[:10]:
                print(f"  LOW: {module.name}: {module.coverage_percent:.1f}% ({module.lines_total} lines)")
                
        # Recommendations
        recommendations = analysis["recommendations"]
        if recommendations:
            print(f"\nTESTING RECOMMENDATIONS:")
            for rec in recommendations:
                priority = rec["priority"]
                print(f"  {priority}: {rec['message']}")
                if "actions" in rec:
                    for action in rec["actions"][:3]:
                        print(f"    - {action}")
                        
        print(f"\nHTML Report: {self.coverage_dir}/tarpaulin-report.html")
        print("="*80)
        
    def generate_test_plan(self, analysis: Dict) -> Dict:
        """Generate specific test implementation plan"""
        
        plan = {
            "phase_1_critical": [],
            "phase_2_integration": [],
            "phase_3_property_based": [],
            "phase_4_fuzzing": []
        }
        
        # Phase 1: Critical modules unit tests
        modules = analysis["module_coverage"]
        critical_uncovered = [m for m in modules if m.critical and m.coverage_percent < 60]
        
        for module in critical_uncovered[:8]:  # Top 8 critical
            plan["phase_1_critical"].append({
                "module": module.path,
                "current_coverage": module.coverage_percent,
                "target_coverage": 80,
                "estimated_tests": max(5, int(module.lines_total * 0.3)),
                "priority": "CRITICAL"
            })
            
        # Phase 2: Integration workflows
        integration_areas = [
            "AI Pipeline: embedding -> reranking -> storage",
            "LLM Communication: provider -> retry -> response", 
            "Memory Operations: search -> promote -> cache",
            "Tool Execution: validate -> execute -> monitor",
            "CLI Commands: parse -> route -> execute"
        ]
        
        for area in integration_areas:
            plan["phase_2_integration"].append({
                "workflow": area,
                "test_type": "integration",
                "estimated_effort": "medium",
                "dependencies": ["mocks", "test-fixtures"]
            })
            
        # Phase 3: Property-based testing areas
        property_areas = [
            "HNSW index operations maintain invariants",
            "Vector similarity calculations are symmetric", 
            "Embedding dimensions remain consistent",
            "Cache eviction preserves most accessed items",
            "Tokenization preserves text semantics"
        ]
        
        for area in property_areas:
            plan["phase_3_property_based"].append({
                "property": area,
                "test_framework": "proptest",
                "complexity": "medium"
            })
            
        # Phase 4: Fuzzing targets
        fuzz_targets = [
            "Tokenizer input parsing",
            "JSON config validation",
            "Command line argument parsing", 
            "Vector deserialization",
            "Query parsing and validation"
        ]
        
        for target in fuzz_targets:
            plan["phase_4_fuzzing"].append({
                "target": target,
                "tool": "cargo-fuzz",
                "priority": "low"
            })
            
        return plan

def main():
    parser = argparse.ArgumentParser(description="Advanced coverage analysis for MAGRAY CLI")
    parser.add_argument("--target", help="Specific crate to analyze")
    parser.add_argument("--run", action="store_true", help="Run coverage before analysis")
    parser.add_argument("--plan", action="store_true", help="Generate test implementation plan")
    
    args = parser.parse_args()
    
    project_root = Path(__file__).parent.parent
    analyzer = CoverageAnalyzer(project_root)
    
    # Run coverage if requested
    if args.run:
        if not analyzer.run_coverage(args.target):
            sys.exit(1)
            
    # Analyze results
    analysis = analyzer.analyze_coverage()
    if not analysis:
        print("❌ No coverage data available. Run with --run first.")
        sys.exit(1)
        
    # Print detailed report
    analyzer.print_detailed_report(analysis)
    
    # Generate test plan if requested
    if args.plan:
        test_plan = analyzer.generate_test_plan(analysis)
        
        print(f"\nTEST IMPLEMENTATION PLAN:")
        print("="*50)
        
        for phase, items in test_plan.items():
            if items:
                phase_name = phase.replace("_", " ").title()
                print(f"\n{phase_name}:")
                for i, item in enumerate(items, 1):
                    if "module" in item:
                        print(f"  {i}. {item['module']} -> {item['target_coverage']}% coverage")
                    elif "workflow" in item:
                        print(f"  {i}. {item['workflow']}")
                    elif "property" in item:
                        print(f"  {i}. {item['property']}")
                    elif "target" in item:
                        print(f"  {i}. {item['target']}")

if __name__ == "__main__":
    main()