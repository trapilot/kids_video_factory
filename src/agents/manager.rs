
use std::sync::Arc;
use async_trait::async_trait;

use crate::AppState;
use crate::agent::*;
use crate::models::*;


pub struct ManagerAgent;

#[async_trait]
impl Agent for ManagerAgent {
    async fn run(&self, _state: &Arc<AppState>, job: &Job) -> Result<(), AgentError> {
        println!("Running workflow...");
        Ok(())
    }
}
