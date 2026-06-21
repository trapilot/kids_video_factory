use std::env;

use base64::Engine;
use reqwest::Client;
use serde_json::{json, Value};

fn gcp_auth() -> Result<(String, String), String> {
    let token = env::var("GOOGLE_ACCESS_TOKEN")
        .map_err(|_| "Missing GOOGLE_ACCESS_TOKEN")?;
    let project = env::var("GOOGLE_PROJECT_ID")
        .map_err(|_| "Missing GOOGLE_PROJECT_ID")?;
    Ok((token, project))
}

pub async fn generate_script(
    client: &Client,
    system: &str,
    user: &str,
    is_json: bool,
) -> Result<String, String> {
    let (token, project) = gcp_auth()?;

    let url = format!(
        "https://us-central1-aiplatform.googleapis.com/v1/projects/{}/locations/us-central1/publishers/google/models/gemini-2.5-flash:generateContent",
        project
    );

    let mut payload = json!({
        "system_instruction": {
            "parts": [{ "text": system }]
        },
        "contents": [{
            "parts": [{ "text": user }]
        }],
        "generationConfig": {
            "temperature": if is_json { 0.1 } else { 0.8 }
        }
    });

    if is_json {
        payload["generationConfig"]["responseMimeType"] =
            json!("application/json");
    }

    let res: Value = client
        .post(url)
        .bearer_auth(token)
        .json(&payload)
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())?;

    res["candidates"][0]["content"]["parts"][0]["text"]
        .as_str()
        .map(|s| s.trim().to_string())
        .ok_or_else(|| format!("Failed to parse Vertex AI response: {}", res))
}

pub async fn generate_tts(
    client: &Client,
    text: &str,
    _voice: &str,
) -> Result<Vec<u8>, String> {
    let (token, _) = gcp_auth()?;

    let url = "https://texttospeech.googleapis.com/v1/text:synthesize";

    let payload = json!({
        "input": { "text": text },
        "voice": {
            "languageCode": "vi-VN",
            "name": "vi-VN-Standard-A"   // vi-VN-Wavenet-A
        },
        "audioConfig": {
            "audioEncoding": "MP3"
        }
    });

    let res: Value = client
        .post(url)
        .bearer_auth(token)
        .json(&payload)
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())?;

    let audio_b64 = res["audioContent"]
        .as_str()
        .ok_or_else(|| format!("Invalid Cloud TTS response: {}", res))?;

    base64::engine::general_purpose::STANDARD
        .decode(audio_b64)
        .map_err(|e| e.to_string())
}

pub async fn generate_image(
    client: &Client,
    prompt: &str,
) -> Result<Vec<u8>, String> {
    let (token, project) = gcp_auth()?;

    let url = format!(
        "https://us-central1-aiplatform.googleapis.com/v1/projects/{}/locations/us-central1/publishers/google/models/imagen-3.0-generate-002:predict",
        project
    );

    let payload = json!({
        "instances": [{ "prompt": prompt }],
        "parameters": {
            "sampleCount": 1,
            "aspectRatio": "1:1"
        }
    });

    let res: Value = client
        .post(url)
        .bearer_auth(token)
        .json(&payload)
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())?;

    let image_b64 = res["predictions"]
        .as_array()
        .and_then(|preds| preds.first())
        .and_then(|p| p["bytesBase64Encoded"].as_str())
        .ok_or_else(|| format!("No image found in Imagen response: {}", res))?;

    base64::engine::general_purpose::STANDARD
        .decode(image_b64)
        .map_err(|e| e.to_string())
}