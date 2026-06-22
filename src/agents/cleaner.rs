
use async_trait::async_trait;

use crate::agent::*;
use crate::models::*;
use crate::workflow;
use crate::uploader;


pub struct CleanerAgent;

#[async_trait]
impl Agent for CleanerAgent {
    async fn run(&self,  ctx: &workflow::Context, job: &Job) -> Result<(), AgentError> {
        println!("📤 [Cleaner] Cleaning the video...");

        let publish_state: uploader::UploaderState =
            serde_json::from_str(&job.payload)
            .map_err(|e| AgentError::Decode(e.to_string()))?;
        
        if let Some(err) = &publish_state.errors.youtube {
            eprintln!("🔴 YouTube error: {}", err);
        }

        if let Some(err) = &publish_state.errors.tiktok {
            eprintln!("🔴 TikTok error: {}", err);
        }

        ctx.db
            .complete_job(&job.id, "DONE".to_string())
            .await
            .map_err(|e| AgentError::Handoff(e.to_string()))?;

        Ok(())
    }
}