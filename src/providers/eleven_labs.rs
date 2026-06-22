use async_trait::async_trait;

use crate::provider::*;

pub struct ElevenLabsClient;

#[async_trait]
impl ProviderClient for ElevenLabsClient {
    async fn call(
        &self,
        req: &ProviderRequest,
        credential: &ApiCredential,
    ) -> Result<ProviderResponse, ProviderError> {
        match req {
            ProviderRequest::Audio(req) => {
                self.audio(req, credential).await
            },

            _ => Err(ProviderError::NotSupported(format!("{}", Self.provider().clone()))),
        }
    }
    
    fn is_quota_error(
        &self,
        status: u16,
        body: &str,
    ) -> bool {
        status == reqwest::StatusCode::TOO_MANY_REQUESTS
            || body.contains("quota_exceeded")
            || body.contains("exceeds your quota")
            || body.contains("rate limit")
    }
    
    fn is_auth_error(
        &self,
        status: u16,
        body: &str,
    ) -> bool {
        status == reqwest::StatusCode::UNAUTHORIZED
    }

    
    fn provider(&self) -> Provider {
        Provider::ElevenLabs
    }

    fn config(&self) -> ProviderConfig {
        ProviderConfig {
            reset_daily: false,
            reset_monthly: true,
            reset_time: None,
            daily_limit: None,
            retry_after: 5,
        }
    }
}

impl ElevenLabsClient {
    async fn audio(
        &self,
        req: &AudioRequest,
        credential: &ApiCredential,
    ) -> Result<ProviderResponse, ProviderError> {

        match req {
            AudioRequest { text, voice_id, speed, stability, similarity_boost , .. } => {
                let payload = serde_json::json!({
                    "text": text,
                    "model_id": "eleven_multilingual_v2",
                    "voice_settings": {
                        "speed": speed,
                        "stability": stability,
                        "similarity_boost": similarity_boost,
                    },
                });

                let url = format!(
                    "https://api.elevenlabs.io/v1/text-to-speech/{}",
                    voice_id.as_deref().unwrap_or("default")
                );

                let res = reqwest::Client::new()
                    .post(url)
                    .header("xi-api-key", &credential.api_key)
                    .header("Accept", "audio/mpeg")
                    .json(&payload)
                    .send()
                    .await
                    .map_err(|e| {
                        ProviderError::ClientResponse {
                            client: "ElevenLabs".to_string(),
                            error: e.to_string(),
                        }
                    })?;
                
                let bytes =
                    self.read_bytes(res).await?;

                Ok(ProviderResponse::Bytes(bytes))
            }

            _ => Err(ProviderError::NotSupported(format!("{}", Self.provider().clone()))),
        }
    }
}