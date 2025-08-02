#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
Скрипт для сравнения качества двух ONNX моделей BGE
Сравнивает:
1. bge-code-v1-int8/model.onnx (квантизированная)
2. 1111/model.onnx (оригинальная)
"""

import onnxruntime as ort
import numpy as np
from transformers import AutoTokenizer
from pathlib import Path
import time
import json
import os
from tabulate import tabulate
from sklearn.metrics.pairwise import cosine_similarity
import logging

# Настройка логирования
logging.basicConfig(level=logging.INFO, format='%(asctime)s - %(levelname)s - %(message)s')
logger = logging.getLogger(__name__)

class ModelComparator:
    def __init__(self, model1_path, model2_path, tokenizer_path=None):
        """
        Инициализация компаратора моделей
        
        Args:
            model1_path: Путь к первой модели (квантизированная int8)
            model2_path: Путь ко второй модели (оригинальная)
            tokenizer_path: Путь к токенизатору (по умолчанию используется huggingface)
        """
        self.model1_path = Path(model1_path)
        self.model2_path = Path(model2_path)
        
        # Проверяем существование моделей
        if not self.model1_path.exists():
            raise FileNotFoundError(f"Модель 1 не найдена: {self.model1_path}")
        if not self.model2_path.exists():
            raise FileNotFoundError(f"Модель 2 не найдена: {self.model2_path}")
            
        logger.info(f"Модель 1 (INT8): {self.model1_path}")
        logger.info(f"Модель 2 (FP32): {self.model2_path}")
        
        # Загружаем токенизатор
        self.tokenizer = self._load_tokenizer(tokenizer_path)
        
        # Провайдеры исполнения
        self.providers = ['CPUExecutionProvider']
        
        # Тестовые тексты для сравнения
        self.test_texts = [
            "def fibonacci(n): return n if n <= 1 else fibonacci(n-1) + fibonacci(n-2)",
            "class DataProcessor: def __init__(self): self.data = []",
            "import numpy as np; arr = np.array([1, 2, 3, 4, 5])",
            "SELECT * FROM users WHERE age > 25 AND status = 'active'",
            "function quickSort(arr) { if (arr.length <= 1) return arr; }",
            "public class Calculator { private int result; public int add(int a, int b) { return a + b; } }",
            "package main; import fmt; func main() { fmt.Println('Hello, World!') }",
            "async def fetch_data(url): async with aiohttp.ClientSession() as session: return await session.get(url)",
            "const express = require('express'); const app = express(); app.listen(3000);",
            "struct Point { x: f64, y: f64 } impl Point { fn new(x: f64, y: f64) -> Self { Point { x, y } } }"
        ]

    def _load_tokenizer(self, tokenizer_path):
        """Загружает токенизатор"""
        try:
            if tokenizer_path and Path(tokenizer_path).exists():
                tokenizer = AutoTokenizer.from_pretrained(tokenizer_path)
            else:
                # Пытаемся найти токенизатор в папке с моделями
                possible_paths = [
                    self.model2_path.parent,  # В папке со второй моделью
                    self.model1_path.parent,  # В папке с первой моделью
                    Path(__file__).parent / "bge-code-v1"
                ]
                
                tokenizer = None
                for path in possible_paths:
                    try:
                        if (path / "tokenizer.json").exists():
                            tokenizer = AutoTokenizer.from_pretrained(str(path))
                            logger.info(f"Токенизатор загружен из: {path}")
                            break
                    except:
                        continue
                
                if not tokenizer:
                    # Fallback на BGE-M3 токенизатор
                    tokenizer = AutoTokenizer.from_pretrained("BAAI/bge-m3")
                    logger.warning("Используется fallback токенизатор BAAI/bge-m3")
                    
            return tokenizer
        except Exception as e:
            logger.error(f"Ошибка загрузки токенизатора: {e}")
            raise

    def get_model_info(self, model_path):
        """Получает информацию о модели"""
        try:
            session = ort.InferenceSession(str(model_path), providers=self.providers)
            
            # Размер файла
            file_size = model_path.stat().st_size / (1024 * 1024)  # MB
            
            # Информация о входах и выходах
            inputs_info = []
            for inp in session.get_inputs():
                inputs_info.append({
                    'name': inp.name,
                    'type': inp.type,
                    'shape': inp.shape
                })
            
            outputs_info = []
            for out in session.get_outputs():
                outputs_info.append({
                    'name': out.name,
                    'type': out.type,
                    'shape': out.shape
                })
            
            # Попытка определить количество параметров
            try:
                # Простая оценка на основе размера файла
                estimated_params = int(file_size * 1024 * 1024 / 4)  # Примерно 4 байта на параметр
            except:
                estimated_params = "N/A"
            
            return {
                'file_size_mb': round(file_size, 2),
                'estimated_params': estimated_params,
                'inputs': inputs_info,
                'outputs': outputs_info,
                'providers': session.get_providers()
            }
        except Exception as e:
            logger.error(f"Ошибка получения информации о модели {model_path}: {e}")
            return None

    def benchmark_model(self, model_path, warmup_runs=3, test_runs=10):
        """Бенчмарк производительности модели"""
        try:
            session = ort.InferenceSession(str(model_path), providers=self.providers)
            
            # Подготовка тестового входа
            test_text = self.test_texts[0]
            encoded = self.tokenizer(
                test_text, 
                padding='max_length', 
                truncation=True, 
                max_length=512, 
                return_tensors='np'
            )
            
            inputs = {
                'input_ids': encoded['input_ids'].astype(np.int64),
                'attention_mask': encoded['attention_mask'].astype(np.int64)
            }
            
            # Прогрев
            logger.info(f"Прогрев модели {model_path.name}...")
            for _ in range(warmup_runs):
                session.run(None, inputs)
            
            # Измерение времени
            logger.info(f"Бенчмарк модели {model_path.name}...")
            times = []
            for i in range(test_runs):
                start_time = time.perf_counter()
                outputs = session.run(None, inputs)
                end_time = time.perf_counter()
                times.append((end_time - start_time) * 1000)  # в миллисекундах
                
                if i == 0:
                    # Сохраняем первый результат
                    first_output = outputs
            
            return {
                'times_ms': times,
                'avg_time_ms': np.mean(times),
                'min_time_ms': np.min(times),
                'max_time_ms': np.max(times),
                'std_time_ms': np.std(times),
                'first_output': first_output
            }
            
        except Exception as e:
            logger.error(f"Ошибка бенчмарка модели {model_path}: {e}")
            return None

    def get_embeddings(self, model_path, texts):
        """Получает embeddings для списка текстов"""
        try:
            session = ort.InferenceSession(str(model_path), providers=self.providers)
            embeddings = []
            
            for text in texts:
                encoded = self.tokenizer(
                    text, 
                    padding='max_length', 
                    truncation=True, 
                    max_length=512, 
                    return_tensors='np'
                )
                
                inputs = {
                    'input_ids': encoded['input_ids'].astype(np.int64),
                    'attention_mask': encoded['attention_mask'].astype(np.int64)
                }
                
                outputs = session.run(None, inputs)
                
                # Извлекаем embedding (обычно это первый или последний выход)
                if len(outputs) == 1:
                    embedding = outputs[0][0]  # Первый элемент батча
                else:
                    # Если несколько выходов, берем последний (обычно pooled output)
                    embedding = outputs[-1][0]
                
                embeddings.append(embedding)
            
            return np.array(embeddings)
            
        except Exception as e:
            logger.error(f"Ошибка получения embeddings для {model_path}: {e}")
            return None

    def compare_embeddings_quality(self, embeddings1, embeddings2):
        """Сравнивает качество embeddings между двумя моделями"""
        try:
            similarities = []
            
            for i in range(len(embeddings1)):
                # Косинусное сходство между embeddings одного текста от разных моделей
                similarity = cosine_similarity(
                    embeddings1[i].reshape(1, -1), 
                    embeddings2[i].reshape(1, -1)
                )[0, 0]
                similarities.append(similarity)
            
            # Вычисляем сходство между разными текстами внутри каждой модели
            sim_matrix1 = cosine_similarity(embeddings1)
            sim_matrix2 = cosine_similarity(embeddings2)
            
            # Средняя корреляция между матрицами сходства
            sim_correlation = np.corrcoef(sim_matrix1.flatten(), sim_matrix2.flatten())[0, 1]
            
            return {
                'individual_similarities': similarities,
                'avg_similarity': np.mean(similarities),
                'min_similarity': np.min(similarities),
                'max_similarity': np.max(similarities),
                'std_similarity': np.std(similarities),
                'similarity_matrix_correlation': sim_correlation
            }
            
        except Exception as e:
            logger.error(f"Ошибка сравнения embeddings: {e}")
            return None

    def run_comparison(self):
        """Запускает полное сравнение моделей"""
        logger.info("Начинаем сравнение моделей...")
        
        results = {
            'model1_path': str(self.model1_path),
            'model2_path': str(self.model2_path),
            'model1_info': None,
            'model2_info': None,
            'model1_benchmark': None,
            'model2_benchmark': None,
            'embeddings_comparison': None
        }
        
        # 1. Получаем информацию о моделях
        logger.info("Получение информации о моделях...")
        results['model1_info'] = self.get_model_info(self.model1_path)
        results['model2_info'] = self.get_model_info(self.model2_path)
        
        # 2. Бенчмарк производительности
        logger.info("Бенчмарк производительности...")
        results['model1_benchmark'] = self.benchmark_model(self.model1_path)
        results['model2_benchmark'] = self.benchmark_model(self.model2_path)
        
        # 3. Сравнение качества embeddings
        logger.info("Сравнение качества embeddings...")
        embeddings1 = self.get_embeddings(self.model1_path, self.test_texts)
        embeddings2 = self.get_embeddings(self.model2_path, self.test_texts)
        
        if embeddings1 is not None and embeddings2 is not None:
            results['embeddings_comparison'] = self.compare_embeddings_quality(embeddings1, embeddings2)
        
        return results

    def print_comparison_table(self, results):
        """Выводит результаты сравнения в виде таблицы"""
        print("\n" + "="*80)
        print("СРАВНЕНИЕ ONNX МОДЕЛЕЙ BGE")
        print("="*80)
        
        # Общая информация
        print("\n[INFO] ОБЩАЯ ИНФОРМАЦИЯ:")
        model1_name = "BGE-Code-v1-INT8 (Квантизированная)"
        model2_name = "BGE-Code-v1-FP32 (Оригинальная)"
        
        general_data = [
            ["Параметр", model1_name, model2_name],
            ["Путь к модели", results['model1_path'].split('\\')[-2] + "\\model.onnx", results['model2_path'].split('\\')[-2] + "\\model.onnx"],
        ]
        
        # Добавляем информацию о размерах файлов
        if results['model1_info'] and results['model2_info']:
            general_data.extend([
                ["Размер файла (MB)", 
                 f"{results['model1_info']['file_size_mb']:.2f}", 
                 f"{results['model2_info']['file_size_mb']:.2f}"],
                ["Сжатие размера", 
                 f"{results['model1_info']['file_size_mb'] / results['model2_info']['file_size_mb']:.2f}x",
                 "1.00x (базовая)"],
            ])
        
        print(tabulate(general_data, headers="firstrow", tablefmt="grid"))
        
        # Производительность
        if results['model1_benchmark'] and results['model2_benchmark']:
            print("\n[PERF] ПРОИЗВОДИТЕЛЬНОСТЬ (INFERENCE):")
            perf_data = [
                ["Метрика", model1_name, model2_name, "Преимущество"],
                ["Среднее время (мс)", 
                 f"{results['model1_benchmark']['avg_time_ms']:.2f}", 
                 f"{results['model2_benchmark']['avg_time_ms']:.2f}",
                 f"{results['model2_benchmark']['avg_time_ms'] / results['model1_benchmark']['avg_time_ms']:.2f}x быстрее" if results['model1_benchmark']['avg_time_ms'] < results['model2_benchmark']['avg_time_ms'] else f"{results['model1_benchmark']['avg_time_ms'] / results['model2_benchmark']['avg_time_ms']:.2f}x медленнее"],
                ["Мин. время (мс)", 
                 f"{results['model1_benchmark']['min_time_ms']:.2f}", 
                 f"{results['model2_benchmark']['min_time_ms']:.2f}",
                 ""],
                ["Макс. время (мс)", 
                 f"{results['model1_benchmark']['max_time_ms']:.2f}", 
                 f"{results['model2_benchmark']['max_time_ms']:.2f}",
                 ""],
                ["Стд. отклонение (мс)", 
                 f"{results['model1_benchmark']['std_time_ms']:.2f}", 
                 f"{results['model2_benchmark']['std_time_ms']:.2f}",
                 ""],
            ]
            print(tabulate(perf_data, headers="firstrow", tablefmt="grid"))
        
        # Качество embeddings
        if results['embeddings_comparison']:
            print("\n[QUALITY] КАЧЕСТВО EMBEDDINGS:")
            quality_data = [
                ["Метрика", "Значение", "Интерпретация"],
                ["Среднее косинусное сходство", 
                 f"{results['embeddings_comparison']['avg_similarity']:.4f}",
                 "Близость к 1.0 = высокое качество"],
                ["Мин. сходство", 
                 f"{results['embeddings_comparison']['min_similarity']:.4f}",
                 "Худший случай"],
                ["Макс. сходство", 
                 f"{results['embeddings_comparison']['max_similarity']:.4f}",
                 "Лучший случай"],
                ["Стд. отклонение сходства", 
                 f"{results['embeddings_comparison']['std_similarity']:.4f}",
                 "Стабильность качества"],
                ["Корреляция матриц сходства", 
                 f"{results['embeddings_comparison']['similarity_matrix_correlation']:.4f}",
                 "Сохранение семантических отношений"],
            ]
            print(tabulate(quality_data, headers="firstrow", tablefmt="grid"))
            
            # Детальный анализ качества
            avg_sim = results['embeddings_comparison']['avg_similarity']
            if avg_sim > 0.95:
                quality_verdict = "[EXCELLENT] ОТЛИЧНОЕ - Квантизация практически не влияет на качество"
            elif avg_sim > 0.90:
                quality_verdict = "[GOOD] ХОРОШЕЕ - Небольшая потеря качества, приемлемо для продакшена"
            elif avg_sim > 0.80:
                quality_verdict = "[OK] УДОВЛЕТВОРИТЕЛЬНОЕ - Заметная потеря качества"
            else:
                quality_verdict = "[BAD] ПЛОХОЕ - Значительная деградация качества"
            
            print(f"\n[VERDICT] ВЕРДИКТ ПО КАЧЕСТВУ: {quality_verdict}")
        
        # Итоговое сравнение
        print("\n[SUMMARY] ИТОГОВОЕ СРАВНЕНИЕ:")
        
        # Вычисляем общие метрики
        size_reduction = results['model1_info']['file_size_mb'] / results['model2_info']['file_size_mb'] if results['model1_info'] and results['model2_info'] else 0
        speed_ratio = results['model1_benchmark']['avg_time_ms'] / results['model2_benchmark']['avg_time_ms'] if results['model1_benchmark'] and results['model2_benchmark'] else 1
        quality_score = results['embeddings_comparison']['avg_similarity'] if results['embeddings_comparison'] else 0
        
        summary_data = [
            ["Критерий", "INT8 модель", "FP32 модель", "Победитель"],
            ["Размер файла", f"[+] {size_reduction:.2f}x меньше", "[-] Больше", "INT8"],
            ["Скорость inference", 
             "[+] Быстрее" if speed_ratio < 1 else "[-] Медленнее", 
             "[-] Медленнее" if speed_ratio < 1 else "[+] Быстрее",
             "INT8" if speed_ratio < 1 else "FP32"],
            ["Качество embeddings", f"[!] {quality_score:.1%} от оригинала", "[+] 100% (эталон)", "FP32"],
            ["Потребление памяти", "[+] Меньше", "[-] Больше", "INT8"],
        ]
        
        print(tabulate(summary_data, headers="firstrow", tablefmt="grid"))
        
        # Рекомендации
        print("\n[RECOMMENDATIONS] РЕКОМЕНДАЦИИ:")
        if quality_score > 0.90 and size_reduction < 0.5:
            print("[RECOMMEND] РЕКОМЕНДУЕТСЯ использовать INT8 модель для продакшена:")
            print("   • Значительное сжатие размера файла")
            print("   • Сохранение высокого качества embeddings")
            print("   • Более быстрый inference (вероятно)")
        elif quality_score > 0.80:
            print("[CHOICE] Выбор зависит от приоритетов:")
            print("   • INT8: для экономии ресурсов при допустимой потере качества")
            print("   • FP32: для максимального качества без ограничений по ресурсам")
        else:
            print("[WARNING] Рассмотрите улучшение процесса квантизации:")
            print("   • Качество INT8 модели может быть неприемлемым")
            print("   • Используйте FP32 модель или попробуйте другие методы квантизации")

def main():
    """Основная функция"""
    base_path = Path(__file__).parent
    model1_path = base_path / "bge-code-v1-int8" / "model.onnx"
    model2_path = base_path / "1111" / "model.onnx"
    
    try:
        # Создаем компаратор
        comparator = ModelComparator(model1_path, model2_path)
        
        # Запускаем сравнение
        results = comparator.run_comparison()
        
        # Выводим результаты
        comparator.print_comparison_table(results)
        
        # Сохраняем результаты в JSON
        output_file = Path(__file__).parent / "model_comparison_results.json"
        with open(output_file, 'w', encoding='utf-8') as f:
            # Преобразуем numpy arrays в списки для JSON
            json_results = results.copy()
            if json_results['embeddings_comparison']:
                json_results['embeddings_comparison']['individual_similarities'] = [
                    float(x) for x in json_results['embeddings_comparison']['individual_similarities']
                ]
            
            json.dump(json_results, f, indent=2, ensure_ascii=False, default=str)
        
        logger.info(f"Результаты сохранены в: {output_file}")
        
    except Exception as e:
        logger.error(f"Ошибка выполнения сравнения: {e}")
        raise

if __name__ == "__main__":
    main()