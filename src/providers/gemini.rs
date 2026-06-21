use async_trait::async_trait;

use crate::provider::*;

pub struct GeminiClient;

#[async_trait]
impl ProviderClient for GeminiClient {
    fn provider(&self) -> Provider {
        Provider::Gemini
    }

    async fn call(
        &self,
        req: &ProviderRequest,
        credential: &ApiCredential,
    ) -> Result<ProviderResponse, ProviderError> {
        match req {
            ProviderRequest::Script(req) => {
                self.script(req, credential).await
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

impl GeminiClient {
    async fn script(
        &self,
        req: &ScriptRequest,
        credential: &ApiCredential,
    ) -> Result<ProviderResponse, ProviderError> {

        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/gemini-2.5-flash:generateContent?key={}",
            credential.api_key
        );

        let mut payload = serde_json::json!({
            "system_instruction": {
                "parts": [{
                    "text": req.system
                }]
            },
            "contents": [{
                "parts": [{
                    "text": req.prompt
                }]
            }],
            "generationConfig": {
                "temperature": req.temperature.unwrap_or(
                    if req.json_mode { 0.1 } else { 0.8 }
                )
            }
        });

        if let Some(max_tokens) = req.max_tokens {
            payload["generationConfig"]["maxOutputTokens"] =
                serde_json::json!(max_tokens);
        }

        if req.json_mode {
            payload["generationConfig"]["responseMimeType"] =
                serde_json::json!("application/json");
        }

        let res = reqwest::Client::new()
            .post(url)
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

        let data: serde_json::Value = res
            .json()
            .await
            .map_err(|e| {
                ProviderError::Http {
                    status: 500,
                    body: format!("invalid json: {}", e),
                }
            })?;

        let text = data["candidates"][0]["content"]["parts"][0]["text"]
            .as_str()
            .ok_or_else(|| {
                ProviderError::Http {
                    status: 500,
                    body: format!("failed to parse response: {}", data),
                }
            })?;

        Ok(ProviderResponse::Text(text.trim().to_string()))
    }
}