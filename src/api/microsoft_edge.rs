use std::env;

use reqwest::Client;

pub async fn generate_tts(
    client: &Client,
    text: &str,
    _voice: &str,
) -> Result<Vec<u8>, String> {
let voice =
        env::var("EDGE_VOICE").unwrap_or_else(|_| "vi-VN-AriaNeural".to_string());

    let url = format!(
        "https://speech.platform.bing.com/consumer/speech/synthesize/readaloud/edge/v1?VoiceName={}",
        voice
    );

    let ssml = format!(
        r#"
        <speak version='1.0' xml:lang='vi-VN'>
            <voice name='{}'>{}</voice>
        </speak>
        "#,
        voice, text
    );

    let res = client
        .post(url)
        .header("Content-Type", "application/ssml+xml")
        .header(
            "X-Microsoft-OutputFormat",
            "audio-24khz-48kbitrate-mono-mp3",
        )
        .header(
            "User-Agent",
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64)",
        )
        .body(ssml)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    let bytes = res
        .bytes()
        .await
        .map_err(|e| e.to_string())?
        .to_vec();

    Ok(bytes)
}