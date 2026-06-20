mod db;
mod api;
mod enums;
mod config;
mod models;
mod agents;
mod helper;
mod provider;
mod renderer;
mod uploader;
mod dispatcher;
mod scheduler;
mod entities;

use std::sync::Arc;


#[derive(Clone)]
pub struct AppContext {
    pub db: Arc<db::DbManager>,
    pub pm: Arc<provider::ProviderManager>,
    pub sm: Arc<provider::SemaphoreManager>,
    pub cfg: Arc<config::Config>,
    pub http: reqwest::Client,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();

    let ctx = AppContext {
        db: Arc::new(db::DbManager::new("data_history.db").await?),
        pm: Arc::new(provider::ProviderManager::new()),
        sm: Arc::new(provider::SemaphoreManager::new()),
        cfg: Arc::new(config::build_config()),
        http: reqwest::Client::new(),
    };

    let pools = agents::AgentPools::new();

    ctx.pm
        .load_keys_from_env(
            enums::Provider::Gemini,
            "GEMINI",
            1500,
        )
        .await;

    ctx.pm
        .load_keys_from_env(
            enums::Provider::ElevenLabs,
            "ELEVENLABS",
            1500,
        )
        .await;

    ctx.pm
        .load_keys_from_env(
            enums::Provider::Cloudflare,
            "CF",
            1500,
        )
        .await;

    println!("🚀 Sentinel System v1.0 - AI Video Engine Startup");
    // let _ = ctx.db.revert_job("7b99c50e-574a-437e-87d7-11e442c8ae58").await;

    scheduler::start(&ctx).await;
    dispatcher::dispatch(&ctx, &pools).await;
    
    Ok(())
}