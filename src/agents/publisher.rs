
use std::collections::HashMap;
use async_trait::async_trait;

use crate::agent::*;
use crate::models::*;
use crate::entities::*;
// use crate::producer;
use crate::uploader;
use crate::workflow;


pub struct PublisherAgent;

#[async_trait]
impl Agent for PublisherAgent {
    async fn run(&self, ctx: &workflow::Context, job: &Job) -> Result<(), AgentError> {
        println!("📤 [Publisher] Publishing the video...");
        
        let video_metadata: VideoMetadata =
            serde_json::from_str(&job.payload)
            .map_err(|e| AgentError::Decode(e.to_string()))?;

        let mut configs = HashMap::new();
        configs.insert(uploader::Channel::Youtube, uploader::ChannelConfig::Youtube {
            category_id: ctx.cfg.movie.youtube_category.clone(),
            tags: ctx.cfg.movie.default_tags.clone(),
            description: ctx.cfg.movie.default_description.clone(),
        });
        configs.insert(uploader::Channel::Tiktok, uploader::ChannelConfig::Tiktok {
            privacy_level: "PUBLIC".into(),
            disable_comment: true,
        });

        let results = ctx.up
            .upload_all(&video_metadata.video_path, &video_metadata.title, configs)
            .await;

        let mut publish_errors = uploader::UploaderError::default();
        let mut success_count = 0;

        for (channel, res) in &results {
            match res {
                Ok(_) => success_count += 1,
                Err(e) => {
                    println!("Error when uploading {:?}: {}", channel, e);

                    match channel {
                        uploader::Channel::Youtube => publish_errors.youtube = Some(e.clone()),
                        uploader::Channel::Tiktok => publish_errors.tiktok = Some(e.clone()),
                    }
                }
            }
        }
        
        let publish_state = uploader::UploaderState {
            all_uploaded: success_count == results.len() && results.len() > 0,
            any_uploaded: success_count > 0,
            errors: publish_errors,
        };
        
        if !publish_state.any_uploaded {
            return Err(AgentError::Execute("Upload error....".to_string()));
        }

        let payload =
            serde_json::to_string(&publish_state)
            .map_err(|e| AgentError::Encode(e.to_string()))?;

        ctx.db
            .handoff_job(job, AgentType::Cleaner, payload.clone())
            .await
            .map_err(|e| AgentError::Handoff(e.to_string()))?;

        ctx.db
            .complete_job(&job.id, payload)
            .await
            .map_err(|e| AgentError::Handoff(e.to_string()))?;
        
        Ok(())
    }
}