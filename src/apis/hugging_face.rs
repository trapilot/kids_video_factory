use std::env;
use reqwest::{Client, StatusCode};
use serde_json::{json, Value};

fn get_hf_api_key() -> Result<String, String> {
    env::var("HF_API_KEY").map_err(|_| "Missing HF_API_KEY environment variable".to_string())
}

async fn handle_hf_error(res: reqwest::Response) -> Result<reqwest::Response, String> {
    if res.status().is_success() {
        Ok(res)
    } else {
        let status = res.status();
        let error_body = res.text().await.unwrap_or_else(|_| "Unknown error body".to_string());
        Err(format!("Hugging Face API Error: HTTP {} - {}", status, error_body))
    }
}

pub async fn generate_script(
    client: &Client,
    system: &str,
    user: &str,
    is_json: bool,
) -> Result<String, String> {
    let api_key = get_hf_api_key()?;

    let url = "https://api-inference.huggingface.co/models/mistralai/Mistral-7B-Instruct-v0.3";

    let prompt = format!("<s>[INST] System: {}\n\nUser: {} [/INST]", system, user);

    let payload = json!({
        "inputs": prompt,
        "parameters": {
            "temperature": if is_json { 0.1 } else { 0.8 },
            "return_full_text": false,
            "max_new_tokens": 1024
        }
    });

    let res = client
        .post(url)
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&payload)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    let res = handle_hf_error(res).await?;
    let json_res: Value = res.json().await.map_err(|e| e.to_string())?;

    let text = json_res[0]["generated_text"]
        .as_str()
        .ok_or_else(|| format!("Failed to parse HF response: {}", json_res))?;

    Ok(text.trim().to_string())
}

pub async fn generate_image(
    client: &Client,
    prompt: &str,
) -> Result<Vec<u8>, String> {
    let api_key = get_hf_api_key()?;

    let url = "https://api-inference.huggingface.co/models/black-forest-labs/FLUX.1-schnell";

    let payload = json!({ "inputs": prompt });

    let res = client
        .post(url)
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&payload)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    let res = handle_hf_error(res).await?;

    let bytes = res
        .bytes()
        .await
        .map_err(|e| e.to_string())?
        .to_vec();

    Ok(bytes)
}

pub async fn generate_tts(
    client: &Client,
    text: &str,
    _voice: &str,
) -> Result<Vec<u8>, String> {
    let api_key = get_hf_api_key()?;

    let url = "https://api-inference.huggingface.co/models/espnet/kan-bayashi_ljspeech_vits";

    let payload = json!({ "inputs": text });

    let res = client
        .post(url)
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&payload)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    let res = handle_hf_error(res).await?;

    let bytes = res
        .bytes()
        .await
        .map_err(|e| e.to_string())?
        .to_vec();

    Ok(bytes)
}