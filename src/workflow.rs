
use std::sync::Arc;
use std::collections::HashMap;

use crate::AppState;
use crate::agent;
use crate::agents;
use crate::models;
use crate::entities;


struct AgentEntry {
    pool: agent::AgentPool,
    agent: Arc<dyn agent::Agent>,
}

pub struct WorkflowEngine {
    state: Arc<AppState>,
    registry: HashMap<agent::AgentType, AgentEntry>,
}

impl AgentEntry {
    pub fn try_spawn(
        &self,
        state: Arc<AppState>,
        job: models::Job,
    ) -> bool {
        self.pool
            .try_spawn(self.agent.clone(), state, job)
    }
}

impl WorkflowEngine {
    pub fn new(state: Arc<AppState>) -> Self {
        let mut registry = HashMap::new();

        registry.insert(
            agent::AgentType::Manager,
            AgentEntry {
                pool: agent::AgentPool::new(1),
                agent: Arc::new(agents::ManagerAgent),
            },
        );
        registry.insert(
            agent::AgentType::Planner,
            AgentEntry {
                pool: agent::AgentPool::new(1),
                agent: Arc::new(agents::PlannerAgent),
            },
        );
        registry.insert(
            agent::AgentType::Writer,
            AgentEntry {
                pool: agent::AgentPool::new(1),
                agent: Arc::new(agents::WriterAgent),
            },
        );
        registry.insert(
            agent::AgentType::Builder,
            AgentEntry {
                pool: agent::AgentPool::new(1),
                agent: Arc::new(agents::BuilderAgent),
            },
        );
        registry.insert(
            agent::AgentType::Renderer,
            AgentEntry {
                pool: agent::AgentPool::new(1),
                agent: Arc::new(agents::RendererAgent),
            },
        );
        registry.insert(
            agent::AgentType::Publisher,
            AgentEntry {
                pool: agent::AgentPool::new(1),
                agent: Arc::new(agents::PublisherAgent),
            },
        );
        registry.insert(
            agent::AgentType::Cleaner,
            AgentEntry {
                pool: agent::AgentPool::new(1),
                agent: Arc::new(agents::CleanerAgent),
            },
        );
        
        Self { state, registry }
    }

    pub async fn start(&self) {
        self.start_scheduler();

        self.dispatch().await;
    }

    async fn dispatch(&self) {
        loop {
            let Ok(Some(job)) = self.state.services.db.claim_job().await else {
                tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                continue;
            };

            let Some(entry) = self.registry.get(&job.agent) else {
                let _ = self.state.services.db.revert_job(&job.id).await;
                tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
                continue;
            };

            if !entry.try_spawn(self.state.clone(), job.clone()) {
                let _ = self.state.services.db.revert_job(&job.id).await;
            }

            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        }
    }

    fn start_scheduler(&self) {
        let state = self.state.clone();

        // 🔥 scheduler spawn
        tokio::spawn(async move {
            loop {
                let db = &state.services.db;
                let workflow_per_day = state.config.workflow_per_day;
                let main_char = entities::Character::main_char();

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
