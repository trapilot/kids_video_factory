use std::env;
use std::sync::Arc;
use tokio::time::{sleep, Duration};

use crate::db::DbManager;
use crate::models::*;
use crate::helper::*;
use crate::workflow;


pub async fn run_scheduler(
    client: reqwest::Client,
    db: Arc<DbManager>,
) {
    let app_debug = "True" == env::var("APP_DEBUG").unwrap_or_else(|_| "False".to_string());

    loop {
        let mut state = match db.get_recent_state().await {
            Ok(Some(s)) => s,
            Ok(None) => VideoState::new(),
            Err(e) => {
                eprintln!("DB error, wait 30 minutes to retry: {}", e);
                sleep(Duration::from_mins(30)).await;
                continue;
            }
        };

        if state.meta.retry_count >= state.meta.max_retry {
            println!(
                "⛔ Pending session: {} | node: {:?} | topic: {}",
                state.session_id,
                state.current_node,
                state.target_topic,
            );

            let updated_at = chrono::DateTime::parse_from_rfc3339(&state.meta.updated_at)
                .map(|dt| dt.with_timezone(&chrono::Local))
                .ok();

            if let Some(last_time) = updated_at {
                let elapsed = chrono::Local::now() - last_time;

                // Threshold: 30 minutes
                if elapsed.num_minutes() >= 30 {
                    let _ = db.delete_state(&state.session_id).await;
                    println!("ℹ️  Removed session: {}", state.session_id);
                } else {
                    if app_debug {
                        let _ = db.save_state(&state.revived()).await;
                        println!("🔄 Revived session: {}", state.session_id);
                    } else {
                        println!("ℹ️  Reported, wait 10 minutes for next check");
                        sleep(Duration::from_mins(10)).await;
                    }
                }
            } else {
                let _ = db.save_state(&state.cancelled()).await;
                println!("ℹ️ Cancelled, wait 30 minutes to start new session");

                sleep(Duration::from_mins(30)).await;
            }
            continue;
        }

        let result = workflow::run_agent_workflow(
            &client,
            &db,
            &mut state,
        ).await;

        match result {
            Ok(artifact) => {
                let _ = db.save_state(&state.done()).await;
                let _ = db.save_topic(state.target_age.clone(), &state.target_topic).await;
 
                println!("✅ OK: {}, Updated into long-term memory", artifact.title);
                sleep(Duration::from_mins(10)).await;
            }

            Err(e) => {
                println!("🛑 ERR: {}", e);
                // 🔥 exponential backoff
                let next_backoff = next_backoff(state.meta.backoff_ms as u64);

                let _ = db.save_state(&state.retry(e)).await;

                if state.meta.retry_count <= state.meta.max_retry {
                    println!("🔁 Retry in {}ms", next_backoff);
                    sleep(Duration::from_millis(next_backoff)).await;
                }
            }
        }
    }
}