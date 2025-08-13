# Scheduler API Documentation

## Overview

The `Scheduler` agent manages background tasks and job scheduling with priority queues, cron-style scheduling, and persistence across application restarts.

## API Contract

### SchedulerTrait

```rust
#[async_trait]
pub trait SchedulerTrait: Send + Sync {
    /// Schedule a job for execution
    async fn schedule_job(&self, job: ScheduledJob) -> Result<JobId>;
    
    /// Cancel a scheduled job
    async fn cancel_job(&self, job_id: JobId) -> Result<()>;
    
    /// Get job status and information
    async fn get_job_status(&self, job_id: JobId) -> Result<JobStatus>;
    
    /// List all jobs with optional filtering
    async fn list_jobs(&self, filter: Option<JobFilter>) -> Result<Vec<JobInfo>>;
    
    /// Pause/resume the scheduler
    async fn pause(&self) -> Result<()>;
    async fn resume(&self) -> Result<()>;
}
```

## Key Data Structures

### ScheduledJob

```rust
pub struct ScheduledJob {
    pub id: JobId,
    pub job_type: JobType,
    pub schedule: Schedule,              // Cron, interval, or one-time
    pub priority: JobPriority,           // High, Normal, Low
    pub payload: serde_json::Value,      // Job-specific data
    pub retry_policy: RetryPolicy,       // Retry configuration
    pub timeout: Option<Duration>,       // Job timeout
}
```

### Schedule Types

```rust
pub enum Schedule {
    Once { at: DateTime<Utc> },                    // One-time execution
    Interval { every: Duration },                   // Periodic execution
    Cron { expression: String },                    // Cron-style scheduling
    AfterJob { job_id: JobId, delay: Duration },   // Dependency-based
}
```

## Usage Examples

### Basic Job Scheduling

```rust
use orchestrator::agents::{Scheduler, SchedulerTrait};
use orchestrator::jobs::{ScheduledJob, Schedule, JobPriority};

#[tokio::main]
async fn main() -> Result<()> {
    let scheduler = Scheduler::new()
        .with_persistence(true)
        .with_max_concurrent_jobs(10);
    
    // Schedule a backup job every day at 2 AM
    let backup_job = ScheduledJob {
        id: JobId::new(),
        job_type: JobType::Backup,
        schedule: Schedule::Cron { 
            expression: "0 2 * * *".to_string() 
        },
        priority: JobPriority::Normal,
        payload: json!({
            "source": "/home/user/data",
            "destination": "/backup/daily"
        }),
        retry_policy: RetryPolicy::default(),
        timeout: Some(Duration::from_secs(3600)),
    };
    
    let job_id = scheduler.schedule_job(backup_job).await?;
    println!("Backup job scheduled: {}", job_id);
    
    Ok(())
}
```

### Job Monitoring

```rust
#[tokio::main]
async fn main() -> Result<()> {
    let scheduler = create_scheduler().await?;
    
    // Monitor all jobs
    let jobs = scheduler.list_jobs(None).await?;
    
    for job in jobs {
        let status = scheduler.get_job_status(job.id).await?;
        
        println!("Job {}: {:?}", job.id, status.state);
        println!("  Next run: {:?}", status.next_execution);
        println!("  Last run: {:?}", status.last_execution);
        
        if let Some(error) = status.last_error {
            println!("  Last error: {}", error);
        }
    }
    
    Ok(())
}
```

For complete documentation, see the source code in `crates/orchestrator/src/agents/scheduler.rs`.