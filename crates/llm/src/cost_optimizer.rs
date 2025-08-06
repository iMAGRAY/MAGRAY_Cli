use crate::{LlmProvider, ProviderType, TaskComplexity, ComplexityLevel, Priority};
use std::collections::HashMap;
use tracing::{info, debug};

/// Cost per 1K tokens for different providers and models
#[derive(Debug, Clone)]
pub struct CostTable {
    costs: HashMap<(ProviderType, String), (f32, f32)>, // (input_cost, output_cost) per 1K tokens
}

impl Default for CostTable {
    fn default() -> Self {
        let mut costs = HashMap::new();
        
        // OpenAI pricing (as of 2024)
        costs.insert((ProviderType::OpenAI, "gpt-4o".to_string()), (0.0025, 0.01));
        costs.insert((ProviderType::OpenAI, "gpt-4o-mini".to_string()), (0.00015, 0.0006));
        costs.insert((ProviderType::OpenAI, "gpt-4-turbo".to_string()), (0.01, 0.03));
        costs.insert((ProviderType::OpenAI, "gpt-3.5-turbo".to_string()), (0.0005, 0.0015));
        
        // Anthropic pricing
        costs.insert((ProviderType::Anthropic, "claude-3-5-sonnet-20241022".to_string()), (0.003, 0.015));
        costs.insert((ProviderType::Anthropic, "claude-3-haiku-20240307".to_string()), (0.00025, 0.00125));
        costs.insert((ProviderType::Anthropic, "claude-3-sonnet-20240229".to_string()), (0.003, 0.015));
        costs.insert((ProviderType::Anthropic, "claude-3-opus-20240229".to_string()), (0.015, 0.075));
        
        // Groq pricing (ultra-fast inference)
        costs.insert((ProviderType::Groq, "llama-3.1-8b-instant".to_string()), (0.00005, 0.00008));
        costs.insert((ProviderType::Groq, "llama-3.1-70b-versatile".to_string()), (0.00059, 0.00079));
        costs.insert((ProviderType::Groq, "mixtral-8x7b-32768".to_string()), (0.00024, 0.00024));
        
        // Local models (essentially free after setup)
        costs.insert((ProviderType::Local, "any".to_string()), (0.0, 0.0));
        costs.insert((ProviderType::Ollama, "any".to_string()), (0.0, 0.0));
        costs.insert((ProviderType::LMStudio, "any".to_string()), (0.0, 0.0));
        
        // Azure (similar to OpenAI but can vary)
        costs.insert((ProviderType::Azure, "gpt-4o".to_string()), (0.0025, 0.01));
        costs.insert((ProviderType::Azure, "gpt-4o-mini".to_string()), (0.00015, 0.0006));
        
        Self { costs }
    }
}

impl CostTable {
    pub fn get_cost(&self, provider_type: &ProviderType, model: &str) -> (f32, f32) {
        self.costs.get(&(provider_type.clone(), model.to_string()))
            .or_else(|| self.costs.get(&(provider_type.clone(), "any".to_string())))
            .copied()
            .unwrap_or((0.001, 0.003)) // Default fallback cost
    }
    
    pub fn estimate_cost(&self, provider_type: &ProviderType, model: &str, input_tokens: u32, output_tokens: u32) -> f32 {
        let (input_cost, output_cost) = self.get_cost(provider_type, model);
        (input_tokens as f32 / 1000.0 * input_cost) + (output_tokens as f32 / 1000.0 * output_cost)
    }
}

/// Smart cost optimizer for selecting optimal providers
#[derive(Debug, Clone)]
pub struct CostOptimizer {
    pub cost_table: CostTable,
    budget_limit: Option<f32>, // Daily budget limit
    current_spend: f32,
}

impl Default for CostOptimizer {
    fn default() -> Self {
        Self {
            cost_table: CostTable::default(),
            budget_limit: None,
            current_spend: 0.0,
        }
    }
}

impl CostOptimizer {
    pub fn new(budget_limit: Option<f32>) -> Self {
        Self {
            cost_table: CostTable::default(),
            budget_limit,
            current_spend: 0.0,
        }
    }
    
    /// Select optimal provider based on task complexity and cost
    pub fn select_optimal_provider(
        &self,
        available_providers: &[LlmProvider],
        task: &TaskComplexity,
    ) -> Option<LlmProvider> {
        if available_providers.is_empty() {
            return None;
        }
        
        debug!("🧮 Selecting optimal provider for task: complexity={:?}, priority={:?}, tokens={}", 
            task.complexity, task.priority, task.tokens);
        
        // Filter providers based on complexity requirements
        let suitable_providers: Vec<&LlmProvider> = available_providers
            .iter()
            .filter(|provider| self.is_suitable_for_complexity(provider, &task.complexity))
            .collect();
        
        if suitable_providers.is_empty() {
            info!("⚠️ No suitable providers for complexity level {:?}, using any available", task.complexity);
            return Some(available_providers[0].clone());
        }
        
        // For critical priority, prefer premium providers
        if task.priority == Priority::Critical {
            let premium = suitable_providers.iter()
                .find(|p| self.is_premium_provider(p));
            if let Some(provider) = premium {
                info!("🏆 Selected premium provider for CRITICAL task");
                return Some((*provider).clone());
            }
        }
        
        // For normal tasks, optimize for cost
        let mut best_provider = suitable_providers[0];
        let mut best_cost = f32::MAX;
        
        for provider in &suitable_providers {
            let estimated_cost = self.estimate_provider_cost(provider, task.tokens, task.tokens / 4);
            
            if estimated_cost < best_cost {
                best_cost = estimated_cost;
                best_provider = provider;
            }
        }
        
        info!("💰 Selected cost-optimal provider: estimated cost ${:.4}", best_cost);
        Some(best_provider.clone())
    }
    
    /// Check if provider is suitable for given complexity
    fn is_suitable_for_complexity(&self, provider: &LlmProvider, complexity: &ComplexityLevel) -> bool {
        match complexity {
            ComplexityLevel::Simple => true, // Any provider can handle simple tasks
            ComplexityLevel::Medium => {
                // Medium tasks need decent models
                match provider {
                    LlmProvider::OpenAI { model, .. } => {
                        !model.contains("gpt-3.5-turbo")
                    }
                    LlmProvider::Anthropic { model, .. } => {
                        !model.contains("haiku") // Haiku might struggle with medium complexity
                    }
                    LlmProvider::Groq { model, .. } => {
                        model.contains("70b") || model.contains("mixtral")
                    }
                    _ => true, // Local models assumed to be capable
                }
            }
            ComplexityLevel::Complex | ComplexityLevel::Expert => {
                // Complex tasks need premium models
                match provider {
                    LlmProvider::OpenAI { model, .. } => {
                        model.contains("gpt-4")
                    }
                    LlmProvider::Anthropic { model, .. } => {
                        model.contains("sonnet") || model.contains("opus")
                    }
                    LlmProvider::Groq { model, .. } => {
                        model.contains("70b") || model.contains("mixtral")
                    }
                    _ => true, // Assume local models are high-quality
                }
            }
        }
    }
    
    /// Check if provider is considered premium (high quality)
    fn is_premium_provider(&self, provider: &LlmProvider) -> bool {
        match provider {
            LlmProvider::OpenAI { model, .. } => model.contains("gpt-4o") || model.contains("gpt-4-turbo"),
            LlmProvider::Anthropic { model, .. } => model.contains("sonnet") || model.contains("opus"),
            _ => false,
        }
    }
    
    /// Estimate cost for a provider
    fn estimate_provider_cost(&self, provider: &LlmProvider, input_tokens: u32, output_tokens: u32) -> f32 {
        let (provider_type, model) = match provider {
            LlmProvider::OpenAI { model, .. } => (ProviderType::OpenAI, model.as_str()),
            LlmProvider::Anthropic { model, .. } => (ProviderType::Anthropic, model.as_str()),
            LlmProvider::Local { model, .. } => (ProviderType::Local, model.as_str()),
            LlmProvider::Ollama { model, .. } => (ProviderType::Ollama, model.as_str()),
            LlmProvider::LMStudio { model, .. } => (ProviderType::LMStudio, model.as_str()),
            LlmProvider::Azure { model, .. } => (ProviderType::Azure, model.as_str()),
            LlmProvider::Groq { model, .. } => (ProviderType::Groq, model.as_str()),
        };
        
        self.cost_table.estimate_cost(&provider_type, model, input_tokens, output_tokens)
    }
    
    /// Record actual spending
    pub fn record_cost(&mut self, cost: f32) {
        self.current_spend += cost;
        debug!("💸 Recorded cost: ${:.4} (total: ${:.2})", cost, self.current_spend);
        
        if let Some(limit) = self.budget_limit {
            if self.current_spend > limit * 0.9 {
                info!("⚠️ Budget warning: {:.1}% of daily limit used", 
                    (self.current_spend / limit) * 100.0);
            }
        }
    }
    
    /// Check if within budget
    pub fn is_within_budget(&self, additional_cost: f32) -> bool {
        match self.budget_limit {
            Some(limit) => (self.current_spend + additional_cost) <= limit,
            None => true,
        }
    }
    
    /// Get spending summary
    pub fn get_spending_summary(&self) -> String {
        match self.budget_limit {
            Some(limit) => {
                format!("Spent: ${:.2} / ${:.2} ({:.1}%)", 
                    self.current_spend, limit, (self.current_spend / limit) * 100.0)
            }
            None => {
                format!("Spent: ${:.2} (no limit)", self.current_spend)
            }
        }
    }
    
    /// Reset daily spending (should be called daily)
    pub fn reset_daily_spending(&mut self) {
        info!("📊 Daily spending reset. Previous: ${:.2}", self.current_spend);
        self.current_spend = 0.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cost_estimation() {
        let optimizer = CostOptimizer::default();
        
        let cost = optimizer.cost_table.estimate_cost(&ProviderType::OpenAI, "gpt-4o-mini", 1000, 500);
        assert!(cost > 0.0);
        println!("GPT-4o-mini cost for 1K input + 500 output: ${:.4}", cost);
        
        let free_cost = optimizer.cost_table.estimate_cost(&ProviderType::Local, "llama", 1000, 500);
        assert_eq!(free_cost, 0.0);
    }
    
    #[test]
    fn test_complexity_filtering() {
        let optimizer = CostOptimizer::default();
        let provider = LlmProvider::OpenAI { 
            api_key: "test".to_string(), 
            model: "gpt-3.5-turbo".to_string() 
        };
        
        assert!(optimizer.is_suitable_for_complexity(&provider, &ComplexityLevel::Simple));
        assert!(!optimizer.is_suitable_for_complexity(&provider, &ComplexityLevel::Medium));
    }
}