use base64::Engine;
use reqwest::Client;
use serde_json::{json, Value};

use crate::config::*;
use crate::enums::ApiError;

pub async fn generate_image(
    client: &Client,
    account_id: &str,
    account_key: &str,
    prompt: &str,
    diffusion: &DiffusionParams,
) -> Result<Vec<u8>, ApiError> {
    let url = format!(
        "https://api.cloudflare.com/client/v4/accounts/{}/ai/run/@cf/black-forest-labs/flux-1-schnell",
        account_id
    );

    let payload = json!({
        "prompt": prompt,
        "num_steps": &diffusion.num_steps,
        "guidance": &diffusion.guidance,
    });
    
    let res = client
        .post(&url)
        .bearer_auth(account_key)
        .json(&payload)
        .send()
        .await
        .map_err(|e| ApiError::Network(e.to_string()))?;

    let status = res.status();
    if status == reqwest::StatusCode::UNAUTHORIZED {
        let body = res.text().await.unwrap_or_default();
        return Err(
            ApiError::Unauthorized(format!("[Cloudflare]: {}", body))
        );
    }
    if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
        let body = res.text().await.unwrap_or_default();
        return Err(
            ApiError::RateLimited(format!("[Cloudflare]: {}", body) , 3600)
        );
    }
    if status == reqwest::StatusCode::FORBIDDEN {
        let body = res.text().await.unwrap_or_default();
        return Err(
            ApiError::RateLimited(format!("[Cloudflare]: {}", body) , 3600)
        );
    }
    if !status.is_success() {
        let body = res.text().await.unwrap_or_default();
        return Err(
            ApiError::InvalidResponse(format!("[Cloudflare] HTTP {}: {}", status, body))
        );
    }

    let data: Value = res
        .json()
        .await
        .map_err(|e| {
            ApiError::InvalidResponse(
                format!("[Cloudflare] Invalid JSON: {}", e)
            )
        })?;


    let b64 = data["result"]["image"]
        .as_str()
        .ok_or_else(|| {
            ApiError::InvalidResponse(
                format!("[Cloudflare] Failed to parse response: {}", data)
            )
        })?;

    base64::engine::general_purpose::STANDARD
        .decode(b64)
        .map_err(|e| {
            ApiError::InvalidResponse(
                format!("[Cloudflare] Failed to decode response: {}", e)
            )
        })
}