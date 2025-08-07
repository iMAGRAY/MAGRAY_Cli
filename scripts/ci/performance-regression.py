#!/usr/bin/env python3
"""
Performance Regression Detection System для MAGRAY CLI
Анализирует benchmark results и детектирует performance regressions
"""

import json
import sys
import os
import argparse
import statistics
from pathlib import Path
from datetime import datetime, timedelta
from typing import Dict, List, Optional, Tuple
import subprocess

class PerformanceAnalyzer:
    def __init__(self, baseline_file: str = "benchmark-baseline.json", 
                 threshold: float = 0.1, verbose: bool = False):
        self.baseline_file = baseline_file
        self.threshold = threshold  # 10% regression threshold по умолчанию
        self.verbose = verbose
        self.git_lfs_enabled = self._check_git_lfs()
        self.history_dir = Path("performance-history")
        self.history_dir.mkdir(exist_ok=True)
        self.results = {
            'regressions': [],
            'improvements': [],
            'stable': [],
            'new_benchmarks': [],
            'missing_benchmarks': []
        }
        
        # AI/ML specific benchmarks критичные для MAGRAY CLI
        self.critical_benchmarks = {
            'simd_cosine_distance': {'threshold': 1.0, 'unit': 'ms', 'target': '<500ns'},
            'hnsw_search_1k_vectors': {'threshold': 5.0, 'unit': 'ms', 'target': '<2ms'}, 
            'embedding_cpu_generation': {'threshold': 100.0, 'unit': 'ms', 'target': '<50ms'},
            'vector_index_insertion': {'threshold': 10.0, 'unit': 'ms', 'target': '<5ms'},
            'memory_pool_allocation': {'threshold': 1.0, 'unit': 'ms', 'target': '<500μs'},
            'binary_startup_time': {'threshold': 2000.0, 'unit': 'ms', 'target': '<1s'}
        }
        
    def log(self, message: str, level: str = "INFO"):
        """Logging с цветами"""
        colors = {
            "INFO": "\033[36m",    # Cyan
            "SUCCESS": "\033[32m", # Green  
            "WARNING": "\033[33m", # Yellow
            "ERROR": "\033[31m",   # Red
            "RESET": "\033[0m"
        }
        
        if self.verbose or level in ["WARNING", "ERROR"]:
            print(f"{colors.get(level, '')}{level}: {message}{colors['RESET']}")

    def load_benchmark_results(self, results_file: str) -> Dict:
        """Загружает benchmark results из JSON файла"""
        try:
            if not os.path.exists(results_file):
                self.log(f"Results file not found: {results_file}", "ERROR")
                return {}
                
            with open(results_file, 'r') as f:
                data = json.load(f)
            
            self.log(f"Loaded {len(data)} benchmark results from {results_file}")
            return data
            
        except Exception as e:
            self.log(f"Error loading results: {e}", "ERROR")
            return {}

    def normalize_benchmark_data(self, data: Dict) -> Dict[str, float]:
        """Нормализует benchmark data к единому формату"""
        normalized = {}
        
        # Поддержка разных форматов benchmark результатов
        if 'benchmarks' in data:
            # Criterion format
            for bench in data['benchmarks']:
                name = bench.get('name', bench.get('id', 'unknown'))
                # Используем mean time в nanoseconds
                time_ns = bench.get('mean', {}).get('estimate', 0)
                if time_ns > 0:
                    normalized[name] = time_ns
                    
        elif 'results' in data:
            # Custom format
            for bench_name, result in data['results'].items():
                if isinstance(result, dict) and 'duration_ns' in result:
                    normalized[bench_name] = result['duration_ns']
                elif isinstance(result, (int, float)):
                    normalized[bench_name] = result
                    
        else:
            # Direct format {benchmark_name: time}
            for name, value in data.items():
                if isinstance(value, (int, float)):
                    normalized[name] = value
        
        self.log(f"Normalized {len(normalized)} benchmarks")
        return normalized

    def compare_benchmarks(self, baseline: Dict[str, float], 
                          current: Dict[str, float]) -> Dict:
        """Сравнивает текущие results с baseline"""
        comparison_results = {
            'regressions': [],
            'improvements': [],
            'stable': [],
            'new_benchmarks': [],
            'missing_benchmarks': []
        }
        
        # Find missing benchmarks
        missing = set(baseline.keys()) - set(current.keys())
        for bench_name in missing:
            comparison_results['missing_benchmarks'].append({
                'name': bench_name,
                'baseline_time': baseline[bench_name]
            })
            
        # Find new benchmarks
        new = set(current.keys()) - set(baseline.keys())
        for bench_name in new:
            comparison_results['new_benchmarks'].append({
                'name': bench_name,
                'current_time': current[bench_name]
            })
        
        # Compare existing benchmarks
        common = set(baseline.keys()) & set(current.keys())
        for bench_name in common:
            baseline_time = baseline[bench_name]
            current_time = current[bench_name]
            
            # Calculate percentage change
            change_pct = ((current_time - baseline_time) / baseline_time) * 100
            
            benchmark_result = {
                'name': bench_name,
                'baseline_time': baseline_time,
                'current_time': current_time,
                'change_pct': change_pct,
                'change_ns': current_time - baseline_time
            }
            
            if abs(change_pct) <= self.threshold * 100:
                comparison_results['stable'].append(benchmark_result)
            elif change_pct > self.threshold * 100:
                comparison_results['regressions'].append(benchmark_result)
            else:
                comparison_results['improvements'].append(benchmark_result)
        
        return comparison_results

    def analyze_trends(self, benchmark_name: str, lookback_days: int = 30) -> Dict:
        """Анализирует trends для конкретного benchmark за период"""
        history_files = list(self.history_dir.glob('*.json'))
        
        # Фильтруем файлы по дате
        cutoff_date = datetime.now() - timedelta(days=lookback_days)
        recent_files = []
        
        for file in history_files:
            try:
                # Извлекаем timestamp из имени файла
                timestamp_str = file.stem.split('_')[1:3]  # ['20250807', '021500']
                if len(timestamp_str) == 2:
                    file_datetime = datetime.strptime(f"{timestamp_str[0]}_{timestamp_str[1]}", 
                                                     '%Y%m%d_%H%M%S')
                    if file_datetime >= cutoff_date:
                        recent_files.append((file_datetime, file))
            except:
                continue
        
        # Сортировка по дате
        recent_files.sort(key=lambda x: x[0])
        
        # Извлекаем значения benchmark'а
        values = []
        for datetime_obj, file_path in recent_files:
            try:
                with open(file_path, 'r') as f:
                    data = json.load(f)
                    if benchmark_name in data.get('benchmarks', {}):
                        values.append({
                            'timestamp': datetime_obj,
                            'value': data['benchmarks'][benchmark_name],
                            'commit': data.get('metadata', {}).get('git_commit', 'unknown')[:8]
                        })
            except:
                continue
        
        if len(values) < 2:
            return {'trend': 'insufficient_data', 'values': values}
        
        # Простой trend analysis
        recent_values = [v['value'] for v in values[-5:]]
        older_values = [v['value'] for v in values[:-5]] if len(values) > 5 else []
        
        trend = 'stable'
        if older_values:
            recent_avg = sum(recent_values) / len(recent_values)
            older_avg = sum(older_values) / len(older_values)
            change = ((recent_avg - older_avg) / older_avg) * 100
            
            if change > 10:
                trend = 'degrading'
            elif change < -10:
                trend = 'improving'
        
        return {
            'trend': trend,
            'values': values,
            'data_points': len(values),
            'lookback_days': lookback_days
        }
    
    def generate_report(self, comparison: Dict) -> str:
        """Генерирует comprehensive отчет с trend analysis"""
        git_info = self.get_git_commit_info()
        report_lines = [
            "🔍 MAGRAY CLI Performance Regression Analysis",
            "=" * 60,
            f"Analysis Date: {datetime.now().isoformat()}",
            f"Git Commit: {git_info['full_commit'][:8]} ({git_info['branch']})",
            f"Regression Threshold: {self.threshold * 100:.1f}%",
            f"Analyzer Version: 2.0 (Enhanced)",
            ""
        ]
        
        # Summary statistics
        total_benchmarks = (len(comparison['regressions']) + 
                          len(comparison['improvements']) + 
                          len(comparison['stable']))
        
        report_lines.extend([
            "📊 Summary:",
            f"  Total Benchmarks: {total_benchmarks}",
            f"  🔴 Regressions: {len(comparison['regressions'])}",
            f"  🟢 Improvements: {len(comparison['improvements'])}",
            f"  🔵 Stable: {len(comparison['stable'])}",
            f"  ➕ New: {len(comparison['new_benchmarks'])}",
            f"  ➖ Missing: {len(comparison['missing_benchmarks'])}",
            ""
        ])
        
        # Detailed regressions с trend analysis
        if comparison['regressions']:
            report_lines.extend([
                "🔴 Performance Regressions Analysis:",
                "-" * 50
            ])
            
            # Sort by severity (worst first)
            regressions = sorted(comparison['regressions'], 
                               key=lambda x: x['change_pct'], reverse=True)
            
            for reg in regressions:
                severity = "CRITICAL" if reg['change_pct'] > 50 else "HIGH" if reg['change_pct'] > 20 else "MEDIUM"
                
                # Trend analysis для этого benchmark
                trend_data = self.analyze_trends(reg['name'])
                trend_icon = {
                    'improving': '📈',
                    'degrading': '📉',
                    'stable': '➡️',
                    'insufficient_data': '❓'
                }.get(trend_data['trend'], '❓')
                
                # Проверка critical benchmarks
                is_critical = reg['name'] in self.critical_benchmarks
                critical_marker = " [CRITICAL AI BENCHMARK]" if is_critical else ""
                
                report_lines.append(
                    f"  • {reg['name']}: +{reg['change_pct']:.1f}% ({severity}){critical_marker}"
                )
                report_lines.append(
                    f"    {reg['baseline_time']:.0f}ns → {reg['current_time']:.0f}ns "
                    f"(+{reg['change_ns']:.0f}ns)"
                )
                report_lines.append(
                    f"    Trend (30d): {trend_icon} {trend_data['trend']} "
                    f"({trend_data['data_points']} data points)"
                )
                
                if is_critical:
                    target = self.critical_benchmarks[reg['name']]['target']
                    report_lines.append(f"    Performance Target: {target}")
                
                report_lines.append("")
        
        # Significant improvements
        if comparison['improvements']:
            improvements = [imp for imp in comparison['improvements'] 
                          if imp['change_pct'] < -10]  # Only significant improvements
            if improvements:
                report_lines.extend([
                    "",
                    "🟢 Significant Performance Improvements:",
                    "-" * 40
                ])
                
                improvements = sorted(improvements, key=lambda x: x['change_pct'])
                for imp in improvements[:5]:  # Top 5
                    report_lines.append(
                        f"  • {imp['name']}: {imp['change_pct']:.1f}%"
                    )
                    report_lines.append(
                        f"    {imp['baseline_time']:.0f}ns → {imp['current_time']:.0f}ns "
                        f"({imp['change_ns']:.0f}ns faster)"
                    )
        
        # New benchmarks
        if comparison['new_benchmarks']:
            report_lines.extend([
                "",
                "➕ New Benchmarks:",
                "-" * 40
            ])
            for new in comparison['new_benchmarks'][:10]:  # Limit output
                report_lines.append(f"  • {new['name']}: {new['current_time']:.0f}ns")
        
        # Missing benchmarks
        if comparison['missing_benchmarks']:
            report_lines.extend([
                "",
                "➖ Missing Benchmarks (WARNING):",
                "-" * 40
            ])
            for missing in comparison['missing_benchmarks']:
                report_lines.append(f"  • {missing['name']}")
        
        # Performance targets status
        report_lines.extend([
            "",
            "🎯 Critical AI Performance Targets:",
            "-" * 50
        ])
        
        for bench_name, config in self.critical_benchmarks.items():
            status = "❓ Not measured"
            if bench_name in comparison.get('stable', []):
                status = "✅ Target met (stable)"
            elif any(r['name'] == bench_name for r in comparison.get('regressions', [])):
                status = "❌ Target missed (regression)"
            elif any(i['name'] == bench_name for i in comparison.get('improvements', [])):
                status = "🚀 Target exceeded (improved)"
                
            report_lines.append(f"  • {bench_name}: {config['target']} - {status}")
        
        # Git LFS status
        report_lines.extend([
            "",
            "📊 Performance Tracking Status:",
            "-" * 50,
            f"• Git LFS Enabled: {'✅' if self.git_lfs_enabled else '❌'}",
            f"• Historical Data Points: {len(list(self.history_dir.glob('*.json')))}",
            f"• Baseline File: {self.baseline_file}",
            f"• History Directory: {self.history_dir}"
        ])
        
        return "\n".join(report_lines)

    def save_baseline(self, current_results: Dict[str, float], update_history: bool = True):
        """Сохраняет текущие results как новый baseline с historical tracking"""
        git_info = self.get_git_commit_info()
        baseline_data = {
            'timestamp': datetime.now().isoformat(),
            'benchmarks': current_results,
            'metadata': {
                'total_benchmarks': len(current_results),
                'tool_version': self.get_tool_version(),
                'git_commit': git_info['full_commit'],
                'git_branch': git_info['branch'],
                'analyzer_version': '2.0'
            }
        }
        
        # Сохранение основного baseline
        with open(self.baseline_file, 'w') as f:
            json.dump(baseline_data, f, indent=2)
            
        # Сохранение в historical tracking
        if update_history:
            self._save_historical_data(baseline_data)
            
        self.log(f"Saved baseline with {len(current_results)} benchmarks to {self.baseline_file}")
        
        # Git LFS tracking для больших baseline файлов
        if self.git_lfs_enabled:
            self._setup_git_lfs_tracking()
    
    def _save_historical_data(self, baseline_data: Dict):
        """Сохраняет historical performance data для trend analysis"""
        timestamp = datetime.now().strftime('%Y%m%d_%H%M%S')
        git_info = self.get_git_commit_info()
        
        history_file = self.history_dir / f"performance_{timestamp}_{git_info['commit']}.json"
        
        with open(history_file, 'w') as f:
            json.dump(baseline_data, f, indent=2)
            
        self.log(f"Saved historical data to {history_file}")
    
    def _setup_git_lfs_tracking(self):
        """Настраивает Git LFS tracking для performance files"""
        try:
            # Проверяем существует ли .gitattributes
            gitattributes_path = Path('.gitattributes')
            lfs_rules = [
                'benchmark-baseline.json filter=lfs diff=lfs merge=lfs -text',
                'performance-history/*.json filter=lfs diff=lfs merge=lfs -text',
                '*.benchmark-results filter=lfs diff=lfs merge=lfs -text'
            ]
            
            if gitattributes_path.exists():
                with open(gitattributes_path, 'r') as f:
                    content = f.read()
                    
                # Добавляем rules если их еще нет
                rules_to_add = [rule for rule in lfs_rules if rule not in content]
                if rules_to_add:
                    with open(gitattributes_path, 'a') as f:
                        f.write('\n# Performance baseline tracking\n')
                        f.write('\n'.join(rules_to_add) + '\n')
            else:
                with open(gitattributes_path, 'w') as f:
                    f.write('# Git LFS tracking for MAGRAY CLI performance baselines\n')
                    f.write('\n'.join(lfs_rules) + '\n')
                    
            self.log("Setup Git LFS tracking for performance files")
        except Exception as e:
            self.log(f"Warning: Could not setup Git LFS tracking: {e}", "WARNING")

    def _check_git_lfs(self) -> bool:
        """Проверяет доступность Git LFS для хранения baseline history"""
        try:
            result = subprocess.run(['git', 'lfs', 'version'], 
                                  capture_output=True, text=True)
            return result.returncode == 0
        except:
            return False
    
    def get_tool_version(self) -> str:
        """Получает версию инструментов"""
        try:
            result = subprocess.run(['cargo', '--version'], 
                                  capture_output=True, text=True)
            return result.stdout.strip()
        except:
            return "unknown"
    
    def get_git_commit_info(self) -> Dict[str, str]:
        """Получает информацию о текущем коммите"""
        try:
            commit_hash = subprocess.run(['git', 'rev-parse', 'HEAD'], 
                                       capture_output=True, text=True).stdout.strip()
            branch = subprocess.run(['git', 'rev-parse', '--abbrev-ref', 'HEAD'], 
                                  capture_output=True, text=True).stdout.strip()
            return {'commit': commit_hash[:8], 'branch': branch, 'full_commit': commit_hash}
        except:
            return {'commit': 'unknown', 'branch': 'unknown', 'full_commit': 'unknown'}

    def analyze(self, results_file: str, update_baseline: bool = False) -> bool:
        """Основной анализ performance regression"""
        self.log("🔍 Starting performance regression analysis...")
        
        # Load current results
        current_data = self.load_benchmark_results(results_file)
        if not current_data:
            self.log("No benchmark data found", "ERROR")
            return False
            
        current_results = self.normalize_benchmark_data(current_data)
        if not current_results:
            self.log("No valid benchmark results", "ERROR")
            return False
        
        # Load baseline
        if not os.path.exists(self.baseline_file):
            self.log(f"No baseline found at {self.baseline_file}", "WARNING")
            self.log("Creating initial baseline...", "INFO")
            self.save_baseline(current_results)
            return True
            
        baseline_data = self.load_benchmark_results(self.baseline_file)
        baseline_results = baseline_data.get('benchmarks', {})
        
        if not baseline_results:
            self.log("Invalid baseline data", "ERROR")
            return False
        
        # Compare
        comparison = self.compare_benchmarks(baseline_results, current_results)
        
        # Generate report
        report = self.generate_report(comparison)
        print(report)
        
        # Save detailed report
        report_file = f"performance-report-{datetime.now().strftime('%Y%m%d-%H%M%S')}.txt"
        with open(report_file, 'w') as f:
            f.write(report)
        
        # Update baseline if requested
        if update_baseline:
            self.save_baseline(current_results)
        
        # Enhanced success/failure logic с critical AI benchmarks
        critical_ai_regressions = [
            reg for reg in comparison['regressions']
            if reg['name'] in self.critical_benchmarks and reg['change_pct'] > 15
        ]
        
        severe_regressions = [
            reg for reg in comparison['regressions']
            if reg['change_pct'] > 25
        ]
        
        if critical_ai_regressions:
            self.log(f"❌ CRITICAL AI performance regressions detected: {len(critical_ai_regressions)}", "ERROR")
            for reg in critical_ai_regressions:
                self.log(f"  • {reg['name']}: +{reg['change_pct']:.1f}%", "ERROR")
            return False
        elif severe_regressions:
            self.log(f"❌ SEVERE performance regressions detected: {len(severe_regressions)}", "ERROR")
            return False
        elif comparison['regressions']:
            self.log(f"⚠️ {len(comparison['regressions'])} minor performance regressions detected", "WARNING")
            # Return True for non-critical regressions (don't block CI)
            return True
        else:
            self.log("✅ No performance regressions detected", "SUCCESS")
            return True

def main():
    parser = argparse.ArgumentParser(
        description="MAGRAY CLI Performance Regression Detection"
    )
    parser.add_argument("results_file", 
                       help="Path to benchmark results JSON file")
    parser.add_argument("--baseline", default="benchmark-baseline.json",
                       help="Path to baseline file")
    parser.add_argument("--threshold", type=float, default=0.1,
                       help="Regression threshold (0.1 = 10%%)")
    parser.add_argument("--update-baseline", action="store_true",
                       help="Update baseline after analysis")
    parser.add_argument("--verbose", action="store_true",
                       help="Verbose output")
    
    args = parser.parse_args()
    
    analyzer = PerformanceAnalyzer(
        baseline_file=args.baseline,
        threshold=args.threshold,
        verbose=args.verbose
    )
    
    success = analyzer.analyze(args.results_file, args.update_baseline)
    sys.exit(0 if success else 1)

if __name__ == "__main__":
    main()