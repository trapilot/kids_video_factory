mod db;
mod agent;
mod oauth;
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


#[derive(Clone)]
pub struct AppState {
    pub services: Arc<Services>,
    pub config: Arc<config::Config>,
}

pub struct Services {
    pub db: Arc<db::DbManager>,
    pub providers: Arc<provider::ProviderManager>,
    pub uploaders: Arc<uploader::UploaderManager>,
}

impl AppState {
    pub async fn build() -> Result<Self, Box<dyn std::error::Error>> {
        let db = Arc::new(db::DbManager::new("data_history.db").await?);

        let providers = Arc::new(provider::ProviderManager::new());
        providers.register_with_keys(
            Arc::new(providers::GeminiClient),
            1,
            "GEMINI",
        ).await;
        providers.register_with_keys(
            Arc::new(providers::CFWorkerClient),
            1,
            "CF_WORKER",
        ).await;
        providers.register_with_keys(
            Arc::new(providers::ElevenLabsClient),
            1,
            "ELEVEN_LABS",
        ).await;


        let uploaders = Arc::new(uploader::UploaderManager::new());
        uploaders.register(
            Arc::new(uploaders::YoutubeUploader::new())
        ).await;

        let services = Arc::new(Services {
            db,
            providers,
            uploaders,
        });

        Ok(Self {
            services,
            config: Arc::new(config::build_config()),
        })
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {   
    dotenvy::dotenv().ok(); 
    println!("🚀 Sentinel System v1.0 - AI Video Engine Startup");

    let revert_job = "";
    let state = Arc::new(AppState::build().await?);

    if !oauth::has_auth_token(&state.services.db).await {
        oauth::start_oauth_server(&state.services.db).await;
    }

    if !revert_job.is_empty() {
        println!("Revert current job: {}", revert_job);
        state.services.db.revert_job(revert_job).await?;
    }    

    let workflow = workflow::WorkflowEngine::new(state);

    workflow.start().await;
    
    Ok(())
}