mod db;
mod agent;
mod enums;
mod config;
mod models;
mod helper;
// mod producer;
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
        2,
        "GEMINI",
        1500,
    ).await;
    pm.register_with_keys(
        Arc::new(providers::CFWorkerClient),
        2,
        "CF_WORKER",
        1500,
    ).await;
    pm.register_with_keys(
        Arc::new(providers::ElevenLabsClient),
        2,
        "ELEVEN_LABS",
        1500,
    ).await;

    db.revert_job("6ad98afc-4fab-445a-8a6f-5734695aa3c4").await?;

    let ctx = workflow::Context { db, pm, up, cfg };
    let workflow = workflow::Workflow::new(ctx);
    println!("🚀 Sentinel System v1.0 - AI Video Engine Startup");
    workflow.start().await;
    
    Ok(())
}