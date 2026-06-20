use tokio::time::Duration;

use crate::AppContext;
use crate::agents::*;


pub async fn dispatch(ctx: &AppContext, pools: &AgentPools) {
    loop {
        let Ok(Some(job)) = ctx.db.claim_job().await else {
            tokio::time::sleep(Duration::from_millis(5000)).await;
            continue;
        };

        let pool = pools.get(&job.agent);

        if !pool.has_capacity() {
            let _ = ctx.db.revert_job(&job.id).await;
            tokio::time::sleep(Duration::from_millis(5000)).await;
            continue;
        }

        pool.spawn(ctx, job);
    }
}