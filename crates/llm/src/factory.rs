use crate::providers::*;
use anyhow::{anyhow, Result};
use domain::config::{MagrayConfig, ProviderConfig, ProviderType};
use std::sync::Arc;

/// Factory for creating LLM providers from configuration
pub struct LlmProviderFactory;

impl LlmProviderFactory {
    /// Create a provider from configuration
    pub fn create_provider(
        provider_name: &str,
        config: &ProviderConfig,
    ) -> Result<Arc<dyn LlmProvider + Send + Sync>> {
        match config.provider_type {
            ProviderType::OpenAI => {
                let api_key = config
                    .api_key
                    .as_ref()
                    .ok_or_else(|| anyhow!("OpenAI API key is required"))?;
                
                let model = config
                    .model
                    .as_ref()
                    .unwrap_or(&"gpt-4o-mini".to_string())
                    .clone();
                
                let endpoint = config.api_base.clone();
                
                let provider = OpenAIProvider::new(api_key.clone(), model, endpoint)?;
                Ok(Arc::new(provider))
            }
            ProviderType::Anthropic => {
                let api_key = config
                    .api_key
                    .as_ref()
                    .ok_or_else(|| anyhow!("Anthropic API key is required"))?;
                
                let model = config
                    .model
                    .as_ref()
                    .unwrap_or(&"claude-3-haiku-20240307".to_string())
                    .clone();
                
                let endpoint = config.api_base.clone();
                
                let provider = AnthropicProvider::new(api_key.clone(), model, endpoint)?;
                Ok(Arc::new(provider))
            }
            ProviderType::Google => {
                let api_key = config
                    .api_key
                    .as_ref()
                    .ok_or_else(|| anyhow!("Google API key is required"))?;
                
                let model = config
                    .model
                    .as_ref()
                    .unwrap_or(&"gemini-pro".to_string())
                    .clone();
                
                // Note: Google provider not implemented yet - placeholder
                Err(anyhow!("Google provider not implemented yet"))
            }
            ProviderType::Local => {
                let model_path = config
                    .model_path
                    .as_ref()
                    .ok_or_else(|| anyhow!("Local model path is required"))?;
                
                let model = config
                    .model
                    .as_ref()
                    .unwrap_or(&"local-model".to_string())
                    .clone();
                
                let provider = LocalProvider::new(model_path.clone(), model)?;
                Ok(Arc::new(provider))
            }
            ProviderType::Azure => {
                let api_key = config
                    .api_key
                    .as_ref()
                    .ok_or_else(|| anyhow!("Azure API key is required"))?;
                
                let model = config
                    .model
                    .as_ref()
                    .unwrap_or(&"gpt-35-turbo".to_string())
                    .clone();
                
                let endpoint = config
                    .api_base
                    .as_ref()
                    .ok_or_else(|| anyhow!("Azure endpoint is required"))?;
                
                let provider = AzureProvider::new(
                    api_key.clone(),
                    model,
                    endpoint.clone(),
                    None, // deployment_name - could be extracted from options
                )?;
                Ok(Arc::new(provider))
            }
            ProviderType::Groq => {
                let api_key = config
                    .api_key
                    .as_ref()
                    .ok_or_else(|| anyhow!("Groq API key is required"))?;
                
                let model = config
                    .model
                    .as_ref()
                    .unwrap_or(&"mixtral-8x7b-32768".to_string())
                    .clone();
                
                let endpoint = config.api_base.clone();
                
                let provider = GroqProvider::new(api_key.clone(), model, endpoint)?;
                Ok(Arc::new(provider))
            }
        }
    }

    /// Create all providers from configuration
    pub fn create_all_providers(
        config: &MagrayConfig,
    ) -> Result<Vec<(String, Arc<dyn LlmProvider + Send + Sync>)>> {
        let mut providers = Vec::new();
        
        for (name, provider_config) in &config.ai.providers {
            match Self::create_provider(name, provider_config) {
                Ok(provider) => {
                    providers.push((name.clone(), provider));
                }
                Err(e) => {
                    tracing::warn!("Failed to create provider '{}': {}", name, e);
                }
            }
        }
        
        if providers.is_empty() {
            return Err(anyhow!("No valid providers could be created from configuration"));
        }
        
        Ok(providers)
    }

    /// Get the default provider from configuration
    pub fn get_default_provider(
        config: &MagrayConfig,
    ) -> Result<Arc<dyn LlmProvider + Send + Sync>> {
        let default_name = &config.ai.default_provider;
        
        let provider_config = config
            .ai
            .providers
            .get(default_name)
            .ok_or_else(|| {
                anyhow!(
                    "Default provider '{}' not found in configuration",
                    default_name
                )
            })?;
        
        Self::create_provider(default_name, provider_config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::config::*;
    use std::collections::HashMap;

    #[test]
    fn test_create_openai_provider() {
        let config = ProviderConfig {
            provider_type: ProviderType::OpenAI,
            api_key: Some("test-api-key".to_string()),
            api_base: Some("https://api.openai.com/v1".to_string()),
            model: Some("gpt-4o-mini".to_string()),
            model_path: None,
            options: HashMap::new(),
        };
        
        let provider = LlmProviderFactory::create_provider("openai", &config).expect("Operation failed - converted from unwrap()");
        assert_eq!(provider.id().provider_type, "openai");
        assert_eq!(provider.id().model, "gpt-4o-mini");
    }

    #[test]
    fn test_create_provider_missing_api_key() {
        let config = ProviderConfig {
            provider_type: ProviderType::OpenAI,
            api_key: None,
            api_base: Some("https://api.openai.com/v1".to_string()),
            model: Some("gpt-4o-mini".to_string()),
            model_path: None,
            options: HashMap::new(),
        };
        
        let result = LlmProviderFactory::create_provider("openai", &config);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("API key is required"));
    }

    #[test]
    fn test_get_default_provider() {
        let mut providers = HashMap::new();
        providers.insert("openai".to_string(), ProviderConfig {
            provider_type: ProviderType::OpenAI,
            api_key: Some("test-api-key".to_string()),
            api_base: Some("https://api.openai.com/v1".to_string()),
            model: Some("gpt-4o-mini".to_string()),
            model_path: None,
            options: HashMap::new(),
        });

        let config = MagrayConfig {
            ai: AiConfig {
                default_provider: "openai".to_string(),
                providers,
                ..Default::default()
            },
            ..Default::default()
        };

        let provider = LlmProviderFactory::get_default_provider(&config).expect("Operation failed - converted from unwrap()");
        assert_eq!(provider.id().provider_type, "openai");
    }
}