//! UnifiedAgent - Modern Clean Architecture Implementation
//! 
//! Этот модуль экспортирует UnifiedAgentV2 как основной UnifiedAgent.
//! Миграция на Clean Architecture завершена.

use crate::unified_agent_v2::UnifiedAgentV2;

// Direct export UnifiedAgentV2 as UnifiedAgent
pub type UnifiedAgent = UnifiedAgentV2;