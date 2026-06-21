use reqwest::Client;
use serde_json::json;

use crate::config::*;
use crate::enums::ApiError;


pub async fn generate_tts(
    client: &Client,
    api_key: &str,
    text: &str,
    voice: &str,
    tts_setting: &TtsConfig,
) -> Result<Vec<u8>, ApiError> {
    let url = format!(
        "https://api.elevenlabs.io/v1/text-to-speech/{}",
        voice
    );

    let payload = json!({
        "text": text,
        "model_id": "eleven_multilingual_v2",
        "voice_settings": {
            "speed": &tts_setting.speed,
            "stability": &tts_setting.stability,
            "similarity_boost": &tts_setting.similarity_boost,
        }
    });

    let res = client
        .post(url)
        .header("xi-api-key", api_key)
        .header("Accept", "audio/mpeg")
        .json(&payload)
        .send()
        .await
        .map_err(|e| ApiError::Network(e.to_string()))?;

    let status = res.status();
    if status == reqwest::StatusCode::UNAUTHORIZED {
        let body = res.text().await.unwrap_or_default();
        if body.contains("quota_exceeded") {
            return Err(
                ApiError::RateLimited(format!("[ElevenLabs] : {}", body) , 3600)
            );
        }
        return Err(
            ApiError::Unauthorized(format!("[ElevenLabs]: {}", body))
        );
    }
    if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
        let body = res.text().await.unwrap_or_default();
        return Err(
            ApiError::RateLimited(format!("[ElevenLabs]: {}", body) , 3600)
        );
    }
    if status == reqwest::StatusCode::FORBIDDEN {
        let body = res.text().await.unwrap_or_default();
        return Err(
            ApiError::RateLimited(format!("[ElevenLabs]: {}", body) , 3600)
        );
    }
    if !status.is_success() {
        let body = res.text().await.unwrap_or_default();
        return Err(
            ApiError::InvalidResponse(format!("[ElevenLabs] HTTP {}: {}", status, body))
        );
    }

    let bytes = res
        .bytes()
        .await
        .map(|b| b.to_vec())
        .map_err(|e| e.to_string())
        .map_err(|e| {
            ApiError::InvalidResponse(
                format!("[ElevenLabs] Invalid JSON: {}", e)
            )
        })?;

    Ok(bytes)
}