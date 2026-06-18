use std::env;
use reqwest::Client;
use serde_json::json;

use crate::config::CONFIG;

pub async fn generate_tts(
    client: &Client,
    text: &str,
    voice: &str,
) -> Result<Vec<u8>, String> {
    let api_key = env::var("ELEVENLABS_API_KEY")
        .map_err(|_| "Missing ELEVENLABS_API_KEY")?;
    
    let url = format!(
        "https://api.elevenlabs.io/v1/text-to-speech/{}",
        voice
    );

    let payload = json!({
        "text": text,
        "model_id": "eleven_multilingual_v2",
        "voice_settings": {
            "speed": &CONFIG.tts.speed,
            "stability": &CONFIG.tts.stability,
            "similarity_boost": &CONFIG.tts.similarity_boost,
        }
    });

    let res = client
        .post(&url)
        .header("xi-api-key", api_key)
        .header("Accept", "audio/mpeg")
        .json(&payload)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !res.status().is_success() {
        let err = res.text().await.unwrap_or_default();
        return Err(format!("ElevenLabs error: {}", err));
    }

    res.bytes()
        .await
        .map(|b| b.to_vec())
        .map_err(|e| e.to_string())
}