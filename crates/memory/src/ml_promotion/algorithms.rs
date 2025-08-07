use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{debug, info};

use super::traits::{PromotionAlgorithm, TrainingExample, AlgorithmConfig};
use super::types::PromotionFeatures;

/// Frequency-based promotion алгоритм
#[derive(Debug, Clone)]
pub struct FrequencyAlgorithm {
    weights: FrequencyWeights,
    accuracy: f32,
    last_training: DateTime<Utc>,
    best_weights: Option<FrequencyWeights>,
    config: AlgorithmConfig,
}

#[derive(Debug, Clone)]
struct FrequencyWeights {
    access_count_weight: f32,
    access_frequency_weight: f32,
    recency_weight: f32,
    bias: f32,
}

impl Default for FrequencyWeights {
    fn default() -> Self {
        Self {
            access_count_weight: 0.5,
            access_frequency_weight: 0.3,
            recency_weight: 0.2,
            bias: 0.1,
        }
    }
}

#[async_trait]
impl PromotionAlgorithm for FrequencyAlgorithm {
    fn predict_score(&self, features: &PromotionFeatures) -> f32 {
        let score = features.access_count * self.weights.access_count_weight +
                   features.access_frequency * self.weights.access_frequency_weight +
                   features.access_recency * self.weights.recency_weight +
                   self.weights.bias;
        
        // Sigmoid activation для [0,1] range
        1.0 / (1.0 + (-score).exp())
    }
    
    async fn train(&mut self, training_data: &[TrainingExample]) -> Result<f32> {
        if training_data.is_empty() {
            return Ok(self.accuracy);
        }
        
        info!("🎯 Тренировка FrequencyAlgorithm на {} примерах", training_data.len());
        
        let mut best_accuracy = 0.0;
        let split_idx = (training_data.len() as f32 * 0.8) as usize;
        let (train_set, test_set) = training_data.split_at(split_idx);
        
        for epoch in 0..self.config.epochs {
            // Простой градиентный спуск
            for batch in train_set.chunks(self.config.batch_size) {
                let gradients = self.compute_gradients(batch);
                self.update_weights(&gradients, self.config.learning_rate);
            }
            
            // Валидация
            let accuracy = self.evaluate_accuracy(test_set);
            if accuracy > best_accuracy {
                best_accuracy = accuracy;
                self.save_best_weights();
            }
            
            if epoch % 20 == 0 {
                debug!("FrequencyAlgorithm epoch {}: accuracy={:.2}%", epoch, accuracy * 100.0);
            }
        }
        
        self.restore_best_weights();
        self.accuracy = best_accuracy;
        self.last_training = Utc::now();
        
        Ok(best_accuracy)
    }
    
    fn get_accuracy(&self) -> f32 {
        self.accuracy
    }
    
    fn save_best_weights(&mut self) {
        self.best_weights = Some(self.weights.clone());
    }
    
    fn restore_best_weights(&mut self) {
        if let Some(weights) = &self.best_weights {
            self.weights = weights.clone();
        }
    }
}

impl FrequencyAlgorithm {
    pub fn new(config: AlgorithmConfig) -> Self {
        Self {
            weights: FrequencyWeights::default(),
            accuracy: 0.7,
            last_training: Utc::now(),
            best_weights: None,
            config,
        }
    }
    
    fn compute_gradients(&self, batch: &[TrainingExample]) -> FrequencyGradients {
        let mut gradients = FrequencyGradients::default();
        
        for example in batch {
            let prediction = self.predict_score(&example.features);
            let error = prediction - example.label;
            let sigmoid_grad = prediction * (1.0 - prediction);
            let base_grad = error * sigmoid_grad;
            
            gradients.access_count_grad += base_grad * example.features.access_count;
            gradients.access_frequency_grad += base_grad * example.features.access_frequency;
            gradients.recency_grad += base_grad * example.features.access_recency;
            gradients.bias_grad += base_grad;
        }
        
        // Усредняем градиенты
        gradients.scale(1.0 / batch.len() as f32);
        gradients
    }
    
    fn update_weights(&mut self, gradients: &FrequencyGradients, learning_rate: f32) {
        self.weights.access_count_weight -= learning_rate * gradients.access_count_grad;
        self.weights.access_frequency_weight -= learning_rate * gradients.access_frequency_grad;
        self.weights.recency_weight -= learning_rate * gradients.recency_grad;
        self.weights.bias -= learning_rate * gradients.bias_grad;
        
        // L2 регуляризация
        let reg = self.config.l2_regularization * learning_rate;
        self.weights.access_count_weight *= 1.0 - reg;
        self.weights.access_frequency_weight *= 1.0 - reg;
        self.weights.recency_weight *= 1.0 - reg;
        
        // Ограничиваем веса в разумных пределах
        self.clamp_weights();
    }
    
    fn clamp_weights(&mut self) {
        self.weights.access_count_weight = self.weights.access_count_weight.clamp(-3.0, 3.0);
        self.weights.access_frequency_weight = self.weights.access_frequency_weight.clamp(-3.0, 3.0);
        self.weights.recency_weight = self.weights.recency_weight.clamp(-3.0, 3.0);
        self.weights.bias = self.weights.bias.clamp(-1.0, 1.0);
    }
    
    fn evaluate_accuracy(&self, test_set: &[TrainingExample]) -> f32 {
        let mut correct = 0;
        let threshold = 0.5;
        
        for example in test_set {
            let prediction = self.predict_score(&example.features);
            let predicted_class = if prediction >= threshold { 1.0 } else { 0.0 };
            let true_class = if example.label >= threshold { 1.0f32 } else { 0.0f32 };
            
            if (predicted_class - true_class).abs() < 0.1f32 {
                correct += 1;
            }
        }
        
        correct as f32 / test_set.len() as f32
    }
}

#[derive(Debug, Default)]
struct FrequencyGradients {
    access_count_grad: f32,
    access_frequency_grad: f32,
    recency_grad: f32,
    bias_grad: f32,
}

impl FrequencyGradients {
    fn scale(&mut self, factor: f32) {
        self.access_count_grad *= factor;
        self.access_frequency_grad *= factor;
        self.recency_grad *= factor;
        self.bias_grad *= factor;
    }
}

/// Semantic-based promotion алгоритм
#[derive(Debug, Clone)]
pub struct SemanticAlgorithm {
    weights: SemanticWeights,
    accuracy: f32,
    last_training: DateTime<Utc>,
    best_weights: Option<SemanticWeights>,
    config: AlgorithmConfig,
    topic_embeddings: HashMap<String, Vec<f32>>,
}

#[derive(Debug, Clone)]
struct SemanticWeights {
    semantic_importance_weight: f32,
    keyword_density_weight: f32,
    topic_relevance_weight: f32,
    bias: f32,
}

impl Default for SemanticWeights {
    fn default() -> Self {
        Self {
            semantic_importance_weight: 0.4,
            keyword_density_weight: 0.3,
            topic_relevance_weight: 0.3,
            bias: 0.0,
        }
    }
}

#[async_trait]
impl PromotionAlgorithm for SemanticAlgorithm {
    fn predict_score(&self, features: &PromotionFeatures) -> f32 {
        let score = features.semantic_importance * self.weights.semantic_importance_weight +
                   features.keyword_density * self.weights.keyword_density_weight +
                   features.topic_relevance * self.weights.topic_relevance_weight +
                   self.weights.bias;
        
        // Sigmoid activation
        1.0 / (1.0 + (-score).exp())
    }
    
    async fn train(&mut self, training_data: &[TrainingExample]) -> Result<f32> {
        if training_data.is_empty() {
            return Ok(self.accuracy);
        }
        
        info!("🧠 Тренировка SemanticAlgorithm на {} примерах", training_data.len());
        
        // Обучаем topic embeddings
        self.train_topic_embeddings(training_data).await?;
        
        let mut best_accuracy = 0.0;
        let split_idx = (training_data.len() as f32 * 0.8) as usize;
        let (train_set, test_set) = training_data.split_at(split_idx);
        
        for epoch in 0..self.config.epochs {
            for batch in train_set.chunks(self.config.batch_size) {
                let gradients = self.compute_gradients(batch);
                self.update_weights(&gradients, self.config.learning_rate);
            }
            
            let accuracy = self.evaluate_accuracy(test_set);
            if accuracy > best_accuracy {
                best_accuracy = accuracy;
                self.save_best_weights();
            }
            
            if epoch % 20 == 0 {
                debug!("SemanticAlgorithm epoch {}: accuracy={:.2}%", epoch, accuracy * 100.0);
            }
        }
        
        self.restore_best_weights();
        self.accuracy = best_accuracy;
        self.last_training = Utc::now();
        
        Ok(best_accuracy)
    }
    
    fn get_accuracy(&self) -> f32 {
        self.accuracy
    }
    
    fn save_best_weights(&mut self) {
        self.best_weights = Some(self.weights.clone());
    }
    
    fn restore_best_weights(&mut self) {
        if let Some(weights) = &self.best_weights {
            self.weights = weights.clone();
        }
    }
}

impl SemanticAlgorithm {
    pub fn new(config: AlgorithmConfig) -> Self {
        Self {
            weights: SemanticWeights::default(),
            accuracy: 0.75,
            last_training: Utc::now(),
            best_weights: None,
            config,
            topic_embeddings: HashMap::new(),
        }
    }
    
    async fn train_topic_embeddings(&mut self, _training_data: &[TrainingExample]) -> Result<()> {
        // Placeholder для обучения topic embeddings
        debug!("Обучение topic embeddings");
        Ok(())
    }
    
    fn compute_gradients(&self, batch: &[TrainingExample]) -> SemanticGradients {
        let mut gradients = SemanticGradients::default();
        
        for example in batch {
            let prediction = self.predict_score(&example.features);
            let error = prediction - example.label;
            let sigmoid_grad = prediction * (1.0 - prediction);
            let base_grad = error * sigmoid_grad;
            
            gradients.semantic_importance_grad += base_grad * example.features.semantic_importance;
            gradients.keyword_density_grad += base_grad * example.features.keyword_density;
            gradients.topic_relevance_grad += base_grad * example.features.topic_relevance;
            gradients.bias_grad += base_grad;
        }
        
        gradients.scale(1.0 / batch.len() as f32);
        gradients
    }
    
    fn update_weights(&mut self, gradients: &SemanticGradients, learning_rate: f32) {
        self.weights.semantic_importance_weight -= learning_rate * gradients.semantic_importance_grad;
        self.weights.keyword_density_weight -= learning_rate * gradients.keyword_density_grad;
        self.weights.topic_relevance_weight -= learning_rate * gradients.topic_relevance_grad;
        self.weights.bias -= learning_rate * gradients.bias_grad;
        
        // L2 регуляризация
        let reg = self.config.l2_regularization * learning_rate;
        self.weights.semantic_importance_weight *= 1.0 - reg;
        self.weights.keyword_density_weight *= 1.0 - reg;
        self.weights.topic_relevance_weight *= 1.0 - reg;
        
        self.clamp_weights();
    }
    
    fn clamp_weights(&mut self) {
        self.weights.semantic_importance_weight = self.weights.semantic_importance_weight.clamp(-3.0, 3.0);
        self.weights.keyword_density_weight = self.weights.keyword_density_weight.clamp(-3.0, 3.0);
        self.weights.topic_relevance_weight = self.weights.topic_relevance_weight.clamp(-3.0, 3.0);
        self.weights.bias = self.weights.bias.clamp(-1.0, 1.0);
    }
    
    fn evaluate_accuracy(&self, test_set: &[TrainingExample]) -> f32 {
        let mut correct = 0;
        let threshold = 0.5;
        
        for example in test_set {
            let prediction = self.predict_score(&example.features);
            let predicted_class = if prediction >= threshold { 1.0 } else { 0.0 };
            let true_class = if example.label >= threshold { 1.0f32 } else { 0.0f32 };
            
            if (predicted_class - true_class).abs() < 0.1f32 {
                correct += 1;
            }
        }
        
        correct as f32 / test_set.len() as f32
    }
}

#[derive(Debug, Default)]
struct SemanticGradients {
    semantic_importance_grad: f32,
    keyword_density_grad: f32,
    topic_relevance_grad: f32,
    bias_grad: f32,
}

impl SemanticGradients {
    fn scale(&mut self, factor: f32) {
        self.semantic_importance_grad *= factor;
        self.keyword_density_grad *= factor;
        self.topic_relevance_grad *= factor;
        self.bias_grad *= factor;
    }
}

/// Hybrid promotion алгоритм (комбинирует frequency + semantic)
#[derive(Debug, Clone)]
pub struct HybridAlgorithm {
    temporal_weights: Vec<f32>,
    usage_weights: Vec<f32>,
    semantic_weights: Vec<f32>,
    bias: f32,
    accuracy: f32,
    last_training: DateTime<Utc>,
    best_weights: Option<HybridWeights>,
    config: AlgorithmConfig,
}

#[derive(Debug, Clone)]
struct HybridWeights {
    temporal_weights: Vec<f32>,
    usage_weights: Vec<f32>,
    semantic_weights: Vec<f32>,
    bias: f32,
}

#[async_trait]
impl PromotionAlgorithm for HybridAlgorithm {
    fn predict_score(&self, features: &PromotionFeatures) -> f32 {
        // Temporal component
        let temporal_score = 
            features.age_hours * self.temporal_weights[0] +
            features.access_recency * self.temporal_weights[1] +
            features.temporal_pattern_score * self.temporal_weights[2];

        // Usage component
        let usage_score = 
            features.access_count * self.usage_weights[0] +
            features.access_frequency * self.usage_weights[1] +
            features.session_importance * self.usage_weights[2];

        // Semantic component
        let semantic_score = 
            features.semantic_importance * self.semantic_weights[0] +
            features.keyword_density * self.semantic_weights[1] +
            features.topic_relevance * self.semantic_weights[2];

        let final_score = temporal_score + usage_score + semantic_score + self.bias;
        
        // Sigmoid activation
        1.0 / (1.0 + (-final_score).exp())
    }
    
    async fn train(&mut self, training_data: &[TrainingExample]) -> Result<f32> {
        if training_data.is_empty() {
            return Ok(self.accuracy);
        }
        
        info!("🚀 Тренировка HybridAlgorithm на {} примерах", training_data.len());
        
        let mut best_accuracy = 0.0;
        let split_idx = (training_data.len() as f32 * 0.8) as usize;
        let (train_set, test_set) = training_data.split_at(split_idx);
        
        for epoch in 0..self.config.epochs {
            for batch in train_set.chunks(self.config.batch_size) {
                let gradients = self.compute_gradients(batch);
                self.update_weights(&gradients, self.config.learning_rate);
            }
            
            let accuracy = self.evaluate_accuracy(test_set);
            if accuracy > best_accuracy {
                best_accuracy = accuracy;
                self.save_best_weights();
            }
            
            if epoch % 10 == 0 {
                debug!("HybridAlgorithm epoch {}: accuracy={:.2}%", epoch, accuracy * 100.0);
            }
        }
        
        self.restore_best_weights();
        self.accuracy = best_accuracy;
        self.last_training = Utc::now();
        
        info!("✅ HybridAlgorithm тренировка завершена: accuracy={:.1}%", best_accuracy * 100.0);
        Ok(best_accuracy)
    }
    
    fn get_accuracy(&self) -> f32 {
        self.accuracy
    }
    
    fn save_best_weights(&mut self) {
        self.best_weights = Some(HybridWeights {
            temporal_weights: self.temporal_weights.clone(),
            usage_weights: self.usage_weights.clone(),
            semantic_weights: self.semantic_weights.clone(),
            bias: self.bias,
        });
    }
    
    fn restore_best_weights(&mut self) {
        if let Some(weights) = &self.best_weights {
            self.temporal_weights = weights.temporal_weights.clone();
            self.usage_weights = weights.usage_weights.clone();
            self.semantic_weights = weights.semantic_weights.clone();
            self.bias = weights.bias;
        }
    }
}

impl HybridAlgorithm {
    pub fn new(config: AlgorithmConfig) -> Self {
        Self {
            temporal_weights: vec![0.2, 0.3, 0.5],
            usage_weights: vec![0.5, 0.3, 0.2],
            semantic_weights: vec![0.4, 0.3, 0.3],
            bias: 0.1,
            accuracy: 0.8,
            last_training: Utc::now(),
            best_weights: None,
            config,
        }
    }
    
    fn compute_gradients(&self, batch: &[TrainingExample]) -> HybridGradients {
        let mut gradients = HybridGradients::default();
        
        for example in batch {
            let prediction = self.predict_score(&example.features);
            let error = prediction - example.label;
            let sigmoid_grad = prediction * (1.0 - prediction);
            let base_grad = error * sigmoid_grad;
            
            // Temporal gradients
            gradients.temporal_grads[0] += base_grad * example.features.age_hours;
            gradients.temporal_grads[1] += base_grad * example.features.access_recency;
            gradients.temporal_grads[2] += base_grad * example.features.temporal_pattern_score;
            
            // Usage gradients
            gradients.usage_grads[0] += base_grad * example.features.access_count;
            gradients.usage_grads[1] += base_grad * example.features.access_frequency;
            gradients.usage_grads[2] += base_grad * example.features.session_importance;
            
            // Semantic gradients
            gradients.semantic_grads[0] += base_grad * example.features.semantic_importance;
            gradients.semantic_grads[1] += base_grad * example.features.keyword_density;
            gradients.semantic_grads[2] += base_grad * example.features.topic_relevance;
            
            gradients.bias_grad += base_grad;
        }
        
        gradients.scale(1.0 / batch.len() as f32);
        gradients
    }
    
    fn update_weights(&mut self, gradients: &HybridGradients, learning_rate: f32) {
        // Update temporal weights
        for i in 0..3 {
            self.temporal_weights[i] -= learning_rate * gradients.temporal_grads[i];
        }
        
        // Update usage weights
        for i in 0..3 {
            self.usage_weights[i] -= learning_rate * gradients.usage_grads[i];
        }
        
        // Update semantic weights
        for i in 0..3 {
            self.semantic_weights[i] -= learning_rate * gradients.semantic_grads[i];
        }
        
        self.bias -= learning_rate * gradients.bias_grad;
        
        // L2 regularization
        let reg = self.config.l2_regularization * learning_rate;
        self.apply_l2_regularization(reg);
        
        self.clamp_weights();
    }
    
    fn apply_l2_regularization(&mut self, reg: f32) {
        for weight in &mut self.temporal_weights {
            *weight *= 1.0 - reg;
        }
        for weight in &mut self.usage_weights {
            *weight *= 1.0 - reg;
        }
        for weight in &mut self.semantic_weights {
            *weight *= 1.0 - reg;
        }
    }
    
    fn clamp_weights(&mut self) {
        let clamp = |weights: &mut Vec<f32>| {
            for w in weights {
                *w = w.clamp(-5.0, 5.0);
            }
        };
        
        clamp(&mut self.temporal_weights);
        clamp(&mut self.usage_weights);
        clamp(&mut self.semantic_weights);
        self.bias = self.bias.clamp(-2.0, 2.0);
    }
    
    fn evaluate_accuracy(&self, test_set: &[TrainingExample]) -> f32 {
        let mut correct = 0;
        let threshold = 0.7; // Hybrid алгоритм использует более высокий threshold
        
        for example in test_set {
            let prediction = self.predict_score(&example.features);
            let predicted_class = if prediction >= threshold { 1.0 } else { 0.0 };
            let true_class = if example.label >= threshold { 1.0f32 } else { 0.0f32 };
            
            if (predicted_class - true_class).abs() < 0.1f32 {
                correct += 1;
            }
        }
        
        correct as f32 / test_set.len() as f32
    }
}

#[derive(Debug, Default)]
struct HybridGradients {
    temporal_grads: [f32; 3],
    usage_grads: [f32; 3],
    semantic_grads: [f32; 3],
    bias_grad: f32,
}

impl HybridGradients {
    fn scale(&mut self, factor: f32) {
        for i in 0..3 {
            self.temporal_grads[i] *= factor;
            self.usage_grads[i] *= factor;
            self.semantic_grads[i] *= factor;
        }
        self.bias_grad *= factor;
    }
}

/// Фабрика для создания алгоритмов
pub struct AlgorithmFactory;

impl AlgorithmFactory {
    pub fn create(algorithm_name: &str, config: AlgorithmConfig) -> Result<Box<dyn PromotionAlgorithm>> {
        let algorithm: Box<dyn PromotionAlgorithm> = match algorithm_name {
            "frequency" => Box::new(FrequencyAlgorithm::new(config)),
            "semantic" => Box::new(SemanticAlgorithm::new(config)),
            "hybrid" => Box::new(HybridAlgorithm::new(config)),
            _ => return Err(anyhow::anyhow!("Unknown algorithm: {}", algorithm_name)),
        };
        
        info!("✅ Создан {} алгоритм promotion", algorithm_name);
        Ok(algorithm)
    }
    
    pub fn available_algorithms() -> Vec<&'static str> {
        vec!["frequency", "semantic", "hybrid"]
    }
}