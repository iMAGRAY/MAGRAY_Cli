#!/usr/bin/env python3
"""
Базовые unit-тесты для architecture_daemon.py
Включают только самые критичные тесты без внешних зависимостей
"""

import unittest
import tempfile
from pathlib import Path
from unittest.mock import patch, MagicMock

# Импортируем классы из нашего модуля
import sys
import os
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

from architecture_daemon import SecurityUtils, ResourceLimiter, OptimizedRegexPatterns, SecurityError

class TestSecurityUtils(unittest.TestCase):
    """Тесты для SecurityUtils"""
    
    def setUp(self):
        self.temp_dir = tempfile.mkdtemp()
        self.temp_path = Path(self.temp_dir)
    
    def test_validate_project_path_valid(self):
        """Тест валидации корректного пути"""
        # Создаем Cargo.toml для теста
        cargo_toml = self.temp_path / "Cargo.toml"
        with open(cargo_toml, 'w') as f:
            f.write('[package]\nname = "test"\nversion = "0.1.0"\n')
        
        try:
            result = SecurityUtils.validate_project_path(str(self.temp_path))
            self.assertEqual(result, self.temp_path.resolve())
        except SecurityError:
            self.fail("Валидный путь не должен вызывать SecurityError")
    
    def test_validate_project_path_dangerous_chars(self):
        """Тест блокировки опасных символов"""
        dangerous_paths = [
            "../../../etc/passwd",
            "path/with/../traversal",
            "path\\with\\..\\traversal",
            "path|with|pipe"
        ]
        
        for path in dangerous_paths:
            with self.assertRaises(SecurityError):
                SecurityUtils.validate_project_path(path)
    
    def test_safe_read_file_size_limit(self):
        """Тест ограничения размера файла"""
        # Создаем файл больше лимита
        large_file = self.temp_path / "large.txt"
        with open(large_file, 'w') as f:
            f.write("x" * (SecurityUtils.MAX_FILE_SIZE + 1))
        
        with self.assertRaises(SecurityError):
            SecurityUtils.safe_read_file(large_file)
    
    def test_safe_read_file_valid(self):
        """Тест чтения корректного файла"""
        test_file = self.temp_path / "test.rs"
        test_content = "fn main() { println!(\"Hello\"); }"
        with open(test_file, 'w') as f:
            f.write(test_content)
        
        result = SecurityUtils.safe_read_file(test_file)
        self.assertEqual(result, test_content)
    
    def test_safe_write_file_size_limit(self):
        """Тест ограничения размера при записи"""
        target_file = self.temp_path / "output.md"
        large_content = "x" * (SecurityUtils.MAX_FILE_SIZE + 1)
        
        with self.assertRaises(SecurityError):
            SecurityUtils.safe_write_file(target_file, large_content)
    
    def tearDown(self):
        """Очистка временных файлов"""
        import shutil
        shutil.rmtree(self.temp_dir, ignore_errors=True)


class TestResourceLimiter(unittest.TestCase):
    """Тесты для ResourceLimiter"""
    
    def setUp(self):
        self.limiter = ResourceLimiter()
    
    def test_file_limit_check(self):
        """Тест лимита файлов"""
        # Симулируем превышение лимита
        self.limiter.file_count = ResourceLimiter.MAX_FILE_COUNT + 1
        
        with self.assertRaises(SecurityError):
            self.limiter.check_file_limit()
    
    def test_content_size_limit(self):
        """Тест лимита размера контента"""
        large_content = "x" * (ResourceLimiter.MAX_CONTENT_SIZE + 1)
        
        with self.assertRaises(SecurityError):
            self.limiter.check_content_size(large_content)
    
    def test_time_limit_check(self):
        """Тест лимита времени"""
        import time
        # Симулируем превышение времени
        self.limiter.start_time = time.time() - ResourceLimiter.MAX_ANALYSIS_TIME - 1
        
        with self.assertRaises(SecurityError):
            self.limiter.check_time_limit()
    
    @patch('psutil.Process')
    def test_memory_check_with_psutil(self, mock_process):
        """Тест проверки памяти с psutil"""
        # Мокаем превышение памяти
        mock_instance = MagicMock()
        mock_instance.memory_info.return_value.rss = (ResourceLimiter.MAX_MEMORY_MB + 100) * 1024 * 1024
        mock_process.return_value = mock_instance
        
        # Устанавливаем счетчик для срабатывания проверки
        self.limiter.memory_check_counter = 49
        
        with self.assertRaises(SecurityError):
            self.limiter.check_memory_usage()
    
    def test_memory_check_without_psutil(self):
        """Тест проверки памяти без psutil (не должно падать)"""
        # Симулируем отсутствие psutil
        with patch('psutil.Process', side_effect=ImportError):
            self.limiter.memory_check_counter = 49
            # Не должно вызывать исключение
            self.limiter.check_memory_usage()


class TestOptimizedRegexPatterns(unittest.TestCase):
    """Тесты для оптимизированных regex паттернов"""
    
    def setUp(self):
        self.patterns = OptimizedRegexPatterns()
        self.rust_code = '''
        pub struct TestStruct {
            field: i32,
        }
        
        pub enum TestEnum {
            Variant1,
            Variant2,
        }
        
        pub trait TestTrait {
            fn test_method(&self);
        }
        
        pub fn test_function() -> i32 {
            42
        }
        
        pub async fn async_test() {}
        
        #[test]
        fn test_example() {
            assert_eq!(1, 1);
        }
        
        impl MockTestStruct for TestStruct {
            fn mock_method(&self) {}
        }
        
        unsafe {
            // Some unsafe code
        }
        '''
    
    def test_find_structs(self):
        """Тест поиска структур"""
        structs = self.patterns.find_structs(self.rust_code)
        self.assertIn('TestStruct', structs)
    
    def test_find_enums(self):
        """Тест поиска перечислений"""
        enums = self.patterns.find_enums(self.rust_code)
        self.assertIn('TestEnum', enums)
    
    def test_find_traits(self):
        """Тест поиска трейтов"""
        traits = self.patterns.find_traits(self.rust_code)
        self.assertIn('TestTrait', traits)
    
    def test_find_functions(self):
        """Тест поиска функций"""
        functions = self.patterns.find_functions(self.rust_code)
        self.assertIn('test_function', functions)
        self.assertIn('async_test', functions)
    
    def test_find_tests(self):
        """Тест поиска тестовых функций"""
        tests = self.patterns.find_tests(self.rust_code)
        self.assertIn('test_example', tests)
    
    def test_find_mocks(self):
        """Тест поиска моков"""
        mocks = self.patterns.find_mocks(self.rust_code)
        self.assertIn('MockTestStruct', mocks)
    
    def test_calculate_complexity(self):
        """Тест расчета сложности"""
        complex_code = '''
        fn complex_function() {
            if condition {
                for item in items {
                    while running {
                        match value {
                            Some(x) => loop { break; },
                            None => {}
                        }
                    }
                }
            }
        }
        '''
        complexity = self.patterns.calculate_complexity(complex_code)
        # 1 (базовая) + 1 (if) + 1 (for) + 1 (while) + 1 (match) + 1 (loop) = 6
        self.assertGreaterEqual(complexity, 6)
    
    def test_find_code_smells(self):
        """Тест поиска проблем в коде"""
        smells = self.patterns.find_code_smells(self.rust_code)
        self.assertGreater(smells['unsafe_blocks'], 0)
        
        # Тестируем код с проблемами
        bad_code = '''
        fn bad_function() {
            let x = vec![1, 2, 3];
            let y = x.clone();
            let result = some_result.unwrap();
        }
        '''
        smells = self.patterns.find_code_smells(bad_code)
        self.assertGreater(smells['clones'], 0)
        self.assertGreater(smells['unwraps'], 0)


class TestPerformance(unittest.TestCase):
    """Базовые тесты производительности"""
    
    def test_regex_compilation_speed(self):
        """Тест скорости компиляции regex"""
        import time
        
        start_time = time.time()
        patterns = OptimizedRegexPatterns()
        compilation_time = time.time() - start_time
        
        # Компиляция всех паттернов должна занимать менее 100мс
        self.assertLess(compilation_time, 0.1)
    
    def test_pattern_matching_speed(self):
        """Тест скорости сопоставления паттернов"""
        import time
        
        patterns = OptimizedRegexPatterns()
        large_rust_code = '''
        pub struct TestStruct {}
        pub fn test_function() {}
        ''' * 1000  # Большой файл с повторениями
        
        start_time = time.time()
        structs = patterns.find_structs(large_rust_code)
        functions = patterns.find_functions(large_rust_code)
        matching_time = time.time() - start_time
        
        # Поиск должен быть быстрым даже для больших файлов
        self.assertLess(matching_time, 0.5)
        self.assertEqual(len(structs), 1000)
        self.assertEqual(len(functions), 1000)


class TestDuplicatesAnalysis(unittest.TestCase):
    """Тесты для анализа дубликатов"""
    
    def setUp(self):
        # Создаем mock объект с нужными методами для тестирования
        from unittest.mock import Mock
        
        self.daemon = Mock()
        
        # Добавляем тестовые дубликаты
        self.daemon.duplicates = {
            'impl Into': [
                ('memory/api.rs', 'Into'),
                ('memory/errors.rs', 'Into'),  
                ('common/types.rs', 'Into')
            ],
            'impl Default': [
                ('common/config.rs', 'Default'),
                ('memory/cache.rs', 'Default')
            ],
            'fn new': [
                ('ai/models.rs', 'new'),
                ('memory/storage.rs', 'new'),
                ('cli/commands.rs', 'new'),
                ('tools/manager.rs', 'new'),
                ('domain/service.rs', 'new')
            ]
        }
        
        # Импортируем и привязываем методы из реального класса
        try:
            from architecture_daemon import ArchitectureDaemon
            self.daemon._analyze_duplicates = ArchitectureDaemon._analyze_duplicates.__get__(self.daemon)
            self.daemon._generate_duplicates_report = ArchitectureDaemon._generate_duplicates_report.__get__(self.daemon)
        except Exception as e:
            self.skipTest(f"Не удалось импортировать методы: {e}")
    
    def test_generate_duplicates_report_empty(self):
        """Тест генерации отчета при отсутствии дубликатов"""
        if not self.daemon:
            self.skipTest("ArchitectureDaemon не инициализирован")
            
        self.daemon.duplicates = {}
        report = self.daemon._generate_duplicates_report()
        self.assertEqual(report, "")
    
    def test_generate_duplicates_report_with_data(self):
        """Тест генерации отчета с данными о дубликатах"""
        if not self.daemon:
            self.skipTest("ArchitectureDaemon не инициализирован")
            
        report = self.daemon._generate_duplicates_report()
        
        # Проверяем наличие ключевых элементов в отчете
        self.assertIn("### 🔄 ДЕТАЛЬНЫЙ АНАЛИЗ ДУБЛИКАТОВ", report)
        self.assertIn("impl Into", report)
        self.assertIn("fn new", report)
        self.assertIn("api.rs", report)  # Файл показывается в сокращенном виде
        
        # Проверяем что дубликаты отсортированы по количеству (fn new = 5 копий должно быть первым)
        lines = report.split('\n')
        first_duplicate_line = next((line for line in lines if line.startswith('**1.')), None)
        self.assertIsNotNone(first_duplicate_line)
        self.assertIn("5 копий", first_duplicate_line)
    
    def test_generate_duplicates_report_grouping(self):
        """Тест группировки файлов по crate"""
        if not self.daemon:
            self.skipTest("ArchitectureDaemon не инициализирован")
            
        report = self.daemon._generate_duplicates_report()
        
        # Проверяем что файлы группируются по crate (для memory где есть несколько файлов)
        self.assertIn("**memory**:", report)
        
        # Проверяем что в группах показываются только имена файлов
        self.assertIn("api.rs", report)
        self.assertIn("errors.rs", report)
    
    def test_analyze_duplicates_method(self):
        """Тест базового метода _analyze_duplicates"""
        if not self.daemon:
            self.skipTest("ArchitectureDaemon не инициализирован")
            
        duplicates = self.daemon._analyze_duplicates()
        
        # Проверяем что возвращаются только реальные дубликаты (>1 копии)
        self.assertIn('impl Into', duplicates)
        self.assertIn('fn new', duplicates)
        
        # impl Default имеет только 2 копии, поэтому должен быть включен
        self.assertIn('impl Default', duplicates)
        
        # Проверяем количество копий
        self.assertEqual(len(duplicates['impl Into']), 3)
        self.assertEqual(len(duplicates['fn new']), 5)


if __name__ == '__main__':
    # Запуск тестов с подробным выводом
    unittest.main(verbosity=2, buffer=True)