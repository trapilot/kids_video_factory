
use std::sync::Arc;
use std::collections::HashMap;

use crate::db;
use crate::agent;
use crate::config;
use crate::agents;
use crate::entities;
use crate::provider;
use crate::uploader;


#[derive(Clone)]
pub struct Context {
    pub db: Arc<db::DbManager>,
    pub pm: Arc<provider::ProviderManager>,
    pub up: Arc<uploader::UploaderManager>,
    pub cfg: Arc<config::Config>,
}

pub struct Workflow {
    ctx: Context,
    pools: HashMap<agent::AgentType, agent::AgentPool>,
    agents: HashMap<agent::AgentType, Arc<dyn agent::Agent>>,
}

impl Workflow {
    pub fn new(ctx: Context) -> Self {
        let mut pools = HashMap::new();
        let mut agents: HashMap<agent::AgentType, Arc<dyn agent::Agent>> = HashMap::new();

        pools.insert(agent::AgentType::Manager, agent::AgentPool::new(1));
        pools.insert(agent::AgentType::Planner, agent::AgentPool::new(1));
        pools.insert(agent::AgentType::Writer, agent::AgentPool::new(1));
        pools.insert(agent::AgentType::Builder, agent::AgentPool::new(1));
        pools.insert(agent::AgentType::Renderer, agent::AgentPool::new(1));
        pools.insert(agent::AgentType::Publisher, agent::AgentPool::new(1));

        agents.insert(agent::AgentType::Manager, Arc::new(agents::ManagerAgent));
        agents.insert(agent::AgentType::Planner, Arc::new(agents::PlannerAgent));
        agents.insert(agent::AgentType::Writer, Arc::new(agents::WriterAgent));
        agents.insert(agent::AgentType::Builder, Arc::new(agents::BuilderAgent));
        agents.insert(agent::AgentType::Renderer, Arc::new(agents::RendererAgent));
        agents.insert(agent::AgentType::Publisher, Arc::new(agents::PublisherAgent));
        agents.insert(agent::AgentType::Cleaner, Arc::new(agents::CleanerAgent));
        
        Self { ctx, pools, agents }
    }

    pub async fn start(&self) {
        self.start_scheduler();
        self.dispatch().await;
    }

    async fn dispatch(&self) {
        loop {
            let Ok(Some(job)) = self.ctx.db.claim_job().await else {
                tokio::time::sleep(tokio::time::Duration::from_millis(5000)).await;
                continue;
            };

            let Some(agent) = self.agents.get(&job.agent) else {
                let _ = self.ctx.db.revert_job(&job.id).await;
                continue;
            };

            let Some(pool) = self.pools.get(&job.agent) else {
                let _ = self.ctx.db.revert_job(&job.id).await;
                continue;
            };

            if !pool.try_spawn(agent.clone(), self.ctx.clone(), job.clone()) {
                let _ = self.ctx.db.revert_job(&job.id).await;
            }
        }
    }

    fn start_scheduler(&self) {
        let db = self.ctx.db.clone();
        let workflow_per_day = self.ctx.cfg.workflow_per_day;
        let main_char = entities::Character::main_char();
        
        // 🔥 scheduler spawn
        tokio::spawn(async move {
            loop {
                let planner_busy =
                    db.agent_is_busy(agent::AgentType::Planner)
                    .await
                    .unwrap_or(true);
                
                let today_count = db.count_workflows_today()
                    .await
                    .unwrap_or(0);

                if !planner_busy && today_count < workflow_per_day {
                    match db.create_workflow(main_char.age(), "Create AI video".to_string()).await {
                        Ok(workflow_id) => {

                            let job = db.create_job(
                                workflow_id,
                                agent::AgentType::Manager,
                                agent::AgentType::Planner,
                                "Create new workflow".to_string(),
                            ).await;

                            match job {
                                Ok(_) => {},
                                Err(e) => println!("🔴 Job ERR: {}", e)
                            }
                        }
                        Err(e) => println!("🔴 Workflow ERR: {}", e)
                    }
                }
                
                tokio::time::sleep(std::time::Duration::from_secs(30)).await;
            }
        });
    }
}
