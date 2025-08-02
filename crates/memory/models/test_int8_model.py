#!/usr/bin/env python3
"""
Полная проверка BGE-code-v1 INT8 модели
"""

import os
import json
import time
import numpy as np
import onnxruntime as ort
from transformers import AutoTokenizer
from sklearn.metrics.pairwise import cosine_similarity
from pathlib import Path
import sys

class ModelTester:
    def __init__(self, model_dir):
        self.model_dir = Path(model_dir)
        self.results = {
            "status": "testing",
            "model_path": str(model_dir),
            "errors": [],
            "warnings": [],
            "metrics": {}
        }
        
    def log(self, message, level="INFO"):
        timestamp = time.strftime("%H:%M:%S")
        print(f"[{timestamp}] {level}: {message}")
        
    def check_files(self):
        """Проверка наличия всех необходимых файлов"""
        self.log("Проверка файлов...")
        
        required_files = {
            "model.onnx": "Основная модель",
            "tokenizer.json": "Токенизатор",
            "config.json": "Конфигурация модели",
            "tokenizer_config.json": "Конфигурация токенизатора"
        }
        
        missing_files = []
        for file, desc in required_files.items():
            file_path = self.model_dir / file
            if file_path.exists():
                size_mb = file_path.stat().st_size / (1024 * 1024)
                self.log(f"OK {file} ({desc}): {size_mb:.2f} MB")
                self.results["metrics"][f"file_size_{file}"] = size_mb
            else:
                missing_files.append(file)
                self.log(f"MISSING {file} ({desc}): NOT FOUND", "ERROR")
                
        if missing_files:
            self.results["errors"].append(f"Missing files: {missing_files}")
            return False
        return True
        
    def load_model(self):
        """Загрузка ONNX модели"""
        self.log("Загрузка модели...")
        
        try:
            model_path = str(self.model_dir / "model.onnx")
            
            # Проверяем провайдеры
            providers = ort.get_available_providers()
            self.log(f"Доступные провайдеры: {providers}")
            
            # Загружаем модель
            start_time = time.time()
            self.session = ort.InferenceSession(
                model_path,
                providers=['CPUExecutionProvider']
            )
            load_time = (time.time() - start_time) * 1000
            
            self.log(f"OK Model loaded in {load_time:.2f} ms")
            self.results["metrics"]["model_load_time_ms"] = load_time
            
            # Получаем информацию о модели
            inputs = self.session.get_inputs()
            outputs = self.session.get_outputs()
            
            self.log(f"Входы модели:")
            for inp in inputs:
                self.log(f"  - {inp.name}: {inp.shape} ({inp.type})")
                
            self.log(f"Выходы модели:")
            for i, out in enumerate(outputs):
                self.log(f"  - [{i}] {out.name}: {out.shape} ({out.type})")
                
            self.results["metrics"]["input_count"] = len(inputs)
            self.results["metrics"]["output_count"] = len(outputs)
            
            return True
            
        except Exception as e:
            self.log(f"ERROR Loading model: {e}", "ERROR")
            self.results["errors"].append(f"Model load error: {str(e)}")
            return False
            
    def load_tokenizer(self):
        """Загрузка токенизатора"""
        self.log("Загрузка токенизатора...")
        
        try:
            # Пробуем загрузить через transformers
            self.tokenizer = AutoTokenizer.from_pretrained(
                str(self.model_dir),
                trust_remote_code=True
            )
            self.log("OK Tokenizer loaded")
            
            # Тестируем токенизацию
            test_text = "def hello_world():"
            tokens = self.tokenizer(test_text, return_tensors="np")
            self.log(f"Тест токенизации: '{test_text}' -> {tokens['input_ids'].shape[1]} токенов")
            
            return True
            
        except Exception as e:
            self.log(f"ERROR Loading tokenizer: {e}", "ERROR")
            self.results["errors"].append(f"Tokenizer load error: {str(e)}")
            return False
            
    def test_inference(self):
        """Тестирование inference"""
        self.log("Тестирование inference...")
        
        test_texts = [
            "def hello_world():\n    print('Hello, World!')",
            "class MyClass:\n    def __init__(self):\n        self.value = 42",
            "import numpy as np\nimport pandas as pd",
            "for i in range(10):\n    print(i)",
            "// This is a JavaScript comment\nconst x = 42;"
        ]
        
        embeddings = []
        times = []
        
        try:
            for i, text in enumerate(test_texts):
                # Токенизация
                inputs = self.tokenizer(
                    text,
                    padding=True,
                    truncation=True,
                    max_length=512,
                    return_tensors="np"
                )
                
                # Inference
                start_time = time.time()
                outputs = self.session.run(None, {
                    'input_ids': inputs['input_ids'].astype(np.int64),
                    'attention_mask': inputs['attention_mask'].astype(np.int64)
                })
                inference_time = (time.time() - start_time) * 1000
                times.append(inference_time)
                
                # Проверяем выходы
                if len(outputs) < 2:
                    self.log(f"WARNING Only {len(outputs)} outputs, expected 2+", "WARNING")
                    self.results["warnings"].append("Model has less than 2 outputs")
                    embedding = outputs[0].mean(axis=1)  # Усредняем по токенам
                else:
                    embedding = outputs[1]  # Sentence embedding
                    
                embeddings.append(embedding)
                self.log(f"  [{i+1}] Shape: {embedding.shape}, Time: {inference_time:.2f} ms")
                
            # Статистика производительности
            avg_time = np.mean(times)
            self.log(f"OK Average inference time: {avg_time:.2f} ms")
            self.results["metrics"]["avg_inference_time_ms"] = avg_time
            self.results["metrics"]["min_inference_time_ms"] = min(times)
            self.results["metrics"]["max_inference_time_ms"] = max(times)
            
            # Проверяем качество embeddings
            self.log("\nПроверка качества embeddings:")
            
            # Похожие тексты должны иметь высокое сходство
            sim_matrix = cosine_similarity(np.vstack(embeddings))
            
            # Тексты 0 и 1 - Python функции
            python_sim = sim_matrix[0, 1]
            self.log(f"  Python функции: {python_sim:.3f} (должно быть высокое)")
            
            # Тексты 0 и 4 - Python vs JavaScript
            cross_lang_sim = sim_matrix[0, 4]
            self.log(f"  Python vs JS: {cross_lang_sim:.3f} (должно быть среднее)")
            
            # Проверяем размерность
            embedding_dim = embeddings[0].shape[-1]
            self.log(f"  Размерность: {embedding_dim}")
            self.results["metrics"]["embedding_dimension"] = embedding_dim
            
            # Проверяем нормы векторов
            norms = [np.linalg.norm(emb) for emb in embeddings]
            avg_norm = np.mean(norms)
            self.log(f"  Средняя норма: {avg_norm:.3f}")
            self.results["metrics"]["avg_embedding_norm"] = avg_norm
            
            # Качество
            if python_sim > 0.7 and cross_lang_sim < python_sim:
                self.log("OK Embedding quality: GOOD")
                self.results["metrics"]["embedding_quality"] = "GOOD"
            else:
                self.log("WARNING Embedding quality: QUESTIONABLE", "WARNING")
                self.results["metrics"]["embedding_quality"] = "QUESTIONABLE"
                self.results["warnings"].append("Embedding quality is questionable")
                
            return True
            
        except Exception as e:
            self.log(f"ERROR Inference: {e}", "ERROR")
            self.results["errors"].append(f"Inference error: {str(e)}")
            return False
            
    def check_quantization_info(self):
        """Проверка информации о квантизации"""
        self.log("\nПроверка квантизации...")
        
        try:
            # Читаем конфигурацию
            config_path = self.model_dir / "model_config.json"
            if config_path.exists():
                with open(config_path, 'r') as f:
                    config = json.load(f)
                self.log(f"OK Quantization config found")
                
            # Читаем метрики
            metrics_path = self.model_dir / "metrics.json"
            if metrics_path.exists():
                with open(metrics_path, 'r') as f:
                    metrics = json.load(f)
                if "latency-avg" in metrics:
                    latency = metrics["latency-avg"]["value"]
                    self.log(f"OK Latency from metrics: {latency:.2f} ms")
                    self.results["metrics"]["reported_latency_ms"] = latency
                    
            # Проверяем размер модели
            model_size = (self.model_dir / "model.onnx").stat().st_size / (1024**3)
            self.log(f"OK Model size: {model_size:.2f} GB")
            self.results["metrics"]["model_size_gb"] = model_size
            
            if model_size < 2.0:
                self.log("OK Effective compression (< 2 GB)")
            else:
                self.log("WARNING Model is still large", "WARNING")
                self.results["warnings"].append("Model size is still large")
                
            return True
            
        except Exception as e:
            self.log(f"WARNING Cannot check quantization: {e}", "WARNING")
            return True  # Не критично
            
    def generate_report(self):
        """Генерация финального отчета"""
        self.log("\n" + "="*60)
        self.log("ФИНАЛЬНЫЙ ОТЧЕТ")
        self.log("="*60)
        
        # Определяем статус
        if self.results["errors"]:
            self.results["status"] = "FAILED"
            verdict = "FAIL: MODEL DID NOT PASS TESTS"
        elif self.results["warnings"]:
            self.results["status"] = "WARNING"
            verdict = "WARNING: MODEL WORKS BUT HAS ISSUES"
        else:
            self.results["status"] = "SUCCESS"
            verdict = "SUCCESS: MODEL IS FULLY READY"
            
        self.log(f"\n{verdict}")
        
        # Ошибки
        if self.results["errors"]:
            self.log("\nERRORS:")
            for error in self.results["errors"]:
                self.log(f"  - {error}")
                
        # Предупреждения
        if self.results["warnings"]:
            self.log("\nWARNINGS:")
            for warning in self.results["warnings"]:
                self.log(f"  - {warning}")
                
        # Метрики
        self.log("\nMETRICS:")
        for key, value in self.results["metrics"].items():
            if isinstance(value, float):
                self.log(f"  - {key}: {value:.2f}")
            else:
                self.log(f"  - {key}: {value}")
                
        # Сохраняем отчет
        report_path = self.model_dir / "test_report.json"
        with open(report_path, 'w', encoding='utf-8') as f:
            json.dump(self.results, f, indent=2, ensure_ascii=False)
        self.log(f"\nReport saved: {report_path}")
        
        return self.results["status"]
        
    def run_all_tests(self):
        """Запуск всех тестов"""
        self.log(f"Тестирование модели: {self.model_dir}")
        self.log("="*60)
        
        # Проверки
        if not self.check_files():
            return self.generate_report()
            
        if not self.load_model():
            return self.generate_report()
            
        if not self.load_tokenizer():
            return self.generate_report()
            
        if not self.test_inference():
            return self.generate_report()
            
        self.check_quantization_info()
        
        return self.generate_report()


def main():
    import sys
    if len(sys.argv) > 1:
        model_dir = sys.argv[1]
    else:
        model_dir = r"C:\Users\1\Documents\GitHub\MAGRAY_Cli\crates\memory\models\bge-code-v1-int8"
    
    print("\n[TESTING] FULL CHECK OF BGE-CODE-V1 INT8 MODEL\n")
    
    tester = ModelTester(model_dir)
    status = tester.run_all_tests()
    
    # Возвращаем код выхода
    if status == "SUCCESS":
        sys.exit(0)
    elif status == "WARNING":
        sys.exit(1)
    else:
        sys.exit(2)


if __name__ == "__main__":
    main()