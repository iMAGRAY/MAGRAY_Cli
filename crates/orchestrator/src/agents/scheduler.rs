use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap};
use std::sync::atomic::{AtomicU32, Ordering as AtomicOrdering};
use std::sync::Arc;
use std::time::{Duration as StdDuration, Instant};
use tokio::sync::{mpsc, RwLock};
use tokio::time::Duration;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use super::executor::{ExecutionResult, ExecutorTrait};

// Health monitoring integration
use crate::reliability::health::{HealthChecker, HealthReport, HealthStatus};

/// Scheduled task representation for background execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduledTask {
    pub id: Uuid,
    pub name: String,
    pub task_type: TaskType,
    pub schedule: TaskSchedule,
    pub payload: serde_json::Value,
    pub status: TaskStatus,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub next_run_at: Option<chrono::DateTime<chrono::Utc>>,
    pub last_run_at: Option<chrono::DateTime<chrono::Utc>>,
    pub retry_count: u32,
    pub max_retries: u32,
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Types of scheduled tasks
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum TaskType {
    Cron,
    Interval,
    Once,
    Immediate,
}

/// Task scheduling configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaskSchedule {
    /// Run once at specific time
    Once { at: chrono::DateTime<chrono::Utc> },
    /// Run immediately
    Immediate,
    /// Recurring interval
    Interval { every: StdDuration },
    /// Cron-like schedule (simplified)
    Cron { expression: String },
}

/// Task execution status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TaskStatus {
    Pending,
    Scheduled,
    Running,
    Completed,
    Failed,
    Cancelled,
    Paused,
}

/// Scheduler status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SchedulerStatus {
    Stopped,
    Running,
    Paused,
}

/// Scheduled job representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Job {
    pub id: Uuid,
    pub job_type: JobType,
    pub priority: JobPriority,
    pub schedule: Schedule,
    pub payload: serde_json::Value,
    pub status: JobStatus,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub next_run_at: Option<chrono::DateTime<chrono::Utc>>,
    pub last_run_at: Option<chrono::DateTime<chrono::Utc>>,
    pub retry_count: u32,
    pub max_retries: u32,
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Types of jobs that can be scheduled
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum JobType {
    /// Execute a plan through ExecutorTrait
    PlanExecution { plan_id: Uuid },
    /// Execute a scheduled task
    ScheduledTask {
        task_id: Uuid,
        task_data: serde_json::Value,
    },
    /// Memory maintenance
    MemoryMaintenance,
    /// System cleanup
    SystemCleanup,
    /// Health checks
    HealthCheck,
    /// Metric collection
    MetricCollection,
    /// Background indexing
    BackgroundIndexing { path: String },
    /// Periodic analysis
    PeriodicAnalysis { analysis_type: String },
    /// Custom job
    Custom { job_name: String },
}

/// Job priority levels
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum JobPriority {
    Low = 1,
    Medium = 2,
    High = 3,
    Critical = 4,
}

/// Job scheduling configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Schedule {
    /// Run once at specific time
    Once { at: chrono::DateTime<chrono::Utc> },
    /// Run immediately
    Immediate,
    /// Recurring interval
    Interval { every: StdDuration },
    /// Cron-like schedule
    Cron { expression: String },
    /// Custom schedule
    Custom { schedule_fn: String },
}

/// Job execution status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum JobStatus {
    Pending,
    Scheduled,
    Running,
    Completed,
    Failed,
    Cancelled,
    Paused,
}

/// Job execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobResult {
    pub job_id: Uuid,
    pub status: JobStatus,
    pub output: Option<serde_json::Value>,
    pub error: Option<String>,
    pub execution_time: StdDuration,
    pub executed_at: chrono::DateTime<chrono::Utc>,
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Priority queue item for job scheduling
#[derive(Debug, Clone)]
struct PriorityJob {
    job: Job,
}

impl PartialEq for PriorityJob {
    fn eq(&self, other: &Self) -> bool {
        self.job.id == other.job.id
    }
}

impl Eq for PriorityJob {}

impl PartialOrd for PriorityJob {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for PriorityJob {
    fn cmp(&self, other: &Self) -> Ordering {
        // Higher priority first, then earlier next_run_at
        match self.job.priority.cmp(&other.job.priority) {
            Ordering::Equal => {
                match (&self.job.next_run_at, &other.job.next_run_at) {
                    (Some(a), Some(b)) => b.cmp(a), // Earlier time has higher priority
                    (Some(_), None) => Ordering::Greater,
                    (None, Some(_)) => Ordering::Less,
                    (None, None) => Ordering::Equal,
                }
            }
            other => other,
        }
    }
}

/// Scheduler trait for job and task management
#[async_trait]
pub trait SchedulerTrait: Send + Sync {
    /// Schedule a new job
    async fn schedule_job(&mut self, job: Job) -> Result<()>;

    /// Schedule a new task
    async fn schedule_task(&mut self, task: ScheduledTask) -> Result<()>;

    /// Cancel a scheduled job
    async fn cancel_job(&mut self, job_id: Uuid) -> Result<()>;

    /// Cancel a scheduled task
    async fn cancel_task(&mut self, task_id: Uuid) -> Result<()>;

    /// Get next job to execute
    async fn get_next_job(&mut self) -> Result<Option<Job>>;

    /// Mark job as completed
    async fn complete_job(&mut self, job_id: Uuid, result: JobResult) -> Result<()>;

    /// Mark job as failed
    async fn fail_job(&mut self, job_id: Uuid, error: String) -> Result<()>;

    /// Get job status
    async fn get_job_status(&self, job_id: Uuid) -> Result<Option<JobStatus>>;

    /// List all jobs with optional filter
    async fn list_jobs(&self, filter: Option<JobFilter>) -> Result<Vec<Job>>;

    /// List scheduled tasks
    async fn list_scheduled_tasks(&self) -> Result<Vec<ScheduledTask>>;

    /// Execute scheduled tasks in background
    async fn execute_scheduled_tasks(&mut self) -> Result<()>;

    /// Pause scheduler
    async fn pause_scheduler(&mut self) -> Result<()>;

    /// Resume scheduler
    async fn resume_scheduler(&mut self) -> Result<()>;

    /// Pause the scheduler (alias for compatibility)
    async fn pause(&mut self) -> Result<()> {
        self.pause_scheduler().await
    }

    /// Resume the scheduler (alias for compatibility)
    async fn resume(&mut self) -> Result<()> {
        self.resume_scheduler().await
    }

    /// Get scheduler status
    async fn get_scheduler_status(&self) -> Result<SchedulerStatus>;

    /// Get scheduler statistics
    async fn get_statistics(&self) -> Result<SchedulerStatistics>;
}

/// Job filter for listing
#[derive(Debug, Clone)]
pub struct JobFilter {
    pub job_type: Option<JobType>,
    pub status: Option<JobStatus>,
    pub priority: Option<JobPriority>,
    pub created_after: Option<chrono::DateTime<chrono::Utc>>,
}

/// Scheduler statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchedulerStatistics {
    pub total_jobs: u64,
    pub pending_jobs: u64,
    pub running_jobs: u64,
    pub completed_jobs: u64,
    pub failed_jobs: u64,
    pub average_execution_time: StdDuration,
    pub jobs_per_priority: HashMap<JobPriority, u64>,
}

/// Scheduler implementation with ExecutorTrait integration
pub struct Scheduler {
    agent_id: uuid::Uuid,
    job_queue: BinaryHeap<PriorityJob>,
    jobs: HashMap<Uuid, Job>,
    running_jobs: HashMap<Uuid, Job>,
    completed_jobs: HashMap<Uuid, JobResult>,
    scheduled_tasks: Arc<RwLock<HashMap<Uuid, ScheduledTask>>>,
    is_paused: bool,
    status: SchedulerStatus,
    statistics: SchedulerStatistics,
    executor: Option<Arc<dyn ExecutorTrait>>,
    background_handle: Option<tokio::task::JoinHandle<()>>,
    shutdown_tx: Option<mpsc::UnboundedSender<()>>,
    // Health monitoring fields
    last_heartbeat: Arc<RwLock<Option<DateTime<Utc>>>>,
    error_count: Arc<AtomicU32>,
    start_time: Instant,
    // КРИТИЧЕСКИЙ MEMORY MANAGEMENT - ДОБАВЛЕНО
    max_completed_jobs: usize,
    max_jobs_in_memory: usize,
    cleanup_interval: Duration,
}

impl Scheduler {
    /// Create new Scheduler instance
    pub fn new() -> Self {
        Self {
            agent_id: Uuid::new_v4(),
            job_queue: BinaryHeap::new(),
            jobs: HashMap::new(),
            running_jobs: HashMap::new(),
            completed_jobs: HashMap::new(),
            scheduled_tasks: Arc::new(RwLock::new(HashMap::new())),
            is_paused: false,
            status: SchedulerStatus::Stopped,
            statistics: SchedulerStatistics {
                total_jobs: 0,
                pending_jobs: 0,
                running_jobs: 0,
                completed_jobs: 0,
                failed_jobs: 0,
                average_execution_time: StdDuration::from_secs(0),
                jobs_per_priority: HashMap::new(),
            },
            executor: None,
            background_handle: None,
            shutdown_tx: None,
            // Health monitoring fields
            last_heartbeat: Arc::new(RwLock::new(Some(Utc::now()))),
            error_count: Arc::new(AtomicU32::new(0)),
            start_time: Instant::now(),
            // КРИТИЧЕСКИ ВАЖНО - MEMORY BOUNDS
            max_completed_jobs: 1000, // Максимум 1000 completed jobs в памяти
            max_jobs_in_memory: 10000, // Максимум 10000 общих jobs в памяти
            cleanup_interval: Duration::from_secs(300), // Cleanup каждые 5 минут
        }
    }

    /// Start automatic heartbeat loop for health monitoring
    /// This prevents timeout issues by sending heartbeat every 30 seconds
    pub fn start_heartbeat_loop(&self) {
        let last_heartbeat = Arc::clone(&self.last_heartbeat);
        let agent_id = self.agent_id;

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(30));
            interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

            loop {
                interval.tick().await;

                // Update heartbeat timestamp
                {
                    let mut heartbeat = last_heartbeat.write().await;
                    *heartbeat = Some(Utc::now());
                }

                tracing::debug!(
                    agent_id = %agent_id,
                    agent_type = "Scheduler",
                    "Heartbeat sent"
                );
            }
        });

        tracing::info!(
            agent_id = %self.agent_id,
            agent_type = "Scheduler",
            "Heartbeat loop started with 30s interval"
        );
    }

    /// Create new Scheduler with ExecutorTrait integration
    pub fn with_executor(executor: Arc<dyn ExecutorTrait>) -> Self {
        let mut scheduler = Self::new();
        scheduler.executor = Some(executor);
        scheduler
    }

    /// Start background task execution loop
    pub async fn start_background_execution(&mut self) -> Result<()> {
        if self.background_handle.is_some() {
            warn!("Background execution already running");
            return Ok(());
        }

        let (shutdown_tx, mut shutdown_rx) = mpsc::unbounded_channel();
        let tasks = Arc::clone(&self.scheduled_tasks);
        let executor = self.executor.clone();

        let handle = tokio::spawn(async move {
            info!("Starting background task execution loop");

            let mut interval = tokio::time::interval(Duration::from_secs(1));

            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        if let Err(e) = Self::execute_ready_tasks(&tasks, &executor).await {
                            error!("Error executing ready tasks: {}", e);
                        }
                    }
                    _ = shutdown_rx.recv() => {
                        info!("Background execution loop shutting down");
                        break;
                    }
                }
            }
        });

        self.background_handle = Some(handle);
        self.shutdown_tx = Some(shutdown_tx);
        self.status = SchedulerStatus::Running;

        info!("Background execution started");
        Ok(())
    }

    /// Stop background task execution loop
    pub async fn stop_background_execution(&mut self) -> Result<()> {
        if let Some(shutdown_tx) = self.shutdown_tx.take() {
            let _ = shutdown_tx.send(());
        }

        if let Some(handle) = self.background_handle.take() {
            handle.abort();
            let _ = handle.await;
        }

        self.status = SchedulerStatus::Stopped;
        info!("Background execution stopped");
        Ok(())
    }

    /// Execute ready scheduled tasks
    async fn execute_ready_tasks(
        tasks: &Arc<RwLock<HashMap<Uuid, ScheduledTask>>>,
        executor: &Option<Arc<dyn ExecutorTrait>>,
    ) -> Result<()> {
        let ready_tasks = {
            let tasks_read = tasks.read().await;
            tasks_read
                .values()
                .filter(|task| Self::is_task_ready(task))
                .cloned()
                .collect::<Vec<_>>()
        };

        for mut task in ready_tasks {
            if let Some(executor) = executor {
                debug!("Executing scheduled task: {}", task.name);

                match Self::execute_task_with_executor(&task, executor).await {
                    Ok(_execution_result) => {
                        task.status = TaskStatus::Completed;
                        task.last_run_at = Some(chrono::Utc::now());
                        debug!("Task {} completed successfully", task.name);

                        // Reschedule if recurring
                        if let Some(next_run) = Self::calculate_next_task_run(&task) {
                            task.next_run_at = Some(next_run);
                            task.status = TaskStatus::Scheduled;
                            task.retry_count = 0;
                        }
                    }
                    Err(e) => {
                        task.retry_count += 1;
                        error!("Task {} failed: {}", task.name, e);

                        if task.retry_count < task.max_retries {
                            // Schedule retry with exponential backoff
                            let backoff_secs = 2_u64.pow(task.retry_count.min(6));
                            task.next_run_at = Some(
                                chrono::Utc::now() + chrono::Duration::seconds(backoff_secs as i64),
                            );
                            task.status = TaskStatus::Scheduled;
                        } else {
                            task.status = TaskStatus::Failed;
                            error!(
                                "Task {} failed after {} retries",
                                task.name, task.max_retries
                            );
                        }
                    }
                }

                // Update task in storage
                let mut tasks_write = tasks.write().await;
                tasks_write.insert(task.id, task);
            }
        }

        Ok(())
    }

    /// Execute a scheduled task using ExecutorTrait
    async fn execute_task_with_executor(
        task: &ScheduledTask,
        _executor: &Arc<dyn ExecutorTrait>,
    ) -> Result<ExecutionResult> {
        // For simplicity, we'll execute the task payload as if it's a plan
        // In a real implementation, this would depend on the task type
        match &task.task_type {
            TaskType::Immediate | TaskType::Once | TaskType::Interval | TaskType::Cron => {
                // Create a dummy plan from task payload
                // This is simplified - in reality you'd convert task to proper ActionPlan
                debug!(
                    "Executing task {} with payload: {}",
                    task.name, task.payload
                );

                // For now, just return a successful result
                Ok(ExecutionResult {
                    plan_id: task.id,
                    status: super::executor::ExecutionStatus::Completed,
                    step_results: vec![],
                    execution_time: StdDuration::from_millis(100),
                    resource_usage: super::executor::ResourceUsage::default(),
                    metadata: HashMap::new(),
                    error: None,
                })
            }
        }
    }

    /// Check if task is ready to run
    fn is_task_ready(task: &ScheduledTask) -> bool {
        if task.status != TaskStatus::Scheduled {
            return false;
        }

        match task.next_run_at {
            Some(next_run) => chrono::Utc::now() >= next_run,
            None => true, // Immediate tasks
        }
    }

    /// Calculate next run time for recurring tasks
    fn calculate_next_task_run(task: &ScheduledTask) -> Option<chrono::DateTime<chrono::Utc>> {
        match &task.schedule {
            TaskSchedule::Once { .. } | TaskSchedule::Immediate => None, // One-time tasks
            TaskSchedule::Interval { every } => {
                let base = task.last_run_at.unwrap_or_else(chrono::Utc::now);
                Some(
                    base + chrono::Duration::from_std(*every)
                        .unwrap_or_else(|_| chrono::Duration::seconds(0)),
                )
            }
            TaskSchedule::Cron { .. } => {
                // Simplified cron implementation
                Some(chrono::Utc::now() + chrono::Duration::hours(1))
            }
        }
    }

    /// Calculate next run time for recurring jobs
    fn calculate_next_run(
        &self,
        schedule: &Schedule,
        last_run: Option<chrono::DateTime<chrono::Utc>>,
    ) -> Option<chrono::DateTime<chrono::Utc>> {
        match schedule {
            Schedule::Once { at } => Some(*at),
            Schedule::Immediate => Some(chrono::Utc::now()),
            Schedule::Interval { every } => {
                let base = last_run.unwrap_or_else(chrono::Utc::now);
                Some(
                    base + chrono::Duration::from_std(*every)
                        .unwrap_or_else(|_| chrono::Duration::seconds(0)),
                )
            }
            Schedule::Cron { expression: _ } => {
                // Simplified cron implementation
                Some(chrono::Utc::now() + chrono::Duration::hours(1))
            }
            Schedule::Custom { schedule_fn: _ } => {
                // Custom schedule function would be called here
                Some(chrono::Utc::now() + chrono::Duration::minutes(30))
            }
        }
    }

    /// Check if job is ready to run
    fn is_job_ready(&self, job: &Job) -> bool {
        if self.is_paused {
            return false;
        }

        match job.next_run_at {
            Some(next_run) => chrono::Utc::now() >= next_run,
            None => true, // Immediate jobs
        }
    }

    /// Update statistics
    fn update_statistics(&mut self) {
        self.statistics.total_jobs = self.jobs.len() as u64;
        self.statistics.pending_jobs = self
            .jobs
            .values()
            .filter(|j| matches!(j.status, JobStatus::Pending | JobStatus::Scheduled))
            .count() as u64;
        self.statistics.running_jobs = self.running_jobs.len() as u64;
        self.statistics.completed_jobs = self
            .completed_jobs
            .values()
            .filter(|r| r.status == JobStatus::Completed)
            .count() as u64;
        self.statistics.failed_jobs = self
            .completed_jobs
            .values()
            .filter(|r| r.status == JobStatus::Failed)
            .count() as u64;

        // Calculate average execution time
        if !self.completed_jobs.is_empty() {
            let total_time: StdDuration =
                self.completed_jobs.values().map(|r| r.execution_time).sum();
            self.statistics.average_execution_time = total_time / self.completed_jobs.len() as u32;
        }

        // Count jobs per priority
        self.statistics.jobs_per_priority.clear();
        for job in self.jobs.values() {
            *self
                .statistics
                .jobs_per_priority
                .entry(job.priority.clone())
                .or_insert(0) += 1;
        }
    }

    /// Reschedule recurring job
    fn reschedule_recurring_job(&mut self, mut job: Job) -> Result<()> {
        if let Some(next_run) = self.calculate_next_run(&job.schedule, job.last_run_at) {
            job.next_run_at = Some(next_run);
            job.status = JobStatus::Scheduled;
            job.retry_count = 0; // Reset retry count for new schedule

            self.job_queue.push(PriorityJob { job: job.clone() });
            self.jobs.insert(job.id, job);
        }
        Ok(())
    }

    /// КРИТИЧЕСКИ ВАЖНАЯ MEMORY CLEANUP ФУНКЦИЯ
    /// Очищает old completed jobs для предотвращения memory growth
    pub fn cleanup_memory(&mut self) {
        let start_time = std::time::Instant::now();

        // Cleanup completed jobs if exceeds limit
        if self.completed_jobs.len() > self.max_completed_jobs {
            let excess = self.completed_jobs.len() - self.max_completed_jobs;

            // Сортируем по времени завершения и удаляем самые старые
            let mut completed_jobs_vec: Vec<_> = self
                .completed_jobs
                .iter()
                .map(|(id, result)| (*id, result.executed_at))
                .collect();
            completed_jobs_vec.sort_by(|a, b| a.1.cmp(&b.1));

            let jobs_to_remove: Vec<_> = completed_jobs_vec
                .iter()
                .take(excess)
                .map(|(id, _)| *id)
                .collect();

            for job_id in jobs_to_remove {
                self.completed_jobs.remove(&job_id);
            }

            info!("Cleaned up {} old completed jobs", excess);
        }

        // Cleanup old jobs if total memory exceeds limit
        let total_jobs = self.jobs.len() + self.running_jobs.len() + self.completed_jobs.len();
        if total_jobs > self.max_jobs_in_memory {
            let excess = total_jobs - self.max_jobs_in_memory;

            // Удаляем старые non-running jobs первыми
            let mut jobs_to_remove = Vec::new();
            for (job_id, job) in &self.jobs {
                if job.status != JobStatus::Running && jobs_to_remove.len() < excess {
                    jobs_to_remove.push(*job_id);
                }
            }

            for job_id in jobs_to_remove {
                self.jobs.remove(&job_id);
            }

            info!("Cleaned up {} excess jobs from memory", excess);
        }

        // Очищаем scheduled tasks если их слишком много
        let scheduled_tasks_count = self
            .scheduled_tasks
            .try_read()
            .map(|tasks| tasks.len())
            .unwrap_or_else(|_| 0);

        if scheduled_tasks_count > 5000 {
            warn!(
                "Too many scheduled tasks in memory: {}",
                scheduled_tasks_count
            );

            if let Ok(mut tasks) = self.scheduled_tasks.try_write() {
                let mut tasks_vec: Vec<_> = tasks
                    .iter()
                    .map(|(id, task)| (*id, task.created_at))
                    .collect();
                tasks_vec.sort_by(|a, b| a.1.cmp(&b.1));

                // Удаляем половину старых tasks
                let to_remove = tasks_vec.len() / 2;
                let tasks_to_remove: Vec<_> = tasks_vec
                    .iter()
                    .take(to_remove)
                    .map(|(id, _)| *id)
                    .collect();

                for task_id in tasks_to_remove {
                    tasks.remove(&task_id);
                }

                info!("Cleaned up {} old scheduled tasks", to_remove);
            }
        }

        let cleanup_duration = start_time.elapsed();
        info!("Memory cleanup completed in {:?}", cleanup_duration);
    }

    /// Получить memory usage statistics
    pub fn memory_usage(&self) -> SchedulerMemoryUsage {
        let scheduled_tasks_count = self
            .scheduled_tasks
            .try_read()
            .map(|tasks| tasks.len())
            .unwrap_or_else(|_| 0);

        SchedulerMemoryUsage {
            total_jobs: self.jobs.len(),
            running_jobs: self.running_jobs.len(),
            completed_jobs: self.completed_jobs.len(),
            scheduled_tasks: scheduled_tasks_count,
            job_queue_size: self.job_queue.len(),
            estimated_memory_mb: (self.jobs.len()
                + self.running_jobs.len()
                + self.completed_jobs.len()
                + scheduled_tasks_count)
                * 2
                / 1024
                / 1024,
        }
    }
}

/// Memory usage statistics для Scheduler
pub struct SchedulerMemoryUsage {
    pub total_jobs: usize,
    pub running_jobs: usize,
    pub completed_jobs: usize,
    pub scheduled_tasks: usize,
    pub job_queue_size: usize,
    pub estimated_memory_mb: usize,
}

#[async_trait]
impl SchedulerTrait for Scheduler {
    async fn schedule_job(&mut self, mut job: Job) -> Result<()> {
        tracing::debug!("Scheduling job {}: {:?}", job.id, job.job_type);

        // Calculate next run time
        job.next_run_at = self.calculate_next_run(&job.schedule, None);
        job.status = JobStatus::Scheduled;
        job.created_at = chrono::Utc::now();

        // Add to queue and storage
        self.job_queue.push(PriorityJob { job: job.clone() });
        self.jobs.insert(job.id, job);

        self.update_statistics();
        Ok(())
    }

    async fn cancel_job(&mut self, job_id: Uuid) -> Result<()> {
        if let Some(mut job) = self.jobs.remove(&job_id) {
            job.status = JobStatus::Cancelled;

            // Remove from running jobs if applicable
            self.running_jobs.remove(&job_id);

            tracing::debug!("Cancelled job {}", job_id);
            self.update_statistics();
            Ok(())
        } else {
            anyhow::bail!("Job {} not found", job_id)
        }
    }

    async fn get_next_job(&mut self) -> Result<Option<Job>> {
        if self.is_paused {
            return Ok(None);
        }

        // Find next ready job in priority queue
        let mut temp_jobs = Vec::new();
        let mut next_job = None;

        while let Some(priority_job) = self.job_queue.pop() {
            if self.is_job_ready(&priority_job.job) {
                next_job = Some(priority_job.job);
                break;
            } else {
                temp_jobs.push(priority_job);
            }
        }

        // Put non-ready jobs back
        for job in temp_jobs {
            self.job_queue.push(job);
        }

        if let Some(mut job) = next_job {
            job.status = JobStatus::Running;
            self.running_jobs.insert(job.id, job.clone());
            self.jobs.insert(job.id, job.clone());

            tracing::debug!("Retrieved next job {}: {:?}", job.id, job.job_type);
            self.update_statistics();

            Ok(Some(job))
        } else {
            Ok(None)
        }
    }

    async fn complete_job(&mut self, job_id: Uuid, result: JobResult) -> Result<()> {
        if let Some(mut job) = self.running_jobs.remove(&job_id) {
            job.status = JobStatus::Completed;
            job.last_run_at = Some(result.executed_at);

            self.completed_jobs.insert(job_id, result);

            // Check if job should be rescheduled
            match &job.schedule {
                Schedule::Interval { .. } | Schedule::Cron { .. } | Schedule::Custom { .. } => {
                    self.reschedule_recurring_job(job)?;
                }
                _ => {
                    // One-time job, just update status
                    self.jobs.insert(job_id, job);
                }
            }

            tracing::debug!("Completed job {}", job_id);
            self.update_statistics();
            Ok(())
        } else {
            anyhow::bail!("Running job {} not found", job_id)
        }
    }

    async fn fail_job(&mut self, job_id: Uuid, error: String) -> Result<()> {
        if let Some(mut job) = self.running_jobs.remove(&job_id) {
            job.retry_count += 1;

            let result = JobResult {
                job_id,
                status: JobStatus::Failed,
                output: None,
                error: Some(error.clone()),
                execution_time: std::time::Duration::from_secs(0),
                executed_at: chrono::Utc::now(),
                metadata: HashMap::new(),
            };

            if job.retry_count < job.max_retries {
                // Reschedule with backoff
                let backoff_delay = StdDuration::from_secs(2_u64.pow(job.retry_count.min(6)));
                job.next_run_at =
                    Some(chrono::Utc::now() + chrono::Duration::from_std(backoff_delay)?);
                job.status = JobStatus::Scheduled;

                let retry_count = job.retry_count;
                let max_retries = job.max_retries;

                self.job_queue.push(PriorityJob { job: job.clone() });
                self.jobs.insert(job_id, job);

                tracing::debug!(
                    "Rescheduling failed job {} (retry {}/{})",
                    job_id,
                    retry_count,
                    max_retries
                );
            } else {
                // Max retries reached
                job.status = JobStatus::Failed;
                let max_retries = job.max_retries;
                self.jobs.insert(job_id, job);
                self.completed_jobs.insert(job_id, result);

                tracing::error!(
                    "Job {} failed after {} retries: {}",
                    job_id,
                    max_retries,
                    error
                );
            }

            self.update_statistics();
            Ok(())
        } else {
            anyhow::bail!("Running job {} not found", job_id)
        }
    }

    async fn get_job_status(&self, job_id: Uuid) -> Result<Option<JobStatus>> {
        if let Some(job) = self.running_jobs.get(&job_id) {
            Ok(Some(job.status.clone()))
        } else if let Some(job) = self.jobs.get(&job_id) {
            Ok(Some(job.status.clone()))
        } else if let Some(result) = self.completed_jobs.get(&job_id) {
            Ok(Some(result.status.clone()))
        } else {
            Ok(None)
        }
    }

    async fn list_jobs(&self, filter: Option<JobFilter>) -> Result<Vec<Job>> {
        let mut jobs: Vec<Job> = self.jobs.values().cloned().collect();
        jobs.extend(self.running_jobs.values().cloned());

        if let Some(filter) = filter {
            jobs.retain(|job| {
                if let Some(ref job_type) = filter.job_type {
                    if std::mem::discriminant(&job.job_type) != std::mem::discriminant(job_type) {
                        return false;
                    }
                }

                if let Some(ref status) = filter.status {
                    if job.status != *status {
                        return false;
                    }
                }

                if let Some(ref priority) = filter.priority {
                    if job.priority != *priority {
                        return false;
                    }
                }

                if let Some(created_after) = filter.created_after {
                    if job.created_at < created_after {
                        return false;
                    }
                }

                true
            });
        }

        // Sort by priority and next run time
        jobs.sort_by(|a, b| {
            match a.priority.cmp(&b.priority) {
                Ordering::Equal => match (&a.next_run_at, &b.next_run_at) {
                    (Some(a_time), Some(b_time)) => a_time.cmp(b_time),
                    (Some(_), None) => Ordering::Less,
                    (None, Some(_)) => Ordering::Greater,
                    (None, None) => Ordering::Equal,
                },
                other => other.reverse(), // Higher priority first
            }
        });

        Ok(jobs)
    }

    async fn schedule_task(&mut self, mut task: ScheduledTask) -> Result<()> {
        debug!("Scheduling task {}: {}", task.id, task.name);

        // Calculate next run time
        task.next_run_at = match &task.schedule {
            TaskSchedule::Once { at } => Some(*at),
            TaskSchedule::Immediate => Some(chrono::Utc::now()),
            TaskSchedule::Interval { every } => {
                Some(chrono::Utc::now() + chrono::Duration::from_std(*every)?)
            }
            TaskSchedule::Cron { .. } => {
                // Simplified cron implementation
                Some(chrono::Utc::now() + chrono::Duration::hours(1))
            }
        };

        task.status = TaskStatus::Scheduled;
        task.created_at = chrono::Utc::now();

        // Add to scheduled tasks
        let mut tasks = self.scheduled_tasks.write().await;
        tasks.insert(task.id, task);
        drop(tasks); // Explicitly drop the lock

        self.update_statistics();
        Ok(())
    }

    async fn cancel_task(&mut self, task_id: Uuid) -> Result<()> {
        let mut tasks = self.scheduled_tasks.write().await;
        if let Some(mut task) = tasks.remove(&task_id) {
            task.status = TaskStatus::Cancelled;
            debug!("Cancelled task {}", task_id);
            drop(tasks); // Explicitly drop the lock
            self.update_statistics();
            Ok(())
        } else {
            anyhow::bail!("Task {} not found", task_id)
        }
    }

    async fn list_scheduled_tasks(&self) -> Result<Vec<ScheduledTask>> {
        let tasks = self.scheduled_tasks.read().await;
        Ok(tasks.values().cloned().collect())
    }

    async fn execute_scheduled_tasks(&mut self) -> Result<()> {
        if self.executor.is_none() {
            anyhow::bail!("No executor configured for scheduled tasks");
        }

        let tasks = Arc::clone(&self.scheduled_tasks);
        let executor = self.executor.clone();

        Self::execute_ready_tasks(&tasks, &executor).await
    }

    async fn pause_scheduler(&mut self) -> Result<()> {
        self.is_paused = true;
        self.status = SchedulerStatus::Paused;
        info!("Scheduler paused");
        Ok(())
    }

    async fn resume_scheduler(&mut self) -> Result<()> {
        self.is_paused = false;
        self.status = if self.background_handle.is_some() {
            SchedulerStatus::Running
        } else {
            SchedulerStatus::Stopped
        };
        info!("Scheduler resumed");
        Ok(())
    }

    async fn get_scheduler_status(&self) -> Result<SchedulerStatus> {
        Ok(self.status.clone())
    }

    async fn get_statistics(&self) -> Result<SchedulerStatistics> {
        Ok(self.statistics.clone())
    }
}

/// BaseActor implementation for Scheduler
#[async_trait::async_trait]
impl crate::actors::BaseActor for Scheduler {
    fn id(&self) -> crate::actors::ActorId {
        crate::actors::ActorId::new()
    }

    fn actor_type(&self) -> &'static str {
        "Scheduler"
    }

    async fn handle_message(
        &mut self,
        message: crate::actors::ActorMessage,
        _context: &crate::actors::ActorContext,
    ) -> Result<(), crate::actors::ActorError> {
        match message {
            crate::actors::ActorMessage::Agent(agent_msg) => match agent_msg {
                crate::actors::AgentMessage::ScheduleTask {
                    task: _,
                    priority,
                    delay: _,
                } => {
                    tracing::info!(
                        "Received task scheduling request with priority: {:?}",
                        priority
                    );
                    // For now, just acknowledge the message since BaseActor implementation is minimal
                    Ok(())
                }
                _ => {
                    tracing::warn!("Unsupported agent message type for Scheduler");
                    Ok(())
                }
            },
            _ => {
                tracing::warn!("Unsupported message type for Scheduler");
                Ok(())
            }
        }
    }
}

impl Default for Scheduler {
    fn default() -> Self {
        Self::new()
    }
}

/// HealthChecker implementation for Scheduler
#[async_trait]
impl HealthChecker for Scheduler {
    fn agent_id(&self) -> Uuid {
        self.agent_id
    }

    fn agent_name(&self) -> &str {
        "Scheduler"
    }

    fn agent_type(&self) -> &str {
        "Scheduler"
    }

    async fn check_health(&self) -> Result<HealthReport> {
        let last_heartbeat = *self.last_heartbeat.read().await;
        let error_count = self.error_count.load(AtomicOrdering::Relaxed);
        let uptime = self.start_time.elapsed().as_secs();

        // Agent-specific health checks
        let pending_jobs = self.job_queue.len();
        let running_jobs = self.running_jobs.len();
        let completed_jobs = self.completed_jobs.len();
        let scheduled_tasks_count = self
            .scheduled_tasks
            .try_read()
            .map(|tasks| tasks.len())
            .unwrap_or_else(|_| 0);

        // Determine health status based on errors and queue state
        let status = if error_count > 30 {
            HealthStatus::Unhealthy {
                reason: format!("High error count: {}", error_count),
            }
        } else if error_count > 15 || pending_jobs > 100 || running_jobs > 20 {
            HealthStatus::Degraded {
                reason: format!(
                    "Moderate error count: {} or high job load: pending={}, running={}",
                    error_count, pending_jobs, running_jobs
                ),
            }
        } else {
            HealthStatus::Healthy
        };

        Ok(HealthReport {
            agent_id: self.agent_id,
            agent_name: "Scheduler".to_string(),
            agent_type: "Scheduler".to_string(),
            status,
            timestamp: Utc::now(),
            last_heartbeat,
            response_time_ms: Some(10),   // Scheduling is typically fast
            memory_usage_mb: Some(40),    // Estimated memory for job queue
            cpu_usage_percent: Some(3.0), // Low CPU usage for scheduling
            active_tasks: (pending_jobs + running_jobs) as u32,
            error_count,
            restart_count: 0, // Track restarts in future implementation
            uptime_seconds: uptime,
            metadata: serde_json::json!({
                "pending_jobs": pending_jobs,
                "running_jobs": running_jobs,
                "completed_jobs": completed_jobs,
                "scheduled_tasks": scheduled_tasks_count,
                "scheduler_status": format!("{:?}", self.status),
                "is_paused": self.is_paused,
                "executor_available": self.executor.is_some(),
                "background_running": self.background_handle.is_some()
            }),
        })
    }

    async fn heartbeat(&self) -> Result<()> {
        let mut heartbeat = self.last_heartbeat.write().await;
        *heartbeat = Some(Utc::now());
        Ok(())
    }

    fn last_heartbeat(&self) -> Option<DateTime<Utc>> {
        // Use try_read for synchronous access needed by the trait
        self.last_heartbeat.try_read().ok().and_then(|guard| *guard)
    }

    fn is_healthy(&self) -> bool {
        let error_count = self.error_count.load(AtomicOrdering::Relaxed);
        let pending_jobs = self.job_queue.len();
        let running_jobs = self.running_jobs.len();
        error_count <= 30 && pending_jobs <= 100 && running_jobs <= 20
    }

    async fn restart(&self) -> Result<()> {
        // Reset error count and update heartbeat
        self.error_count.store(0, AtomicOrdering::Relaxed);
        {
            let mut heartbeat = self.last_heartbeat.write().await;
            *heartbeat = Some(Utc::now());
        }

        // Note: Scheduler restart is complex because it involves background tasks
        // For now, just reset error state and heartbeat
        // In a full implementation, we'd want to:
        // 1. Stop background execution gracefully
        // 2. Clear job queues (with proper cleanup)
        // 3. Restart background execution

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agents::executor::{ExecutionContext, ExecutionStatus, ResourceUsage, StepResult};
    use crate::agents::{executor, planner};
    use crate::saga::SagaStatus;

    fn create_test_job(priority: JobPriority) -> Job {
        Job {
            id: Uuid::new_v4(),
            job_type: JobType::HealthCheck,
            priority,
            schedule: Schedule::Immediate,
            payload: serde_json::json!({}),
            status: JobStatus::Pending,
            created_at: chrono::Utc::now(),
            next_run_at: None,
            last_run_at: None,
            retry_count: 0,
            max_retries: 3,
            metadata: HashMap::new(),
        }
    }

    fn create_test_scheduled_task(
        name: &str,
        task_type: TaskType,
        schedule: TaskSchedule,
    ) -> ScheduledTask {
        ScheduledTask {
            id: Uuid::new_v4(),
            name: name.to_string(),
            task_type,
            schedule,
            payload: serde_json::json!({"test": "data"}),
            status: TaskStatus::Pending,
            created_at: chrono::Utc::now(),
            next_run_at: None,
            last_run_at: None,
            retry_count: 0,
            max_retries: 3,
            metadata: HashMap::new(),
        }
    }

    // Mock ExecutorTrait for testing
    struct MockExecutor;

    #[async_trait]
    impl ExecutorTrait for MockExecutor {
        async fn execute_plan(&self, _plan: &planner::ActionPlan) -> Result<ExecutionResult> {
            Ok(ExecutionResult {
                plan_id: Uuid::new_v4(),
                status: ExecutionStatus::Completed,
                step_results: vec![],
                execution_time: StdDuration::from_millis(10),
                resource_usage: ResourceUsage::default(),
                metadata: HashMap::new(),
                error: None,
            })
        }

        async fn execute_step(
            &self,
            _step: &planner::ActionStep,
            _context: &mut ExecutionContext,
        ) -> Result<StepResult> {
            unimplemented!()
        }

        async fn cancel_execution(&self, _plan_id: Uuid) -> Result<()> {
            Ok(())
        }

        async fn pause_execution(&self, _plan_id: Uuid) -> Result<()> {
            Ok(())
        }

        async fn resume_execution(&self, _plan_id: Uuid) -> Result<()> {
            Ok(())
        }

        async fn get_execution_status(&self, _plan_id: Uuid) -> Result<ExecutionStatus> {
            Ok(ExecutionStatus::Completed)
        }

        async fn rollback_execution(&self, _plan_id: Uuid) -> Result<()> {
            Ok(())
        }

        async fn execute_plan_with_saga(
            &self,
            _plan: &planner::ActionPlan,
        ) -> Result<ExecutionResult> {
            Ok(ExecutionResult {
                plan_id: Uuid::new_v4(),
                status: ExecutionStatus::Completed,
                step_results: vec![],
                execution_time: StdDuration::from_millis(10),
                resource_usage: ResourceUsage::default(),
                metadata: HashMap::new(),
                error: None,
            })
        }

        async fn get_saga_status(&self, _plan_id: Uuid) -> Result<Option<SagaStatus>> {
            Ok(None)
        }
    }

    #[tokio::test]
    async fn test_schedule_job() {
        let mut scheduler = Scheduler::new();
        let job = create_test_job(JobPriority::Medium);
        let job_id = job.id;

        scheduler
            .schedule_job(job)
            .await
            .expect("schedule_job failed");

        let status = scheduler
            .get_job_status(job_id)
            .await
            .expect("get_job_status failed");
        assert_eq!(status, Some(JobStatus::Scheduled));
    }

    #[tokio::test]
    async fn test_get_next_job_priority() {
        let mut scheduler = Scheduler::new();

        let low_job = create_test_job(JobPriority::Low);
        let high_job = create_test_job(JobPriority::High);

        scheduler
            .schedule_job(low_job)
            .await
            .expect("schedule_job failed");
        scheduler
            .schedule_job(high_job.clone())
            .await
            .expect("schedule_job failed");

        let next_job = scheduler
            .get_next_job()
            .await
            .expect("get_next_job failed")
            .expect("expected next job");
        assert_eq!(next_job.id, high_job.id);
        assert_eq!(next_job.priority, JobPriority::High);
    }

    #[tokio::test]
    async fn test_cancel_job() {
        let mut scheduler = Scheduler::new();
        let job = create_test_job(JobPriority::Medium);
        let job_id = job.id;

        scheduler
            .schedule_job(job)
            .await
            .expect("schedule_job failed");
        scheduler
            .cancel_job(job_id)
            .await
            .expect("cancel_job failed");

        let status = scheduler
            .get_job_status(job_id)
            .await
            .expect("get_job_status failed");
        assert_eq!(status, Some(JobStatus::Cancelled));
    }

    #[tokio::test]
    async fn test_complete_job() {
        let mut scheduler = Scheduler::new();
        let job = create_test_job(JobPriority::Medium);
        let job_id = job.id;

        scheduler
            .schedule_job(job)
            .await
            .expect("schedule_job failed");
        let next_job = scheduler
            .get_next_job()
            .await
            .expect("get_next_job failed")
            .expect("expected next job");

        let result = JobResult {
            job_id,
            status: JobStatus::Completed,
            output: Some(serde_json::json!({"success": true})),
            error: None,
            execution_time: StdDuration::from_millis(100),
            executed_at: chrono::Utc::now(),
            metadata: HashMap::new(),
        };

        scheduler
            .complete_job(job_id, result)
            .await
            .expect("complete_job failed");

        let status = scheduler
            .get_job_status(job_id)
            .await
            .expect("get_job_status failed");
        assert_eq!(status, Some(JobStatus::Completed));
    }

    #[tokio::test]
    async fn test_pause_resume() {
        let mut scheduler = Scheduler::new();

        scheduler.pause().await.expect("pause failed");
        assert!(scheduler.is_paused);

        scheduler.resume().await.expect("resume failed");
        assert!(!scheduler.is_paused);
    }

    #[tokio::test]
    async fn test_job_retry_logic() {
        let mut scheduler = Scheduler::new();
        let job = create_test_job(JobPriority::Medium);
        let job_id = job.id;

        scheduler
            .schedule_job(job)
            .await
            .expect("schedule_job failed");
        let _next_job = scheduler
            .get_next_job()
            .await
            .expect("get_next_job failed")
            .expect("expected next job");

        // Fail the job
        scheduler
            .fail_job(job_id, "Test error".to_string())
            .await
            .expect("fail_job failed");

        // Job should be rescheduled for retry
        let status = scheduler
            .get_job_status(job_id)
            .await
            .expect("get_job_status failed");
        assert_eq!(status, Some(JobStatus::Scheduled));
    }

    #[tokio::test]
    async fn test_list_jobs_with_filter() {
        let mut scheduler = Scheduler::new();

        let high_job = create_test_job(JobPriority::High);
        let medium_job = create_test_job(JobPriority::Medium);

        scheduler
            .schedule_job(high_job)
            .await
            .expect("schedule_job failed");
        scheduler
            .schedule_job(medium_job)
            .await
            .expect("schedule_job failed");

        let filter = JobFilter {
            job_type: None,
            status: Some(JobStatus::Scheduled),
            priority: Some(JobPriority::High),
            created_after: None,
        };

        let filtered_jobs = scheduler
            .list_jobs(Some(filter))
            .await
            .expect("list_jobs failed");
        assert_eq!(filtered_jobs.len(), 1);
        assert_eq!(filtered_jobs[0].priority, JobPriority::High);
    }

    // New comprehensive tests for enhanced scheduler functionality

    #[tokio::test]
    async fn test_schedule_task() {
        let mut scheduler = Scheduler::new();
        let task =
            create_test_scheduled_task("test_task", TaskType::Immediate, TaskSchedule::Immediate);
        let task_id = task.id;

        scheduler
            .schedule_task(task)
            .await
            .expect("schedule_task failed");

        let tasks = scheduler
            .list_scheduled_tasks()
            .await
            .expect("list_scheduled_tasks failed");
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].id, task_id);
        assert_eq!(tasks[0].status, TaskStatus::Scheduled);
    }

    #[tokio::test]
    async fn test_cancel_task() {
        let mut scheduler = Scheduler::new();
        let task = create_test_scheduled_task(
            "test_task",
            TaskType::Once,
            TaskSchedule::Once {
                at: chrono::Utc::now() + chrono::Duration::hours(1),
            },
        );
        let task_id = task.id;

        scheduler
            .schedule_task(task)
            .await
            .expect("schedule_task failed");
        scheduler
            .cancel_task(task_id)
            .await
            .expect("cancel_task failed");

        let tasks = scheduler
            .list_scheduled_tasks()
            .await
            .expect("list_scheduled_tasks failed");
        assert_eq!(tasks.len(), 0); // Cancelled task is removed
    }

    #[tokio::test]
    async fn test_scheduler_with_executor() {
        let executor = Arc::new(MockExecutor);
        let scheduler = Scheduler::with_executor(executor);

        assert!(scheduler.executor.is_some());
        assert_eq!(scheduler.status, SchedulerStatus::Stopped);
    }

    #[tokio::test]
    async fn test_background_execution_lifecycle() {
        let executor = Arc::new(MockExecutor);
        let mut scheduler = Scheduler::with_executor(executor);

        // Start background execution
        scheduler
            .start_background_execution()
            .await
            .expect("start_background_execution failed");
        assert_eq!(
            scheduler
                .get_scheduler_status()
                .await
                .expect("get_scheduler_status failed"),
            SchedulerStatus::Running
        );

        // Stop background execution
        scheduler
            .stop_background_execution()
            .await
            .expect("stop_background_execution failed");
        assert_eq!(
            scheduler
                .get_scheduler_status()
                .await
                .expect("get_scheduler_status failed"),
            SchedulerStatus::Stopped
        );
    }

    #[tokio::test]
    async fn test_pause_resume_scheduler() {
        let mut scheduler = Scheduler::new();

        scheduler
            .pause_scheduler()
            .await
            .expect("pause_scheduler failed");
        assert_eq!(
            scheduler
                .get_scheduler_status()
                .await
                .expect("get_scheduler_status failed"),
            SchedulerStatus::Paused
        );

        scheduler
            .resume_scheduler()
            .await
            .expect("resume_scheduler failed");
        assert_eq!(
            scheduler
                .get_scheduler_status()
                .await
                .expect("get_scheduler_status failed"),
            SchedulerStatus::Stopped
        );
    }

    #[tokio::test]
    async fn test_execute_scheduled_tasks_without_executor() {
        let mut scheduler = Scheduler::new();

        let result = scheduler.execute_scheduled_tasks().await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("No executor configured"));
    }

    #[tokio::test]
    async fn test_execute_scheduled_tasks_with_executor() {
        let executor = Arc::new(MockExecutor);
        let mut scheduler = Scheduler::with_executor(executor);

        let task = create_test_scheduled_task(
            "immediate_task",
            TaskType::Immediate,
            TaskSchedule::Immediate,
        );

        scheduler
            .schedule_task(task)
            .await
            .expect("schedule_task failed");
        scheduler
            .execute_scheduled_tasks()
            .await
            .expect("execute_scheduled_tasks failed");

        // After execution, task should still exist but status might change
        let tasks = scheduler
            .list_scheduled_tasks()
            .await
            .expect("list_scheduled_tasks failed");
        assert_eq!(tasks.len(), 1);
    }

    #[tokio::test]
    async fn test_task_scheduling_types() {
        let mut scheduler = Scheduler::new();

        // Test immediate task
        let immediate_task =
            create_test_scheduled_task("immediate", TaskType::Immediate, TaskSchedule::Immediate);
        scheduler
            .schedule_task(immediate_task)
            .await
            .expect("schedule_task failed");

        // Test interval task
        let interval_task = create_test_scheduled_task(
            "interval",
            TaskType::Interval,
            TaskSchedule::Interval {
                every: StdDuration::from_secs(60),
            },
        );
        scheduler
            .schedule_task(interval_task)
            .await
            .expect("schedule_task failed");

        // Test once task
        let once_task = create_test_scheduled_task(
            "once",
            TaskType::Once,
            TaskSchedule::Once {
                at: chrono::Utc::now() + chrono::Duration::hours(1),
            },
        );
        scheduler
            .schedule_task(once_task)
            .await
            .expect("schedule_task failed");

        // Test cron task
        let cron_task = create_test_scheduled_task(
            "cron",
            TaskType::Cron,
            TaskSchedule::Cron {
                expression: "0 */6 * * *".to_string(),
            },
        );
        scheduler
            .schedule_task(cron_task)
            .await
            .expect("schedule_task failed");

        let tasks = scheduler
            .list_scheduled_tasks()
            .await
            .expect("list_scheduled_tasks failed");
        assert_eq!(tasks.len(), 4);

        let task_types: std::collections::HashSet<_> =
            tasks.iter().map(|t| t.task_type.clone()).collect();
        assert!(task_types.contains(&TaskType::Immediate));
        assert!(task_types.contains(&TaskType::Interval));
        assert!(task_types.contains(&TaskType::Once));
        assert!(task_types.contains(&TaskType::Cron));
    }

    #[tokio::test]
    async fn test_task_ready_logic() {
        // Test immediate task is ready
        let immediate_task =
            create_test_scheduled_task("immediate", TaskType::Immediate, TaskSchedule::Immediate);
        assert!(Scheduler::is_task_ready(&ScheduledTask {
            status: TaskStatus::Scheduled,
            next_run_at: Some(chrono::Utc::now()),
            ..immediate_task
        }));

        // Test future task is not ready
        let future_task = create_test_scheduled_task(
            "future",
            TaskType::Once,
            TaskSchedule::Once {
                at: chrono::Utc::now() + chrono::Duration::hours(1),
            },
        );
        assert!(!Scheduler::is_task_ready(&ScheduledTask {
            status: TaskStatus::Scheduled,
            next_run_at: Some(chrono::Utc::now() + chrono::Duration::hours(1)),
            ..future_task
        }));

        // Test non-scheduled task is not ready
        let pending_task =
            create_test_scheduled_task("pending", TaskType::Immediate, TaskSchedule::Immediate);
        assert!(!Scheduler::is_task_ready(&ScheduledTask {
            status: TaskStatus::Pending,
            next_run_at: Some(chrono::Utc::now()),
            ..pending_task
        }));
    }

    #[tokio::test]
    async fn test_scheduler_statistics_integration() {
        let mut scheduler = Scheduler::new();

        let job = create_test_job(JobPriority::High);
        let task =
            create_test_scheduled_task("stats_task", TaskType::Immediate, TaskSchedule::Immediate);

        scheduler
            .schedule_job(job)
            .await
            .expect("schedule_job failed");
        scheduler
            .schedule_task(task)
            .await
            .expect("schedule_task failed");

        let stats = scheduler
            .get_statistics()
            .await
            .expect("get_statistics failed");
        assert!(stats.total_jobs > 0);

        // Test that scheduler status is accessible
        let status = scheduler
            .get_scheduler_status()
            .await
            .expect("get_scheduler_status failed");
        assert_eq!(status, SchedulerStatus::Stopped);
    }
}
