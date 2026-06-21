
use async_trait::async_trait;

use crate::agent::*;
use crate::models::*;
use crate::workflow;


pub struct ManagerAgent;

#[async_trait]
impl Agent for ManagerAgent {
    async fn run(&self, _ctx: &workflow::Context, job: &Job) -> Result<(), AgentError> {
        println!("Running job: {}", job.id);
        Ok(())
    }
}
