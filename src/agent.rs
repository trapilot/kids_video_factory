use std::sync::Arc;
use tokio::sync::Semaphore;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use strum_macros::{Display, EnumString};
use thiserror::Error;

use crate::models;
use crate::workflow;


#[derive(Debug, Clone, Display, EnumString, Serialize, Deserialize, Eq, PartialEq, Hash, sqlx::Type)]
#[strum(serialize_all = "lowercase")]
#[serde(rename_all = "lowercase")]
#[sqlx(rename_all = "lowercase", type_name = "text")]
pub enum AgentType {
    Manager,
    Planner,
    Writer,
    Builder,
    Renderer,
    Publisher,
    Cleaner,
}

#[derive(Debug, Error)]
pub enum AgentError {
    #[error("Agent handoff job error: {0}")]
    Handoff(String),

    #[error("Agent execute error: {0}")]
    Execute(String),

    #[error("Agent encode error: {0}")]
    Encode(String),

    #[error("Agent decode error: {0}")]
    Decode(String),
}

#[async_trait]
pub trait Agent: Send + Sync {
    async fn run(
        &self,
        ctx: &workflow::Context,
        job: &models::Job,
    ) -> Result<(), AgentError>;
}

pub struct AgentPool {
    semaphore: Arc<tokio::sync::Semaphore>,
}

impl AgentPool {
    pub fn new(max_concurrency: usize) -> Self {
        Self {
            semaphore: Arc::new(Semaphore::new(max_concurrency)),
        }
    }

    pub fn try_spawn(
        &self,
        agent: Arc<dyn Agent>,
        ctx: workflow::Context,
        job: models::Job,
    ) -> bool {
        let Ok(permit) = self.semaphore.clone().try_acquire_owned() else {
            return false;
        };

        let job_clone = job.clone();
        let ctx_clone = ctx.clone();

        tokio::spawn(async move {
            let _permit = permit;

            if let Err(e) = agent.run(&ctx, &job).await {
                eprintln!("❌ Agent {:?} failed job {} --> {}", job_clone.agent, job_clone.id, e);
                
                let retry_count = job_clone.retry_count;
                let max_retry = job_clone.max_retry;

                let db_result = if retry_count < max_retry {
                    ctx_clone.db.retry_job(&job_clone.id, e.to_string()).await
                } else {
                    ctx_clone.db.fail_job(&job_clone.id, e.to_string()).await
                };

                match db_result {
                    Ok(_) => {},
                    Err(e) => eprintln!("❌ Failed job error {} --> {}", job_clone.id, e),
                }
            }
        });

        true
    }
}