mod db;
mod enums;
mod models;
mod agents;
mod workflow;
mod helper;
mod render;
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

    // for age in target_ages {
    //     let client = client.clone();
    //     let db = db.clone();
    //     tokio::spawn(async move {
    //         scheduler::run_scheduler(client, db, age).await;
    //     });
    // }

    // // keep process alive
    // loop {
    //     tokio::time::sleep(std::time::Duration::from_secs(3600)).await;
    // }

    // let mut interval = tokio::time::interval(Duration::from_secs(24 * 60 * 60));

    // loop {
    //     interval.tick().await;
    //     println!("\n🕒 Start date of production cycle: {}", chrono::Local::now().format("%Y-%m-%d"));

    //     let mut tasks = vec![];

    //     for &age in &target_ages {
    //         let client = http_client.clone();
    //         let db = db.clone();

    //         // Extract the history and pass it to the Task.
    //         let history_list = db.get_recent_topics(age, 10).await.unwrap_or_default();
    //         let history_str = if history_list.is_empty() { "Not yet".to_string() } else { history_list.join(", ") };

    //         let task = tokio::spawn(async move {
    //             println!("▶️ Working on the script for age group: {}", age);
    //             match workflow::run_agent_workflow(&client, &db, age, &history_str).await {
    //                 Ok(artifact) => {
    //                     println!("✅ Video has been rendered: {}", artifact.title);
                        
    //                     let video_path = artifact.video_path.as_ref().unwrap();

    //                     // Simultaneous upload
    //                     let title = format!("{} #shorts #forkids", artifact.title);

    //                     let (yt_res, tt_res) = tokio::join!(
    //                         uploader::upload_to_youtube(&client, video_path, &title),
    //                         uploader::upload_to_tiktok(&client, video_path, &title),
    //                     );

    //                     if yt_res.is_ok() || tt_res.is_ok() {
    //                         println!("✅ Video has been successfully uploaded: {}", artifact.title);
    //                         return Ok((age, artifact.title)); // Return to save to the database.
    //                     }
    //                 }
    //                 Err(e) => eprintln!("🔴 Error with age {}: {}", age, e),
    //             }
    //             Err("Failed".to_string())
    //         });
    //         tasks.push(task);
    //     }

    //     // Wait for all tasks to complete and save the successful results to the database.
    //     let results = futures::future::join_all(tasks).await;
    //     for res in results {
    //         if let Ok(Ok((age, topic))) = res {
    //             let _ = db.save_topic(age, &topic).await;
    //             println!("💾 Updated '{}' into long-term memory.", topic);
    //         }
    //     }
    // }
}