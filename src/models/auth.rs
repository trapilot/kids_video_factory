
use serde::{Deserialize, Serialize};


#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct OAuthToken {
    pub provider: String,
    pub client_id: String,
    pub client_secret: String,
    pub access_token: Option<String>,
    pub refresh_token: Option<String>,
    pub auth_code: Option<String>,
    pub expires_at: Option<i64>,
    pub updated_at: i64,
}