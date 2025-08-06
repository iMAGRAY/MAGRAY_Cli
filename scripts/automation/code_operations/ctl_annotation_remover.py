#!/usr/bin/env python3
"""
Скрипт для полного удаления всех CTL аннотаций из кодовой базы MAGRAY CLI.

НАЗНАЧЕНИЕ:
- Поиск всех CTL аннотаций в .rs файлах проекта
- Массовое удаление найденных аннотаций
- Создание backup перед изменениями
- Отчет об удаленных аннотациях
- Проверка компиляции после очистки

АВТОР: Claude Code AI Assistant
ВЕРСИЯ: 1.0.0
ЛИЦЕНЗИЯ: MIT
"""

import os
import re
import shutil
import subprocess
import sys
import json
import logging
from pathlib import Path
from datetime import datetime
from typing import List, Dict, Tuple, Optional
from dataclasses import dataclass, asdict
from concurrent.futures import ThreadPoolExecutor, as_completed
import argparse
from rich.console import Console
from rich.progress import Progress, TaskID, BarColumn, TextColumn, TimeRemainingColumn
from rich.table import Table
from rich.panel import Panel
from rich.text import Text

# Настройка логирования
logging.basicConfig(
    level=logging.INFO,
    format='%(asctime)s - %(levelname)s - %(message)s',
    handlers=[
        logging.FileHandler('ctl_cleanup.log'),
        logging.StreamHandler()
    ]
)

logger = logging.getLogger(__name__)
console = Console()

@dataclass
class CTLAnnotation:
    """Структура для хранения информации о CTL аннотации."""
    file_path: str
    line_number: int
    line_content: str
    annotation_type: str  # @component, @ctl4, CTL v3.0, etc.

@dataclass
class RemovalResult:
    """Результат удаления аннотаций из файла."""
    file_path: str
    removed_count: int
    original_lines: int
    final_lines: int
    annotations: List[CTLAnnotation]
    success: bool
    error: Optional[str] = None

@dataclass
class CleanupReport:
    """Отчет о полной очистке проекта."""
    total_files_processed: int
    total_annotations_removed: int
    files_with_annotations: int
    backup_location: str
    compilation_success: bool
    duration_seconds: float
    detailed_results: List[RemovalResult]

class CTLAnnotationRemover:
    """Основной класс для удаления CTL аннотаций."""
    
    # Паттерны для поиска CTL аннотаций
    CTL_PATTERNS = [
        r'^\s*//\s*@component:\s*\{.*\}\s*$',           # @component: {"k":"C",...}
        r'^\s*///\s*@component:\s*\{.*\}\s*$',          # /// @component: {...}
        r'^\s*//\s*@ctl4:\s*\{.*\}\s*$',                # @ctl4: {...}
        r'^\s*//\s*CTL\s+v3\.0.*$',                     # // CTL v3.0
        r'^\s*//\s*CTL\s+v2\.0.*$',                     # // CTL v2.0
        r'^\s*//\s*CTL\s+v\d+\.\d+.*$',                 # // CTL vX.Y
        r'^\s*///\s*CTL\s+v\d+\.\d+.*$',                # /// CTL vX.Y
        r'^\s*//.*@component.*:.*$',                     # любые строки с @component:
        r'^\s*///.*@component.*:.*$',                    # /// варианты @component:
        r'^\s*//.*Ⱦ\[.*\].*:=.*$',                      # тензорная нотация Ⱦ[...]
        r'^\s*//.*∇.*⊗.*⊕.*$',                          # тензорные операторы
    ]
    
    def __init__(self, project_root: str, dry_run: bool = False):
        """
        Инициализация ремовера.
        
        Args:
            project_root: Корневой путь к проекту
            dry_run: Если True, только показывает что будет удалено
        """
        self.project_root = Path(project_root).resolve()
        self.crates_path = self.project_root / "crates"
        self.dry_run = dry_run
        self.backup_dir: Optional[Path] = None
        
        # Проверяем существование директории crates
        if not self.crates_path.exists():
            raise FileNotFoundError(f"Директория crates не найдена: {self.crates_path}")
    
    def create_backup(self) -> Path:
        """Создает backup всех .rs файлов перед очисткой."""
        timestamp = datetime.now().strftime("%Y%m%d_%H%M%S")
        backup_name = f"ctl_cleanup_backup_{timestamp}"
        self.backup_dir = self.project_root / "backups" / backup_name
        
        console.print(f"[yellow]Создание backup в: {self.backup_dir}")
        
        # Создаем директорию backup
        self.backup_dir.mkdir(parents=True, exist_ok=True)
        
        # Копируем все .rs файлы сохраняя структуру
        rs_files = list(self.crates_path.rglob("*.rs"))
        
        with Progress(
            TextColumn("[progress.description]{task.description}"),
            BarColumn(),
            "[progress.percentage]{task.percentage:>3.0f}%",
            TimeRemainingColumn(),
        ) as progress:
            backup_task = progress.add_task("Backup файлов...", total=len(rs_files))
            
            for rs_file in rs_files:
                rel_path = rs_file.relative_to(self.crates_path)
                backup_file = self.backup_dir / "crates" / rel_path
                backup_file.parent.mkdir(parents=True, exist_ok=True)
                shutil.copy2(rs_file, backup_file)
                progress.advance(backup_task)
        
        console.print(f"[green]OK Backup создан: {len(rs_files)} файлов")
        return self.backup_dir
    
    def find_all_annotations(self) -> List[CTLAnnotation]:
        """Находит все CTL аннотации в проекте."""
        annotations = []
        rs_files = list(self.crates_path.rglob("*.rs"))
        
        console.print(f"[blue]Сканирование {len(rs_files)} .rs файлов...")
        
        with Progress(
            TextColumn("[progress.description]{task.description}"),
            BarColumn(),
            "[progress.percentage]{task.percentage:>3.0f}%",
            TimeRemainingColumn(),
        ) as progress:
            scan_task = progress.add_task("Поиск аннотаций...", total=len(rs_files))
            
            for rs_file in rs_files:
                try:
                    file_annotations = self._scan_file_for_annotations(rs_file)
                    annotations.extend(file_annotations)
                except Exception as e:
                    logger.error(f"Ошибка при сканировании {rs_file}: {e}")
                finally:
                    progress.advance(scan_task)
        
        console.print(f"[green]OK Найдено {len(annotations)} CTL аннотаций в {len(set(a.file_path for a in annotations))} файлах")
        return annotations
    
    def _scan_file_for_annotations(self, file_path: Path) -> List[CTLAnnotation]:
        """Сканирует файл на предмет CTL аннотаций."""
        annotations = []
        
        try:
            with open(file_path, 'r', encoding='utf-8') as f:
                lines = f.readlines()
            
            for line_num, line in enumerate(lines, 1):
                for pattern in self.CTL_PATTERNS:
                    if re.match(pattern, line, re.IGNORECASE):
                        annotation_type = self._detect_annotation_type(line)
                        annotations.append(CTLAnnotation(
                            file_path=str(file_path),
                            line_number=line_num,
                            line_content=line.strip(),
                            annotation_type=annotation_type
                        ))
                        break  # Одна аннотация на строку
        except Exception as e:
            logger.error(f"Ошибка при чтении {file_path}: {e}")
        
        return annotations
    
    def _detect_annotation_type(self, line: str) -> str:
        """Определяет тип CTL аннотации."""
        line_lower = line.lower().strip()
        
        if '@component:' in line_lower:
            return '@component'
        elif '@ctl4:' in line_lower:
            return '@ctl4'
        elif 'ctl v3.0' in line_lower:
            return 'CTL v3.0'
        elif 'ctl v2.0' in line_lower:
            return 'CTL v2.0'
        elif re.search(r'ctl\s+v\d+\.\d+', line_lower):
            return 'CTL version'
        elif 'ⱦ[' in line:
            return 'Tensor notation'
        elif any(op in line for op in ['∇', '⊗', '⊕', '⊙', '⊡']):
            return 'Tensor operators'
        else:
            return 'Unknown CTL'
    
    def remove_annotations_from_file(self, file_path: Path, annotations: List[CTLAnnotation]) -> RemovalResult:
        """Удаляет CTL аннотации из конкретного файла."""
        try:
            # Читаем файл
            with open(file_path, 'r', encoding='utf-8') as f:
                lines = f.readlines()
            
            original_line_count = len(lines)
            
            # Получаем номера строк для удаления (сортируем по убыванию)
            lines_to_remove = sorted([ann.line_number - 1 for ann in annotations], reverse=True)
            
            # Удаляем строки (с конца, чтобы не сбить нумерацию)
            for line_index in lines_to_remove:
                if 0 <= line_index < len(lines):
                    del lines[line_index]
            
            final_line_count = len(lines)
            removed_count = original_line_count - final_line_count
            
            if not self.dry_run:
                # Записываем обновленный файл
                with open(file_path, 'w', encoding='utf-8') as f:
                    f.writelines(lines)
            
            return RemovalResult(
                file_path=str(file_path),
                removed_count=removed_count,
                original_lines=original_line_count,
                final_lines=final_line_count,
                annotations=annotations,
                success=True
            )
            
        except Exception as e:
            logger.error(f"Ошибка при обработке файла {file_path}: {e}")
            return RemovalResult(
                file_path=str(file_path),
                removed_count=0,
                original_lines=0,
                final_lines=0,
                annotations=annotations,
                success=False,
                error=str(e)
            )
    
    def remove_all_annotations(self, annotations: List[CTLAnnotation]) -> List[RemovalResult]:
        """Удаляет все найденные аннотации."""
        # Группируем аннотации по файлам
        file_annotations = {}
        for annotation in annotations:
            if annotation.file_path not in file_annotations:
                file_annotations[annotation.file_path] = []
            file_annotations[annotation.file_path].append(annotation)
        
        results = []
        
        mode_text = "DRY RUN: Имитация удаления" if self.dry_run else "Удаление аннотаций"
        console.print(f"[red]{mode_text} из {len(file_annotations)} файлов...")
        
        with Progress(
            TextColumn("[progress.description]{task.description}"),
            BarColumn(),
            "[progress.percentage]{task.percentage:>3.0f}%",
            TimeRemainingColumn(),
        ) as progress:
            removal_task = progress.add_task(mode_text, total=len(file_annotations))
            
            # Параллельная обработка для ускорения
            with ThreadPoolExecutor(max_workers=4) as executor:
                future_to_file = {
                    executor.submit(self.remove_annotations_from_file, Path(file_path), anns): file_path
                    for file_path, anns in file_annotations.items()
                }
                
                for future in as_completed(future_to_file):
                    result = future.result()
                    results.append(result)
                    progress.advance(removal_task)
        
        successful_removals = [r for r in results if r.success]
        failed_removals = [r for r in results if not r.success]
        
        total_removed = sum(r.removed_count for r in successful_removals)
        
        if self.dry_run:
            console.print(f"[yellow]DRY RUN: Будет удалено {total_removed} строк из {len(successful_removals)} файлов")
        else:
            console.print(f"[green]OK Удалено {total_removed} строк из {len(successful_removals)} файлов")
        
        if failed_removals:
            console.print(f"[red]WARNING Ошибки при обработке {len(failed_removals)} файлов")
            for failed in failed_removals:
                console.print(f"  - {failed.file_path}: {failed.error}")
        
        return results
    
    def check_compilation(self) -> bool:
        """Проверяет компиляцию проекта после удаления аннотаций."""
        if self.dry_run:
            console.print("[yellow]DRY RUN: Пропуск проверки компиляции")
            return True
        
        console.print("[blue]Проверка компиляции проекта...")
        
        try:
            # Запускаем cargo check --workspace
            result = subprocess.run(
                ["cargo", "check", "--workspace"],
                cwd=self.project_root,
                capture_output=True,
                text=True,
                timeout=300  # 5 минут таймаут
            )
            
            if result.returncode == 0:
                console.print("[green]OK Компиляция успешна!")
                return True
            else:
                console.print("[red]ERROR Ошибка компиляции:")
                console.print(f"[red]{result.stderr}")
                return False
                
        except subprocess.TimeoutExpired:
            console.print("[red]ERROR Таймаут компиляции (5 минут)")
            return False
        except FileNotFoundError:
            console.print("[red]ERROR cargo не найден в PATH")
            return False
        except Exception as e:
            console.print(f"[red]ERROR Ошибка при проверке компиляции: {e}")
            return False
    
    def generate_report(self, results: List[RemovalResult], 
                       compilation_success: bool, 
                       duration: float) -> CleanupReport:
        """Генерирует итоговый отчет."""
        successful_results = [r for r in results if r.success]
        
        report = CleanupReport(
            total_files_processed=len(results),
            total_annotations_removed=sum(r.removed_count for r in successful_results),
            files_with_annotations=len([r for r in successful_results if r.removed_count > 0]),
            backup_location=str(self.backup_dir) if self.backup_dir else "",
            compilation_success=compilation_success,
            duration_seconds=duration,
            detailed_results=results
        )
        
        return report
    
    def display_report(self, report: CleanupReport):
        """Отображает итоговый отчет."""
        # Основная информация
        main_panel = Panel.fit(
            f"""[bold green]CTL CLEANUP REPORT[/bold green]
            
[blue]Обработано файлов:[/blue] {report.total_files_processed}
[blue]Файлов с аннотациями:[/blue] {report.files_with_annotations}
[blue]Всего удалено строк:[/blue] {report.total_annotations_removed}
[blue]Время выполнения:[/blue] {report.duration_seconds:.2f} секунд
[blue]Компиляция:[/blue] {'OK Успешно' if report.compilation_success else 'ERROR Ошибка'}
[blue]Backup:[/blue] {report.backup_location}""",
            title="Итоги очистки",
            border_style="green"
        )
        console.print(main_panel)
        
        # Детальная таблица по файлам
        if report.files_with_annotations > 0:
            table = Table(title="Детализация по файлам")
            table.add_column("Файл", style="blue")
            table.add_column("Строк удалено", justify="right", style="green")
            table.add_column("Было строк", justify="right")
            table.add_column("Стало строк", justify="right")
            table.add_column("Типы аннотаций", style="yellow")
            
            for result in report.detailed_results:
                if result.success and result.removed_count > 0:
                    relative_path = str(Path(result.file_path).relative_to(self.project_root))
                    annotation_types = ", ".join(set(ann.annotation_type for ann in result.annotations))
                    
                    table.add_row(
                        relative_path,
                        str(result.removed_count),
                        str(result.original_lines),
                        str(result.final_lines),
                        annotation_types
                    )
            
            console.print(table)
        
        # Статистика по типам аннотаций
        annotation_stats = {}
        for result in report.detailed_results:
            for ann in result.annotations:
                annotation_stats[ann.annotation_type] = annotation_stats.get(ann.annotation_type, 0) + 1
        
        if annotation_stats:
            stats_table = Table(title="Статистика по типам аннотаций")
            stats_table.add_column("Тип аннотации", style="blue")
            stats_table.add_column("Количество", justify="right", style="green")
            
            for ann_type, count in sorted(annotation_stats.items(), key=lambda x: x[1], reverse=True):
                stats_table.add_row(ann_type, str(count))
            
            console.print(stats_table)
    
    def save_report_to_file(self, report: CleanupReport, output_file: Optional[str] = None):
        """Сохраняет отчет в JSON файл."""
        if not output_file:
            timestamp = datetime.now().strftime("%Y%m%d_%H%M%S")
            output_file = f"ctl_cleanup_report_{timestamp}.json"
        
        report_dict = asdict(report)
        
        with open(output_file, 'w', encoding='utf-8') as f:
            json.dump(report_dict, f, indent=2, ensure_ascii=False)
        
        console.print(f"[green]OK Отчет сохранен в: {output_file}")
    
    def run_cleanup(self) -> CleanupReport:
        """Запускает полную процедуру очистки."""
        start_time = datetime.now()
        
        try:
            # 1. Создаем backup
            if not self.dry_run:
                self.create_backup()
            
            # 2. Ищем все аннотации
            annotations = self.find_all_annotations()
            
            if not annotations:
                console.print("[green]OK CTL аннотации не найдены!")
                return CleanupReport(
                    total_files_processed=0,
                    total_annotations_removed=0,
                    files_with_annotations=0,
                    backup_location=str(self.backup_dir) if self.backup_dir else "",
                    compilation_success=True,
                    duration_seconds=0.0,
                    detailed_results=[]
                )
            
            # 3. Удаляем аннотации
            results = self.remove_all_annotations(annotations)
            
            # 4. Проверяем компиляцию
            compilation_success = self.check_compilation()
            
            # 5. Генерируем отчет
            duration = (datetime.now() - start_time).total_seconds()
            report = self.generate_report(results, compilation_success, duration)
            
            return report
            
        except Exception as e:
            logger.error(f"Критическая ошибка при очистке: {e}")
            duration = (datetime.now() - start_time).total_seconds()
            return CleanupReport(
                total_files_processed=0,
                total_annotations_removed=0,
                files_with_annotations=0,
                backup_location=str(self.backup_dir) if self.backup_dir else "",
                compilation_success=False,
                duration_seconds=duration,
                detailed_results=[]
            )

def main():
    """Основная функция скрипта."""
    parser = argparse.ArgumentParser(
        description="Скрипт для удаления всех CTL аннотаций из кодовой базы MAGRAY CLI",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Примеры использования:
  py ctl_annotation_remover.py                    # Удаление всех аннотаций
  py ctl_annotation_remover.py --dry-run          # Предварительный просмотр
  py ctl_annotation_remover.py --project-root ..  # Указать путь к проекту
  py ctl_annotation_remover.py --report report.json  # Сохранить отчет в файл
        """
    )
    
    parser.add_argument(
        "--project-root",
        type=str,
        default="../../..",
        help="Путь к корневой директории проекта (по умолчанию: ../../..)"
    )
    
    parser.add_argument(
        "--dry-run",
        action="store_true",
        help="Режим предварительного просмотра (не изменяет файлы)"
    )
    
    parser.add_argument(
        "--report",
        type=str,
        help="Файл для сохранения JSON отчета"
    )
    
    parser.add_argument(
        "--no-backup",
        action="store_true",
        help="Не создавать backup (только для dry-run)"
    )
    
    args = parser.parse_args()
    
    # Валидация аргументов
    if args.no_backup and not args.dry_run:
        console.print("[red]Ошибка: --no-backup можно использовать только с --dry-run")
        sys.exit(1)
    
    try:
        # Инициализируем remover
        remover = CTLAnnotationRemover(
            project_root=args.project_root,
            dry_run=args.dry_run
        )
        
        # Показываем начальную информацию
        mode = "DRY RUN" if args.dry_run else "PRODUCTION"
        console.print(Panel.fit(
            f"""[bold blue]CTL ANNOTATION REMOVER v1.0.0[/bold blue]

[blue]Режим:[/blue] {mode}
[blue]Проект:[/blue] {remover.project_root}
[blue]Crates:[/blue] {remover.crates_path}
[blue]Backup:[/blue] {'Отключен' if args.dry_run or args.no_backup else 'Будет создан'}""",
            title="Конфигурация",
            border_style="blue"
        ))
        
        # Запускаем очистку
        report = remover.run_cleanup()
        
        # Отображаем отчет
        remover.display_report(report)
        
        # Сохраняем отчет в файл
        if args.report:
            remover.save_report_to_file(report, args.report)
        
        # Проверяем результат
        if not report.compilation_success and not args.dry_run:
            console.print("\n[red]WARNING ВНИМАНИЕ: Обнаружены ошибки компиляции!")
            console.print("Рекомендуется восстановить из backup и исправить проблемы.")
            sys.exit(1)
        
        if args.dry_run and report.total_annotations_removed > 0:
            console.print("\n[yellow]Для реального удаления запустите без --dry-run")
        
        console.print("\n[green]OK Очистка завершена успешно!")
        
    except KeyboardInterrupt:
        console.print("\n[yellow]Прервано пользователем")
        sys.exit(130)
    except Exception as e:
        console.print(f"\n[red]ERROR Критическая ошибка: {e}")
        logger.exception("Необработанная ошибка")
        sys.exit(1)

if __name__ == "__main__":
    main()