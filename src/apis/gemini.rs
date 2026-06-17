use std::env;

use base64::Engine;
use reqwest::Client;
use serde_json::{json, Value};

pub async fn generate_script(
    client: &Client,
    system: &str,
    user: &str,
    is_json: bool,
) -> Result<String, String> {

    let api_key =
        env::var("GEMINI_API_KEY")
            .map_err(|_| "Missing GEMINI_API_KEY")?;

    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/gemini-2.5-flash:generateContent?key={}",
        api_key
    );

    let mut payload = json!({
        "system_instruction": {
            "parts": [{
                "text": system
            }]
        },
        "contents": [{
            "parts": [{
                "text": user
            }]
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
        .ok_or_else(|| format!("Failed to parse Gemini response: {}", res))
}

pub async fn generate_tts(
    client: &Client,
    text: &str,
    speaker: Option<&str>,
) -> Result<Vec<u8>, String> {
    let api_key =
        env::var("GEMINI_API_KEY")
            .map_err(|_| "Missing GEMINI_API_KEY")?;

    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/gemini-2.5-flash-preview-tts:generateContent?key={}",
        api_key
    );

    let payload = json!({
        "contents": [{
            "parts": [{
                "text": text
            }]
        }],
        "generationConfig": {
            "responseModalities": ["AUDIO"]
        }
    });

    let res: Value = client
        .post(url)
        .json(&payload)
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())?;

    let audio_b64 =
        res["candidates"][0]["content"]["parts"][0]["inlineData"]["data"]
            .as_str()
            .ok_or_else(|| format!("Invalid Gemini TTS response: {}", res))?;

    let audio =
        base64::engine::general_purpose::STANDARD
            .decode(audio_b64)
            .map_err(|e| e.to_string())?;

    Ok(audio)
}

pub async fn generate_image(
    client: &Client,
    prompt: &str,
) -> Result<Vec<u8>, String> {

    let api_key =
        env::var("GEMINI_API_KEY")
            .map_err(|_| "Missing GEMINI_API_KEY")?;

    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/gemini-2.5-flash-image:generateContent?key={}",
        api_key
    );

    let payload = json!({
        "contents": [{
            "parts": [{
                "text": prompt
            }]
        }],
        "generationConfig": {
            "responseModalities": ["TEXT", "IMAGE"]
        }
    });

    let res: Value = client
        .post(url)
        .json(&payload)
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())?;

    let image_b64 = res["candidates"][0]["content"]["parts"]
        .as_array()
        .and_then(|parts| {
            parts.iter().find_map(|p| {
                p["inlineData"]["data"].as_str()
            })
        })
        .ok_or_else(|| format!("No image found in Gemini response: {}", res))?;

    let image_bytes =
        base64::engine::general_purpose::STANDARD
            .decode(image_b64)
            .map_err(|e| e.to_string())?;
    
    Ok(image_bytes)
}