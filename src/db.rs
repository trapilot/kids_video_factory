use std::path::Path;
use sqlx::sqlite::SqlitePoolOptions;
use sqlx::SqlitePool;

use crate::agent;
use crate::models::*;


pub const APP_VERSION: u8 = 1;

#[derive(Clone)]
pub struct DbManager {
    pub pool: SqlitePool,
}
impl DbManager {
    pub async fn new(db_url: &str) -> Result<Self, sqlx::Error> {
        let initialized = Path::new(db_url).exists();
        if !initialized {
            std::fs::File::create(db_url).ok();
        }

        let pool = SqlitePoolOptions::new()
            .max_connections(10)
            .connect(&format!("sqlite:{}?mode=rwc", db_url))
            .await?;

        if !initialized {
            sqlx::query(include_str!("../migrations/schema_01.sql"))
                .execute(&pool)
                .await?;
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        }


        // let now_date = chrono::Utc::now().date_naive();
        // let schema01_date = chrono::NaiveDate::from_ymd_opt(2026, 1, 1).unwrap();
        // if now_date <= schema01_date {}

        Ok(Self { pool })
    }

    pub async fn get_oauth_token(
        &self,
        provider: &str,
    ) -> Result<Option<OAuthToken>, sqlx::Error> {
        let token = sqlx::query_as::<_, OAuthToken>(
            r#"
            SELECT
                provider,
                client_id,
                client_secret,
                access_token,
                refresh_token,
                auth_code,
                expires_at,
                updated_at
            FROM oauth_tokens
            WHERE provider = ?
            "#
        )
        .bind(provider)
        .fetch_optional(&self.pool)
        .await?;

        Ok(token)
    }

    pub async fn save_oauth_token(
        &self,
        token: &OAuthToken,
    ) -> Result<Option<OAuthToken>, sqlx::Error> {
        let auth_token = sqlx::query_as::<_, OAuthToken>(
            r#"
            INSERT INTO oauth_tokens (
                provider,
                client_id,
                client_secret,
                access_token,
                refresh_token,
                auth_code,
                expires_at,
                updated_at
            )
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(provider)
            DO UPDATE SET
                client_id = excluded.client_id,
                client_secret = excluded.client_secret,
                access_token = excluded.access_token,
                refresh_token = excluded.refresh_token,
                auth_code = excluded.auth_code,
                expires_at = excluded.expires_at,
                updated_at = excluded.updated_at
            RETURNING *
            "#
        )
        .bind(&token.provider)
        .bind(&token.client_id)
        .bind(&token.client_secret)
        .bind(&token.access_token)
        .bind(&token.refresh_token)
        .bind(&token.auth_code)
        .bind(token.expires_at)
        .bind(token.updated_at)
        .fetch_optional(&self.pool)
        .await?;

        Ok(auth_token)
    }

    pub async fn create_job(
        &self,
        workflow_id: String,
        parent: agent::AgentType,
        agent: agent::AgentType,
        payload: String,
    ) -> Result<(), sqlx::Error> {
        let now = chrono::Utc::now().timestamp();
        let next_24h = now + 24 * 60 * 60;

        sqlx::query(
            r#"
            INSERT INTO jobs (
                id,
                workflow_id,
                parent,
                agent,
                version,
                status,
                payload,
                threshold_at,
                created_at,
                updated_at
            )
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#
        )
        .bind(uuid::Uuid::new_v4().to_string())
        .bind(workflow_id)
        .bind(parent.to_string())
        .bind(agent.to_string())
        .bind(APP_VERSION)
        .bind(JobStatus::Pending.to_string())
        .bind(payload)
        .bind(next_24h)
        .bind(now)
        .bind(now)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn claim_job(&self) -> Result<Option<Job>, sqlx::Error> {
        let job = sqlx::query_as::<_, Job>(
            r#"
            UPDATE jobs
            SET status = ?, locked_at = ?
            WHERE id = (
                SELECT id
                FROM jobs
                WHERE version = ? AND status = ? AND locked_at IS NULL
                ORDER BY created_at
                LIMIT 1
            )
            RETURNING *
            "#
        )
        .bind(JobStatus::Processing.to_string())
        .bind(chrono::Utc::now().timestamp())
        .bind(APP_VERSION)
        .bind(JobStatus::Pending.to_string())
        .fetch_optional(&self.pool)
        .await?;

        // match &job {
        //     Some(j) => println!("Job: {}", j.workflow_id),
        //     None => println!("No pending jobs found"),
        // }

        Ok(job)
    }

    pub async fn complete_job(
        &self,
        job_id: &str,
        result: String,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            UPDATE jobs
            SET status = ?, result = ?, updated_at = ?
            WHERE id = ?
            "#,
        )
        .bind(JobStatus::Completed.to_string())
        .bind(result)
        .bind(chrono::Utc::now().timestamp())
        .bind(job_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn retry_job(
        &self,
        job_id: &str,
        error: String,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            UPDATE jobs
            SET status = ?, result = ?, updated_at = ?, locked_at = NULL, retry_count = retry_count + 1
            WHERE id = ?
            "#,
        )
        .bind(JobStatus::Pending.to_string())
        .bind(error)
        .bind(chrono::Utc::now().timestamp())
        .bind(job_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn fail_job(
        &self,
        job_id: &str,
        error: String,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            UPDATE jobs
            SET status = ?, result = ?, updated_at = ?
            WHERE id = ?
            "#,
        )
        .bind(JobStatus::Failed.to_string())
        .bind(error)
        .bind(chrono::Utc::now().timestamp())
        .bind(job_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn revert_job(&self, job_id: &str) -> Result<bool, sqlx::Error> {
        let result = sqlx::query(
            r#"
            UPDATE jobs
            SET status = ?, retry_count = 0, locked_at = NULL
            WHERE id = ?
            "#
        )
        .bind(JobStatus::Pending.to_string())
        .bind(job_id)
        .execute(&self.pool)
        .await?;

        // println!("Rollback: {}", result.rows_affected() == 1);
        Ok(result.rows_affected() == 1)
    }

    pub async fn handoff_job(
        &self,
        job: &Job,
        agent: agent::AgentType,
        payload: String,
    ) -> Result<(), String> {
        if job.agent != agent {
            self.create_job(
                job.workflow_id.clone(),
                job.agent.clone(),
                agent,
                payload.clone(),
            )
            .await
            .map_err(|e| e.to_string())?;
        }

        self.complete_job(&job.id, payload)
            .await
            .map_err(|e| e.to_string())?;

        Ok(())
    }

    pub async fn create_workflow(
        &self,
        age: i32,
        task: String,
    ) -> Result<String, sqlx::Error> {

        let workflow_id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now().timestamp();
        let next_24h = now + 24 * 60 * 60;

        sqlx::query(
            r#"
            INSERT INTO workflows (
                id,
                age,
                task,
                status,
                threshold_at,
                created_at,
                updated_at
            )
            VALUES (?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&workflow_id)
        .bind(age)
        .bind(task)
        .bind(WorkflowStatus::Running.to_string())
        .bind(next_24h)
        .bind(now)
        .bind(now)
        .execute(&self.pool)
        .await?;

        Ok(workflow_id)
    }

    pub async fn count_workflows_today(&self) -> Result<i64, sqlx::Error> {
        let start = self.start_of_today_ts();

        let count: (i64,) = sqlx::query_as(
            r#"
            SELECT COUNT(*)
            FROM workflows
            WHERE created_at >= ?
            "#
        )
        .bind(start)
        .fetch_one(&self.pool)
        .await?;

        Ok(count.0)
    }

    pub async fn save_topic(
        &self,
        workflow_id: &str,
        topic: String,
    ) -> Result<Option<Workflow>, sqlx::Error> {
        let workflow = sqlx::query_as::<_, Workflow>(
            r#"
            UPDATE workflows
            SET topic = ?, updated_at = ?
            WHERE id = ?
            RETURNING *
            "#,
        )
        .bind(topic)
        .bind(chrono::Utc::now().timestamp())
        .bind(workflow_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(workflow)
    }

    pub async fn get_recent_topics(&self, age: i32, days: u8) -> Result<Vec<String>, sqlx::Error> {
        let since = self.start_of_recent_ts(days);
        let topics = sqlx::query_scalar::<_, String>(
            r#"
            SELECT topic
            FROM workflows
            WHERE age = ? AND created_at >= ? AND status != ?
            ORDER BY id DESC
            LIMIT ?
            "#
        )
        .bind(age)
        .bind(since)
        .bind(WorkflowStatus::Failed.to_string())
        .fetch_all(&self.pool)
        .await?;

        Ok(topics)
    }
    
    pub async fn agent_is_busy(
        &self,
        agent: agent::AgentType,
    ) -> Result<bool, sqlx::Error> {
        let exists: i64 = sqlx::query_scalar(
            r#"
            SELECT EXISTS(
                SELECT 1
                FROM jobs
                WHERE version = ? AND agent = ?
                AND status IN (?, ?)
            )
            "#,
        )
        .bind(APP_VERSION)
        .bind(agent.to_string())
        .bind(JobStatus::Pending.to_string())
        .bind(JobStatus::Processing.to_string())
        .fetch_one(&self.pool)
        .await?;

        Ok(exists > 0)
    }

    fn start_of_today_ts(&self) -> i64 {
        let now = chrono::Utc::now();
        let start = now.date_naive().and_hms_opt(0, 0, 0).unwrap();
        start.and_utc().timestamp()
    }

    fn start_of_recent_ts(&self, days: u8) -> i64 {
        let days_back = days.saturating_sub(1) as u64;
        chrono::Utc::now()
            .date_naive()
            .checked_sub_days(chrono::Days::new(days_back))
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap()
            .and_utc()
            .timestamp()
    }
}
