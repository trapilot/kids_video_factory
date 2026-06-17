mod db;
mod enums;
mod models;
mod apis;
mod workflow;
mod helper;
mod renderer;
mod uploader;
mod scheduler;
mod macro_rules;

use std::sync::Arc;
use crate::db::DbManager;


#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();

    let client = reqwest::Client::new();
    let db = Arc::new(DbManager::new("data_history.db"));

    println!("🚀 Sentinel System v1.0 - AI Video Engine Startup");
    
    scheduler::run_scheduler(client, db).await;

    Ok(())
}