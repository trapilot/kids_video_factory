use async_trait::async_trait;

use crate::provider::*;

pub struct ElevenLabsClient;

#[async_trait]
impl ProviderClient for ElevenLabsClient {
    fn provider(&self) -> Provider {
        Provider::ElevenLabs
    }

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
        status == 429
            || body.contains("RESOURCE_EXHAUSTED")
            || body.contains("quota")
            || body.contains("rate limit")
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
                        if e.is_timeout() {
                            ProviderError::Timeout
                        } else {
                            ProviderError::Network(e.to_string())
                        }
                    })?;
                

                let status = res.status();

                if status == reqwest::StatusCode::UNAUTHORIZED {
                    let body = res.text().await.unwrap_or_default();

                    return Err(
                        ProviderError::InvalidApiKey(body)
                    );
                }

                if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
                    let body = res.text().await.unwrap_or_default();

                    return Err(
                        ProviderError::QuotaExceeded {
                            retry_after: 3600,
                            body,
                        }
                    );
                }

                if status == reqwest::StatusCode::FORBIDDEN {
                    let body = res.text().await.unwrap_or_default();

                    if body.contains("RESOURCE_EXHAUSTED")
                        || body.contains("quota")
                        || body.contains("rate")
                    {
                        return Err(
                            ProviderError::QuotaExceeded {
                                retry_after: 3600,
                                body,
                            }
                        );
                    }

                    return Err(
                        ProviderError::InvalidApiKey(body)
                    );
                }

                if !status.is_success() {
                    let body = res.text().await.unwrap_or_default();

                    return Err(
                        ProviderError::Http {
                            status: status.as_u16(),
                            body,
                        }
                    );
                }

                let bytes = res
                    .bytes()
                    .await
                    .map(|b| b.to_vec())
                    .map_err(|e| e.to_string())
                    .map_err(|e| {
                        ProviderError::Http {
                            status: 500,
                            body: format!("failed to parse response: {}", e),
                        }
                    })?;

                Ok(ProviderResponse::Bytes(bytes))
            }

            _ => Err(ProviderError::NotSupported(format!("{}", Self.provider().clone()))),
        }
    }
}