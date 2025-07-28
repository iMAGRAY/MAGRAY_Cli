use anyhow::Result;
use std::collections::HashMap;
use tracing::{info, debug, warn};
use crate::{ExecutionContext, StepStatus};

// === State Machine ===

#[derive(Debug, Clone)]
pub enum ExecutionState {
    Created,
    Planning,
    Executing,
    Paused,
    Completed,
    Failed,
    Cancelled,
}

pub struct StateMachine {
    current_state: ExecutionState,
    state_history: Vec<StateTransition>,
}

#[derive(Debug, Clone)]
struct StateTransition {
    from: ExecutionState,
    to: ExecutionState,
    timestamp: chrono::DateTime<chrono::Utc>,
    reason: Option<String>,
}

impl StateMachine {
    pub fn new() -> Self {
        Self {
            current_state: ExecutionState::Created,
            state_history: Vec::new(),
        }
    }

    pub fn current_state(&self) -> &ExecutionState {
        &self.current_state
    }

    pub fn can_transition_to(&self, target: &ExecutionState) -> bool {
        use ExecutionState::*;
        
        match (&self.current_state, target) {
            // Из Created можно перейти в Planning
            (Created, Planning) => true,
            
            // Из Planning можно перейти в Executing или Failed
            (Planning, Executing) | (Planning, Failed) => true,
            
            // Из Executing можно перейти в любое состояние
            (Executing, Paused) | (Executing, Completed) | 
            (Executing, Failed) | (Executing, Cancelled) => true,
            
            // Из Paused можно возобновить выполнение или отменить
            (Paused, Executing) | (Paused, Cancelled) => true,
            
            // Из финальных состояний никуда нельзя перейти
            (Completed, _) | (Failed, _) | (Cancelled, _) => false,
            
            // Все остальные переходы запрещены
            _ => false,
        }
    }

    pub fn transition_to(&mut self, target: ExecutionState, reason: Option<String>) -> Result<()> {
        if !self.can_transition_to(&target) {
            return Err(anyhow::anyhow!(
                "Невозможен переход из {:?} в {:?}", 
                self.current_state, target
            ));
        }

        let transition = StateTransition {
            from: self.current_state.clone(),
            to: target.clone(),
            timestamp: chrono::Utc::now(),
            reason,
        };

        self.state_history.push(transition);
        self.current_state = target;
        
        debug!("🔄 Переход состояния: {:?} -> {:?}", 
               self.state_history.last().unwrap().from, 
               self.current_state);
        
        Ok(())
    }

    pub fn start_planning(&mut self) -> Result<()> {
        self.transition_to(ExecutionState::Planning, Some("Начало планирования".to_string()))
    }

    pub fn start_execution(&mut self) -> Result<()> {
        self.transition_to(ExecutionState::Executing, Some("Начало выполнения плана".to_string()))
    }

    pub fn pause(&mut self, reason: String) -> Result<()> {
        self.transition_to(ExecutionState::Paused, Some(reason))
    }

    pub fn resume(&mut self) -> Result<()> {
        self.transition_to(ExecutionState::Executing, Some("Возобновление выполнения".to_string()))
    }

    pub fn complete(&mut self) -> Result<()> {
        self.transition_to(ExecutionState::Completed, Some("Выполнение успешно завершено".to_string()))
    }

    pub fn fail(&mut self, error: String) -> Result<()> {
        self.transition_to(ExecutionState::Failed, Some(format!("Ошибка: {}", error)))
    }

    pub fn cancel(&mut self, reason: String) -> Result<()> {
        self.transition_to(ExecutionState::Cancelled, Some(reason))
    }

    pub fn is_final_state(&self) -> bool {
        matches!(self.current_state, 
                 ExecutionState::Completed | 
                 ExecutionState::Failed | 
                 ExecutionState::Cancelled)
    }

    pub fn is_running(&self) -> bool {
        matches!(self.current_state, ExecutionState::Executing)
    }

    pub fn get_history(&self) -> &[StateTransition] {
        &self.state_history
    }

    pub fn get_duration(&self) -> Option<chrono::Duration> {
        if let (Some(start), Some(end)) = (self.state_history.first(), self.state_history.last()) {
            Some(end.timestamp - start.timestamp)
        } else {
            None
        }
    }
}

// === Execution Monitor ===

pub struct ExecutionMonitor {
    state_machine: StateMachine,
    step_states: HashMap<String, StepStatus>,
    progress_callbacks: Vec<Box<dyn Fn(&ExecutionContext) + Send + Sync>>,
}

impl ExecutionMonitor {
    pub fn new() -> Self {
        Self {
            state_machine: StateMachine::new(),
            step_states: HashMap::new(),
            progress_callbacks: Vec::new(),
        }
    }

    pub fn get_state_machine(&self) -> &StateMachine {
        &self.state_machine
    }

    pub fn get_state_machine_mut(&mut self) -> &mut StateMachine {
        &mut self.state_machine
    }

    pub fn update_step_status(&mut self, step_id: String, status: StepStatus) {
        let old_status = self.step_states.get(&step_id).cloned();
        self.step_states.insert(step_id.clone(), status.clone());
        
        debug!("📊 Обновлен статус шага '{}': {:?} -> {:?}", 
               step_id, old_status, status);
        
        // Проверяем общий прогресс
        self.check_overall_progress();
    }

    fn check_overall_progress(&mut self) {
        let total_steps = self.step_states.len();
        if total_steps == 0 {
            return;
        }

        let completed = self.step_states.values()
            .filter(|&status| matches!(status, StepStatus::Completed))
            .count();
        
        let failed = self.step_states.values()
            .filter(|&status| matches!(status, StepStatus::Failed))
            .count();

        // Если есть провалившиеся шаги
        if failed > 0 {
            if let Err(e) = self.state_machine.fail(format!("{} шагов провалено", failed)) {
                warn!("Не удалось перейти в состояние Failed: {}", e);
            }
            return;
        }

        // Если все шаги завершены
        if completed == total_steps {
            if let Err(e) = self.state_machine.complete() {
                warn!("Не удалось перейти в состояние Completed: {}", e);
            }
            return;
        }

        let progress = (completed as f32 / total_steps as f32) * 100.0;
        debug!("📈 Прогресс выполнения: {:.1}% ({}/{})", progress, completed, total_steps);
    }

    pub fn add_progress_callback<F>(&mut self, callback: F) 
    where 
        F: Fn(&ExecutionContext) + Send + Sync + 'static 
    {
        self.progress_callbacks.push(Box::new(callback));
    }

    pub fn notify_progress(&self, context: &ExecutionContext) {
        for callback in &self.progress_callbacks {
            callback(context);
        }
    }

    pub fn get_statistics(&self) -> MonitorStatistics {
        let total = self.step_states.len();
        let mut stats = MonitorStatistics {
            total_steps: total,
            ..Default::default()
        };

        for status in self.step_states.values() {
            match status {
                StepStatus::Pending => stats.pending += 1,
                StepStatus::Running => stats.running += 1,
                StepStatus::Completed => stats.completed += 1,
                StepStatus::Failed => stats.failed += 1,
                StepStatus::Skipped => stats.skipped += 1,
            }
        }

        if let Some(duration) = self.state_machine.get_duration() {
            stats.execution_duration = Some(duration);
        }

        stats.current_state = self.state_machine.current_state().clone();
        stats
    }
}

#[derive(Debug, Default)]
pub struct MonitorStatistics {
    pub total_steps: usize,
    pub pending: usize,
    pub running: usize,
    pub completed: usize,
    pub failed: usize,
    pub skipped: usize,
    pub current_state: ExecutionState,
    pub execution_duration: Option<chrono::Duration>,
}

impl MonitorStatistics {
    pub fn progress_percentage(&self) -> f32 {
        if self.total_steps == 0 {
            0.0
        } else {
            (self.completed as f32 / self.total_steps as f32) * 100.0
        }
    }

    pub fn is_healthy(&self) -> bool {
        self.failed == 0 && !matches!(self.current_state, ExecutionState::Failed)
    }
}

impl Default for ExecutionState {
    fn default() -> Self {
        ExecutionState::Created
    }
}