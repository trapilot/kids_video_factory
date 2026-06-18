use std::env;

use base64::Engine;
use reqwest::Client;
use serde_json::{json, Value};

pub async fn generate_image(
    client: &Client,
    prompt: &str,
) -> Result<Vec<u8>, String> {
    let api_token = env::var("CF_API_TOKEN")
        .map_err(|_| "Missing CF_API_TOKEN")?;
    let account_id = env::var("CF_ACCOUNT_ID")
        .map_err(|_| "Missing CF_ACCOUNT_ID")?;

    let url = format!(
        "https://api.cloudflare.com/client/v4/accounts/{}/ai/run/@cf/black-forest-labs/flux-1-schnell",
        account_id
    );

    let payload = json!({
        "prompt": prompt,
        "num_steps": 4      // Schnell only needs 4 steps
    });
    
    let res: Value = client
        .post(&url)
        .bearer_auth(api_token)
        .json(&payload)
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())?;

    let image_b64 = res["result"]["image"]
        .as_str()
        .ok_or_else(|| format!("No image in CF response: {}", res))?;

    base64::engine::general_purpose::STANDARD
        .decode(image_b64)
        .map_err(|e| e.to_string())
}