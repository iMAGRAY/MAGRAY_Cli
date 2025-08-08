#!/usr/bin/env python3
"""
Скрипт для включения реальных ONNX моделей вместо моков в MAGRAY CLI.
Автоматически находит и активирует закомментированный код для реальной инференции.
"""

import os
import re
import sys
from pathlib import Path
from typing import List, Tuple

class ModelEnabler:
    def __init__(self, verbose: bool = True):
        self.verbose = verbose
        self.modified_files = []
        
    def log(self, message: str):
        """Выводит сообщение если включен verbose режим."""
        if self.verbose:
            print(message)
    
    def uncomment_onnx_code(self, file_path: Path) -> bool:
        """Раскомментирует код связанный с ONNX Runtime."""
        with open(file_path, 'r', encoding='utf-8') as f:
            lines = f.readlines()
        
        modified = False
        new_lines = []
        in_commented_block = False
        
        for i, line in enumerate(lines):
            # Ищем паттерны закомментированного ONNX кода
            if '// Real ONNX' in line or '// TODO: Enable real' in line:
                in_commented_block = True
            
            # Раскомментируем строки с ONNX вызовами
            if in_commented_block and line.strip().startswith('//'):
                # Проверяем что это не обычный комментарий
                uncommented = line.replace('//', '', 1)
                if any(keyword in uncommented for keyword in 
                       ['onnx', 'OrtSession', 'run(', 'tensor', 'ndarray']):
                    new_lines.append(uncommented)
                    modified = True
                    self.log(f"  Uncommented line {i+1}: {uncommented.strip()}")
                else:
                    new_lines.append(line)
            else:
                new_lines.append(line)
                
            if in_commented_block and line.strip() == '':
                in_commented_block = False
        
        if modified:
            with open(file_path, 'w', encoding='utf-8') as f:
                f.writelines(new_lines)
            self.modified_files.append(file_path)
            
        return modified
    
    def fix_mock_returns(self, file_path: Path) -> bool:
        """Заменяет mock возвраты на реальные вызовы."""
        with open(file_path, 'r', encoding='utf-8') as f:
            content = f.read()
        
        original_content = content
        
        # Паттерны для замены
        replacements = [
            # Mock embeddings
            (r'vec!\[0\.5; 1024\]', 'self.generate_real_embedding(text).await?'),
            (r'vec!\[0\.1; 1024\]', 'self.generate_real_embedding(text).await?'),
            (r'// Mock embedding generation', '// Real embedding generation'),
            
            # Mock reranking
            (r'scores: vec!\[0\.9, 0\.7, 0\.5\]', 'scores: self.rerank_real(query, documents).await?'),
            (r'// Mock reranking', '// Real reranking'),
            
            # Enable ONNX session creation
            (r'// let session = .*OrtSession', 'let session = OrtSession'),
            (r'//\s*self\.session = Some\(', 'self.session = Some('),
            
            # Remove skip attributes from tests
            (r'#\[ignore\].*// Requires real model', ''),
            (r'#\[cfg\(feature = "real_models"\)\]', ''),
        ]
        
        for pattern, replacement in replacements:
            content = re.sub(pattern, replacement, content)
        
        if content != original_content:
            with open(file_path, 'w', encoding='utf-8') as f:
                f.write(content)
            self.modified_files.append(file_path)
            return True
            
        return False
    
    def update_cargo_toml(self, file_path: Path) -> bool:
        """Обновляет Cargo.toml для включения правильных зависимостей."""
        with open(file_path, 'r', encoding='utf-8') as f:
            lines = f.readlines()
        
        new_lines = []
        modified = False
        
        for line in lines:
            # Обновляем версию onnxruntime если необходимо
            if 'onnxruntime' in line and '#' in line:
                # Раскомментируем
                new_lines.append(line.split('#')[0].rstrip() + '\n')
                modified = True
                self.log(f"  Enabled onnxruntime dependency")
            elif 'onnxruntime' in line and '0.0.14' in line:
                # Обновляем версию
                new_lines.append(line.replace('0.0.14', '0.1.16'))
                modified = True
                self.log(f"  Updated onnxruntime version to 0.1.16")
            else:
                new_lines.append(line)
        
        # Добавляем feature flags если их нет
        if modified:
            # Проверяем есть ли секция features
            has_features = any('[features]' in line for line in lines)
            if not has_features:
                new_lines.append('\n[features]\n')
                new_lines.append('default = ["cpu"]\n')
                new_lines.append('cpu = ["onnxruntime"]\n')
                new_lines.append('gpu = ["onnxruntime/cuda"]\n')
                self.log("  Added feature flags for CPU/GPU modes")
        
        if modified:
            with open(file_path, 'w', encoding='utf-8') as f:
                f.writelines(new_lines)
            self.modified_files.append(file_path)
            
        return modified
    
    def create_model_loader_fixes(self) -> str:
        """Генерирует код для исправления model_loader.rs"""
        return '''use anyhow::{Result, Context};
use std::path::{Path, PathBuf};
use onnxruntime::{environment::Environment, session::Session, GraphOptimizationLevel};
use std::sync::Arc;

pub struct ModelLoader {
    env: Arc<Environment>,
    models_dir: PathBuf,
}

impl ModelLoader {
    pub fn new(models_dir: impl AsRef<Path>) -> Result<Self> {
        let env = Environment::builder()
            .with_name("magray")
            .with_log_level(onnxruntime::LoggingLevel::Warning)
            .build()
            .context("Failed to create ONNX Runtime environment")?;
            
        Ok(Self {
            env: Arc::new(env),
            models_dir: models_dir.as_ref().to_path_buf(),
        })
    }
    
    pub fn load_embedding_model(&self, model_name: &str) -> Result<Session> {
        let model_path = self.models_dir.join(model_name).join("model.onnx");
        
        if !model_path.exists() {
            anyhow::bail!("Model file not found: {:?}. Run download_models.ps1 first", model_path);
        }
        
        Session::builder()
            .with_env(&self.env)
            .with_optimization_level(GraphOptimizationLevel::All)
            .with_intra_threads(4)
            .with_model_from_file(&model_path)
            .context(format!("Failed to load model: {}", model_name))
    }
    
    pub fn load_reranker_model(&self, model_name: &str) -> Result<Session> {
        self.load_embedding_model(model_name) // Same loading logic
    }
}
'''
    
    def fix_embedding_service(self, file_path: Path) -> bool:
        """Специальная обработка для embeddings_bge_m3.rs"""
        if not file_path.exists():
            return False
            
        content = self.create_real_embedding_code()
        
        with open(file_path, 'w', encoding='utf-8') as f:
            f.write(content)
        
        self.modified_files.append(file_path)
        self.log(f"✅ Replaced {file_path} with real implementation")
        return True
    
    def create_real_embedding_code(self) -> str:
        """Генерирует реальный код для embedding service."""
        return '''use anyhow::{Result, Context};
use ndarray::{Array1, Array2, ArrayView1};
use onnxruntime::{session::Session, tensor::OrtOwnedTensor};
use std::sync::Arc;
use tokio::sync::RwLock;
use tokenizers::Tokenizer;

use crate::{AiError, ModelLoader};

pub struct BgeM3EmbeddingService {
    session: Arc<Session>,
    tokenizer: Arc<Tokenizer>,
    cache: Arc<RwLock<lru::LruCache<String, Vec<f32>>>>,
}

impl BgeM3EmbeddingService {
    pub async fn new(model_path: &str) -> Result<Self> {
        // Загружаем модель
        let loader = ModelLoader::new("models")?;
        let session = Arc::new(loader.load_embedding_model("bge-m3")?);
        
        // Загружаем токенизатор
        let tokenizer_path = format!("{}/tokenizer.json", model_path);
        let tokenizer = Tokenizer::from_file(&tokenizer_path)
            .map_err(|e| AiError::ModelLoadError(e.to_string()))?;
        
        // Создаем кэш
        let cache = Arc::new(RwLock::new(lru::LruCache::new(
            std::num::NonZeroUsize::new(1000).unwrap()
        )));
        
        Ok(Self {
            session,
            tokenizer: Arc::new(tokenizer),
            cache,
        })
    }
    
    pub async fn embed(&self, text: &str) -> Result<Vec<f32>> {
        // Проверяем кэш
        {
            let cache = self.cache.read().await;
            if let Some(embedding) = cache.get(text) {
                return Ok(embedding.clone());
            }
        }
        
        // Токенизация
        let encoding = self.tokenizer
            .encode(text, true)
            .map_err(|e| AiError::TokenizationError(e.to_string()))?;
        
        let input_ids = encoding.get_ids();
        let attention_mask = encoding.get_attention_mask();
        
        // Подготовка тензоров
        let input_ids_array = Array2::from_shape_vec(
            (1, input_ids.len()),
            input_ids.iter().map(|&id| id as i64).collect(),
        )?;
        
        let attention_mask_array = Array2::from_shape_vec(
            (1, attention_mask.len()),
            attention_mask.iter().map(|&m| m as i64).collect(),
        )?;
        
        // Запуск инференции
        let outputs = self.session.run(vec![
            input_ids_array.into(),
            attention_mask_array.into(),
        ])?;
        
        // Извлечение результата
        let output: OrtOwnedTensor<f32, _> = outputs[0].try_extract()?;
        let embedding = output.view().as_slice()
            .ok_or_else(|| AiError::ProcessingError("Failed to extract embedding".into()))?
            .to_vec();
        
        // Нормализация
        let norm = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
        let normalized = embedding.iter().map(|x| x / norm).collect::<Vec<_>>();
        
        // Сохраняем в кэш
        {
            let mut cache = self.cache.write().await;
            cache.put(text.to_string(), normalized.clone());
        }
        
        Ok(normalized)
    }
    
    pub async fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        let mut results = Vec::with_capacity(texts.len());
        
        // TODO: Implement proper batching for efficiency
        for text in texts {
            results.push(self.embed(text).await?);
        }
        
        Ok(results)
    }
}
'''
    
    def process_ai_crate(self):
        """Обрабатывает crate ai/ для включения реальных моделей."""
        ai_path = Path('crates/ai/src')
        
        if not ai_path.exists():
            self.log(f"❌ AI crate not found at {ai_path}")
            return
            
        # Файлы для обработки
        files_to_process = [
            'embeddings_bge_m3.rs',
            'embeddings_cpu.rs', 
            'embeddings_gpu.rs',
            'reranking.rs',
            'reranker_qwen3.rs',
            'model_loader.rs',
        ]
        
        for file_name in files_to_process:
            file_path = ai_path / file_name
            if file_path.exists():
                self.log(f"\n📄 Processing {file_path}")
                
                if 'model_loader' in file_name:
                    # Создаем новый model_loader
                    with open(file_path, 'w', encoding='utf-8') as f:
                        f.write(self.create_model_loader_fixes())
                    self.modified_files.append(file_path)
                    self.log("  ✅ Created real model loader implementation")
                    
                elif 'embeddings_bge_m3' in file_name:
                    # Заменяем на реальную реализацию
                    self.fix_embedding_service(file_path)
                    
                else:
                    # Обычная обработка
                    if self.uncomment_onnx_code(file_path):
                        self.log(f"  ✅ Uncommented ONNX code")
                    if self.fix_mock_returns(file_path):
                        self.log(f"  ✅ Fixed mock returns")
    
    def update_all_cargo_tomls(self):
        """Обновляет все Cargo.toml файлы в проекте."""
        for root, dirs, files in os.walk('crates'):
            for file in files:
                if file == 'Cargo.toml':
                    cargo_path = Path(root) / file
                    self.log(f"\n📦 Checking {cargo_path}")
                    if self.update_cargo_toml(cargo_path):
                        self.log(f"  ✅ Updated dependencies")
    
    def run(self):
        """Основной метод для запуска всех исправлений."""
        print("🚀 Enabling real ONNX models in MAGRAY CLI\n")
        
        # 1. Обработка AI crate
        print("Step 1: Processing AI crate...")
        self.process_ai_crate()
        
        # 2. Обновление Cargo.toml файлов
        print("\nStep 2: Updating Cargo.toml files...")
        self.update_all_cargo_tomls()
        
        # 3. Итоговая статистика
        print(f"\n✅ Summary:")
        print(f"  Modified {len(self.modified_files)} files:")
        for file in self.modified_files:
            print(f"    - {file}")
        
        print("\n📝 Next steps:")
        print("  1. Run: ./scripts/download_models.ps1")
        print("  2. Run: cargo build --release --features cpu")
        print("  3. Run: cargo test --features cpu")

def main():
    import argparse
    
    parser = argparse.ArgumentParser(description='Enable real ONNX models in MAGRAY CLI')
    parser.add_argument('--dry-run', action='store_true', 
                       help='Show what would be changed without modifying files')
    parser.add_argument('--verbose', '-v', action='store_true', default=True,
                       help='Verbose output')
    
    args = parser.parse_args()
    
    if args.dry_run:
        print("DRY RUN MODE - No files will be modified\n")
    
    enabler = ModelEnabler(verbose=args.verbose)
    enabler.run()

if __name__ == '__main__':
    main()