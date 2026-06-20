use crate::AppContext;
use crate::enums::*;
use crate::helper::*;
use crate::entities::*;


pub async fn start(ctx: &AppContext) {
    let db = ctx.db.clone();
    let main_char = Character::main_char();
    let workflow_per_day = ctx.cfg.workflow_per_day;
    
    // 🔥 scheduler spawn
    tokio::spawn(async move {
        loop {
            let planner_busy =
                db.agent_is_busy(AgentType::Planner)
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
                            AgentType::Manager,
                            AgentType::Planner,
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
            
            tokio::time::sleep(std::time::Duration::from_hours(2)).await;
        }
    });
}