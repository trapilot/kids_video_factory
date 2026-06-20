use base64::Engine;
use reqwest::Client;
use serde_json::{json, Value};

use crate::enums::ApiError;


pub async fn generate_script(
    client: &Client,
    api_key: &str,
    system: &str,
    user: &str,
    is_json: bool,
) -> Result<String, ApiError> {
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

    let res = client
        .post(url)
        .json(&payload)
        .send()
        .await
        .map_err(|e| ApiError::Network(e.to_string()))?;
        
    let status = res.status();
    if status == reqwest::StatusCode::UNAUTHORIZED {
        let body = res.text().await.unwrap_or_default();
        return Err(
            ApiError::Unauthorized(format!("[Gemini]: {}", body))
        );
    }
    if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
        let body = res.text().await.unwrap_or_default();
        return Err(
            ApiError::RateLimited(format!("[Gemini]: {}", body) , 3600)
        );
    }
    if status == reqwest::StatusCode::FORBIDDEN {
        let body = res.text().await.unwrap_or_default();
        return Err(
            ApiError::RateLimited(format!("[Gemini]: {}", body) , 3600)
        );
    }
    if !status.is_success() {
        let body = res.text().await.unwrap_or_default();
        return Err(
            ApiError::InvalidResponse(format!("[Gemini] HTTP {}: {}", status, body))
        );
    }

    let data: Value = res
        .json()
        .await
        .map_err(|e| {
            ApiError::InvalidResponse(
                format!("[Gemini] Invalid JSON: {}", e)
            )
        })?;
    
    data["candidates"][0]["content"]["parts"][0]["text"]
        .as_str()
        .map(|s| s.trim().to_string())
        .ok_or_else(|| {
            ApiError::InvalidResponse(
                format!("[Gemini] Failed to parse response: {}", data)
            )
        })
}

pub async fn generate_tts(
    client: &Client,
    api_key: &str,
    text: &str,
    _voice: &str,
) -> Result<Vec<u8>, ApiError> {
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

    let res = client
        .post(url)
        .json(&payload)
        .send()
        .await
        .map_err(|e| ApiError::Network(e.to_string()))?;

    let status = res.status();
    if status == reqwest::StatusCode::UNAUTHORIZED {
        let body = res.text().await.unwrap_or_default();
        return Err(
            ApiError::Unauthorized(format!("[Gemini]: {}", body))
        );
    }
    if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
        let body = res.text().await.unwrap_or_default();
        return Err(
            ApiError::RateLimited(format!("[Gemini]: {}", body) , 3600)
        );
    }
    if status == reqwest::StatusCode::FORBIDDEN {
        let body = res.text().await.unwrap_or_default();
        return Err(
            ApiError::RateLimited(format!("[Gemini]: {}", body) , 3600)
        );
    }
    if !status.is_success() {
        let body = res.text().await.unwrap_or_default();
        return Err(
            ApiError::InvalidResponse(format!("[Gemini] HTTP {}: {}", status, body))
        );
    }

    let data: Value = res
        .json()
        .await
        .map_err(|e| {
            ApiError::InvalidResponse(
                format!("[Gemini] Invalid JSON: {}", e)
            )
        })?;

    let audio_b64 =
        data["candidates"][0]["content"]["parts"][0]["inlineData"]["data"]
            .as_str()
            .ok_or_else(|| {
                ApiError::InvalidResponse(
                    format!("[Gemini] Invalid JSON: {}", data)
                )
            })?;

    let audio =
        base64::engine::general_purpose::STANDARD
            .decode(audio_b64)
            .map_err(|e| {
                ApiError::InvalidResponse(
                    format!("[Gemini] Invalid JSON: {}", e)
                )
            })?;

    Ok(audio)
}

pub async fn generate_image(
    client: &Client,
    api_key: &str,
    prompt: &str,
) -> Result<Vec<u8>, ApiError> {
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

    let res = client
        .post(url)
        .json(&payload)
        .send()
        .await
        .map_err(|e| ApiError::Network(e.to_string()))?;
        
    let status = res.status();
    if status == reqwest::StatusCode::UNAUTHORIZED {
        let body = res.text().await.unwrap_or_default();
        return Err(
            ApiError::Unauthorized(format!("[Gemini]: {}", body))
        );
    }
    if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
        let body = res.text().await.unwrap_or_default();
        return Err(
            ApiError::RateLimited(format!("[Gemini]: {}", body) , 3600)
        );
    }
    if status == reqwest::StatusCode::FORBIDDEN {
        let body = res.text().await.unwrap_or_default();
        return Err(
            ApiError::RateLimited(format!("[Gemini]: {}", body) , 3600)
        );
    }
    if !status.is_success() {
        let body = res.text().await.unwrap_or_default();
        return Err(
            ApiError::InvalidResponse(format!("[Gemini] HTTP {}: {}", status, body))
        );
    }

    let data: Value = res
        .json()
        .await
        .map_err(|e| {
            ApiError::InvalidResponse(
                format!("[Gemini] Invalid JSON: {}", e)
            )
        })?;

    let image_b64 = data["candidates"][0]["content"]["parts"]
        .as_array()
        .and_then(|parts| {
            parts.iter().find_map(|p| {
                p["inlineData"]["data"].as_str()
            })
        })
        .ok_or_else(|| {
            ApiError::InvalidResponse(
                format!("[Gemini] Invalid JSON: {}", data)
            )
        })?;

    let image_bytes =
        base64::engine::general_purpose::STANDARD
            .decode(image_b64)
            .map_err(|e| {
            ApiError::InvalidResponse(
                format!("[Gemini] Invalid JSON: {}", e)
            )
        })?;
    
    Ok(image_bytes)
}