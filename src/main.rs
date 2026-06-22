mod db;
mod agent;
mod enums;
mod config;
mod models;
mod provider;
mod uploader;
mod entities;
mod workflow;

mod agents;
mod providers;
mod uploaders;

use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();

    let db = Arc::new(db::DbManager::new("data_history.db").await?);
    let pm = Arc::new(provider::ProviderManager::new());
    let up = Arc::new(uploader::UploaderManager::new());
    let cfg = Arc::new(config::build_config());

    up.register(Arc::new(uploaders::YoutubeUploader::new())).await;
    pm.register_with_keys(
        Arc::new(providers::GeminiClient),
        1,
        "GEMINI",
    ).await;
    pm.register_with_keys(
        Arc::new(providers::CFWorkerClient),
        1,
        "CF_WORKER",
    ).await;
    pm.register_with_keys(
        Arc::new(providers::ElevenLabsClient),
        1,
        "ELEVEN_LABS",
    ).await;

    // db.revert_job("d1552fa0-7f00-4dc4-9baf-6767ad7b67fa").await?;

    let ctx = workflow::Context { db, pm, up, cfg };
    let workflow = workflow::Workflow::new(ctx);
    println!("🚀 Sentinel System v1.0 - AI Video Engine Startup");
    workflow.start().await;
    
    Ok(())
}