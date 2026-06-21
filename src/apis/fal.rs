use std::env;

use reqwest::Client;
use serde_json::{json, Value};

pub async fn generate_image(
    client: &Client,
    prompt: &str,
) -> Result<Vec<u8>, String> {
    let api_key = env::var("FAL_API_KEY")
        .map_err(|_| "Missing FAL_API_KEY")?;

    // Submit job
    let submit: Value = client
        .post("https://queue.fal.run/fal-ai/flux/schnell")
        .header("Authorization", format!("Key {}", api_key))
        .json(&json!({ "prompt": prompt }))
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())?;

    let request_id = submit["request_id"]
        .as_str()
        .ok_or_else(|| format!("No request_id: {}", submit))?
        .to_string();

    // Poll result
    loop {
        let result: Value = client
            .get(format!("https://queue.fal.run/fal-ai/flux/schnell/requests/{}/status", request_id))
            .header("Authorization", format!("Key {}", api_key))
            .send()
            .await
            .map_err(|e| e.to_string())?
            .json()
            .await
            .map_err(|e| e.to_string())?;

        match result["status"].as_str() {
            Some("COMPLETED") => break,
            Some("FAILED") => return Err(format!("Fal job failed: {}", result)),
            _ => tokio::time::sleep(tokio::time::Duration::from_millis(500)).await,
        }
    }

    // Fetch final result
    let output: Value = client
        .get(format!("https://queue.fal.run/fal-ai/flux/schnell/requests/{}", request_id))
        .header("Authorization", format!("Key {}", api_key))
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())?;

    let image_url = output["images"][0]["url"]
        .as_str()
        .ok_or_else(|| format!("No image URL: {}", output))?;

    // Download image bytes
    client
        .get(image_url)
        .send()
        .await
        .map_err(|e| e.to_string())?
        .bytes()
        .await
        .map(|b| b.to_vec())
        .map_err(|e| e.to_string())
}