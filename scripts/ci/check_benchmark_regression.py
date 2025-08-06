#!/usr/bin/env python3
"""
MAGRAY CLI - Performance Regression Detection
Анализирует benchmark результаты и детектирует regression > 10%
"""

import json
import sys
import os
from typing import Dict, List, Any, Optional
from dataclasses import dataclass
from pathlib import Path

@dataclass
class BenchmarkResult:
    """Результат одного benchmark теста"""
    name: str
    value: float
    unit: str
    lower_is_better: bool = True

@dataclass 
class RegressionResult:
    """Результат анализа регрессии"""
    benchmark_name: str
    current_value: float
    baseline_value: float
    change_percent: float
    is_regression: bool
    severity: str  # 'minor', 'major', 'critical'

class PerformanceAnalyzer:
    """Анализатор производительности для MAGRAY CLI"""
    
    # Критические benchmark'и для AI операций
    CRITICAL_BENCHMARKS = {
        'hnsw_search_latency': {'threshold': 5.0, 'unit': 'ms'},
        'vector_distance_calculation': {'threshold': 1.0, 'unit': 'ms'}, 
        'embedding_generation': {'threshold': 100.0, 'unit': 'ms'},
        'memory_allocation': {'threshold': 10.0, 'unit': 'MB'},
        'startup_time': {'threshold': 2000.0, 'unit': 'ms'},
    }
    
    # Пороги регрессии
    REGRESSION_THRESHOLDS = {
        'minor': 5.0,    # 5% regression - warning
        'major': 10.0,   # 10% regression - fail PR
        'critical': 25.0  # 25% regression - block release
    }
    
    def __init__(self, results_file: str, baseline_file: str = "benchmark-baseline.json"):
        self.results_file = Path(results_file)
        self.baseline_file = Path(baseline_file)
        
    def load_benchmark_results(self, file_path: Path) -> Dict[str, BenchmarkResult]:
        """Загружает результаты benchmark'ов из JSON файла"""
        try:
            with open(file_path, 'r') as f:
                data = json.load(f)
            
            results = {}
            # Парсинг различных форматов benchmark результатов
            if isinstance(data, list):
                # Cargo bench JSON format
                for item in data:
                    if 'id' in item and 'value' in item:
                        name = item['id']
                        value = float(item['value'])
                        unit = item.get('unit', 'ns')
                        results[name] = BenchmarkResult(name, value, unit)
            elif isinstance(data, dict):
                # Custom benchmark format
                for name, result in data.items():
                    if isinstance(result, dict):
                        value = float(result.get('value', result.get('time', 0)))
                        unit = result.get('unit', 'ns')
                        results[name] = BenchmarkResult(name, value, unit)
                    else:
                        results[name] = BenchmarkResult(name, float(result), 'ns')
                        
            return results
        except Exception as e:
            print(f"❌ Ошибка загрузки benchmark результатов из {file_path}: {e}")
            return {}
    
    def normalize_unit(self, value: float, unit: str) -> float:
        """Нормализует значения к миллисекундам для сравнения"""
        unit_multipliers = {
            'ns': 1e-6,   # nanoseconds to ms
            'us': 1e-3,   # microseconds to ms  
            'ms': 1.0,    # milliseconds
            's': 1000.0,  # seconds to ms
            'MB': 1.0,    # megabytes (для памяти)
            'KB': 1e-3,   # kilobytes to MB
            'B': 1e-6,    # bytes to MB
        }
        return value * unit_multipliers.get(unit, 1.0)
    
    def analyze_regression(self, current: Dict[str, BenchmarkResult], 
                          baseline: Dict[str, BenchmarkResult]) -> List[RegressionResult]:
        """Анализирует регрессии производительности"""
        regressions = []
        
        for name, current_result in current.items():
            if name not in baseline:
                print(f"⚠️ Новый benchmark: {name} = {current_result.value} {current_result.unit}")
                continue
                
            baseline_result = baseline[name]
            
            # Нормализация значений для сравнения
            current_normalized = self.normalize_unit(current_result.value, current_result.unit)
            baseline_normalized = self.normalize_unit(baseline_result.value, baseline_result.unit)
            
            # Расчет изменения в процентах
            if baseline_normalized == 0:
                continue
                
            change_percent = ((current_normalized - baseline_normalized) / baseline_normalized) * 100
            
            # Определение является ли это регрессией
            is_regression = change_percent > self.REGRESSION_THRESHOLDS['minor']
            
            # Определение severity
            severity = 'minor'
            if change_percent > self.REGRESSION_THRESHOLDS['critical']:
                severity = 'critical'
            elif change_percent > self.REGRESSION_THRESHOLDS['major']:
                severity = 'major'
                
            if is_regression or abs(change_percent) > 2:  # Show significant changes
                regressions.append(RegressionResult(
                    benchmark_name=name,
                    current_value=current_normalized,
                    baseline_value=baseline_normalized, 
                    change_percent=change_percent,
                    is_regression=is_regression,
                    severity=severity
                ))
        
        return regressions
    
    def check_critical_thresholds(self, current: Dict[str, BenchmarkResult]) -> List[str]:
        """Проверяет критические пороги производительности"""
        violations = []
        
        for benchmark_name, thresholds in self.CRITICAL_BENCHMARKS.items():
            if benchmark_name in current:
                result = current[benchmark_name]
                normalized_value = self.normalize_unit(result.value, result.unit)
                threshold = thresholds['threshold']
                
                if normalized_value > threshold:
                    violations.append(
                        f"{benchmark_name}: {normalized_value:.2f} {thresholds['unit']} "
                        f"(превышает порог {threshold} {thresholds['unit']})"
                    )
        
        return violations
    
    def generate_report(self, regressions: List[RegressionResult], 
                       violations: List[str]) -> str:
        """Генерирует отчет о производительности"""
        report = []
        report.append("# 🏁 Performance Regression Analysis Report")
        report.append(f"**Date**: {os.popen('date').read().strip()}")
        report.append("")
        
        # Общая сводка
        total_regressions = len([r for r in regressions if r.is_regression])
        critical_regressions = len([r for r in regressions if r.severity == 'critical'])
        major_regressions = len([r for r in regressions if r.severity == 'major'])
        
        report.append("## Executive Summary")
        report.append(f"- **Total Regressions**: {total_regressions}")
        report.append(f"- **Critical**: {critical_regressions}")
        report.append(f"- **Major**: {major_regressions}")  
        report.append(f"- **Threshold Violations**: {len(violations)}")
        report.append("")
        
        # Статус
        if critical_regressions > 0 or violations:
            report.append("## ❌ PERFORMANCE STATUS: CRITICAL")
            report.append("Critical performance regressions detected - immediate action required!")
        elif major_regressions > 0:
            report.append("## ⚠️ PERFORMANCE STATUS: WARNING")  
            report.append("Major performance regressions detected - review required.")
        else:
            report.append("## ✅ PERFORMANCE STATUS: PASSED")
            report.append("No critical performance issues detected.")
            
        report.append("")
        
        # Детальные регрессии
        if regressions:
            report.append("## Detailed Regression Analysis")
            report.append("")
            
            for regression in sorted(regressions, key=lambda x: x.change_percent, reverse=True):
                icon = "❌" if regression.severity == "critical" else "⚠️" if regression.severity == "major" else "📊"
                report.append(f"### {icon} {regression.benchmark_name}")
                report.append(f"- **Current**: {regression.current_value:.3f} ms")
                report.append(f"- **Baseline**: {regression.baseline_value:.3f} ms")
                report.append(f"- **Change**: {regression.change_percent:+.1f}%")
                report.append(f"- **Severity**: {regression.severity.upper()}")
                report.append("")
        
        # Пороги нарушения
        if violations:
            report.append("## Critical Threshold Violations")
            report.append("")
            for violation in violations:
                report.append(f"- ❌ {violation}")
            report.append("")
            
        return "\n".join(report)
    
    def run_analysis(self) -> int:
        """Запускает полный анализ производительности"""
        print("🔍 Analyzing MAGRAY CLI Performance...")
        
        # Загрузка результатов
        current_results = self.load_benchmark_results(self.results_file)
        if not current_results:
            print("❌ Не удалось загрузить текущие benchmark результаты")
            return 1
            
        print(f"📊 Загружено {len(current_results)} benchmark результатов")
        
        # Проверка критических порогов
        violations = self.check_critical_thresholds(current_results)
        
        # Анализ регрессий (если есть baseline)
        regressions = []
        if self.baseline_file.exists():
            baseline_results = self.load_benchmark_results(self.baseline_file)
            if baseline_results:
                regressions = self.analyze_regression(current_results, baseline_results)
                print(f"📈 Найдено {len(regressions)} изменений производительности")
        else:
            print("⚠️ Baseline файл не найден - создаем новый baseline")
            # Сохраняем текущие результаты как baseline для будущих сравнений
            with open(self.baseline_file, 'w') as f:
                json.dump({name: {'value': result.value, 'unit': result.unit} 
                          for name, result in current_results.items()}, f, indent=2)
        
        # Генерация отчета
        report = self.generate_report(regressions, violations)
        
        # Сохранение отчета
        with open("performance-report.md", "w") as f:
            f.write(report)
            
        print("\n" + "="*50)
        print(report)
        print("="*50)
        
        # Определение статуса выхода
        critical_issues = len([r for r in regressions if r.severity == 'critical']) + len(violations)
        major_issues = len([r for r in regressions if r.severity == 'major'])
        
        if critical_issues > 0:
            print(f"❌ CRITICAL: {critical_issues} критических проблем производительности!")
            return 2  # Critical failure
        elif major_issues > 0:
            print(f"⚠️ WARNING: {major_issues} значительных регрессий производительности")
            return 1  # Major issues
        else:
            print("✅ PASSED: Производительность в пределах допустимого")
            return 0  # Success

def main():
    """Main entry point"""
    import argparse
    
    parser = argparse.ArgumentParser(description="MAGRAY CLI Performance Regression Detection")
    parser.add_argument("--results", default="benchmark-results.json", 
                       help="Path to current benchmark results")
    parser.add_argument("--baseline", default="benchmark-baseline.json",
                       help="Path to baseline benchmark results") 
    parser.add_argument("--strict", action="store_true",
                       help="Use strict thresholds (5% instead of 10%)")
    
    args = parser.parse_args()
    
    analyzer = PerformanceAnalyzer(args.results, args.baseline)
    
    if args.strict:
        analyzer.REGRESSION_THRESHOLDS = {
            'minor': 2.0,
            'major': 5.0, 
            'critical': 15.0
        }
        print("🔍 Using STRICT regression thresholds")
    
    return analyzer.run_analysis()

if __name__ == "__main__":
    sys.exit(main())