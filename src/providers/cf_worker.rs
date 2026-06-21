use async_trait::async_trait;
use base64::Engine;

use crate::provider::*;

pub struct CFWorkerClient;

#[async_trait]
impl ProviderClient for CFWorkerClient {
    fn provider(&self) -> Provider {
        Provider::CFWorker
    }

    async fn call(
        &self,
        req: &ProviderRequest,
        credential: &ApiCredential,
    ) -> Result<ProviderResponse, ProviderError> {
        match req {
            ProviderRequest::Image(req) => {
                self.image(req, credential).await
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

impl CFWorkerClient {
    async fn image(
        &self,
        req: &ImageRequest,
        credential: &ApiCredential,
    ) -> Result<ProviderResponse, ProviderError> {

        match req {
            ImageRequest { prompt, num_steps, guidance, .. } => {
                let payload = serde_json::json!({
                    "prompt": prompt,
                    "num_steps": num_steps,
                    "guidance": guidance,
                });


                let Some(account_id) = &credential.account_id else {
                    return Err(ProviderError::InvalidAccount(format!("CFWorkerClient")));
                };

                let url = format!(
                    "https://api.cloudflare.com/client/v4/accounts/{}/ai/run/@cf/black-forest-labs/flux-1-schnell",
                    account_id
                );

                let res = reqwest::Client::new()
                    .post(url)
                    .bearer_auth(&credential.api_key)
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
                        ProviderError::InvalidResponse(
                            format!("[CF Worker] Invalid JSON: {}", e)
                        )
                    })?;


                let b64: &str = data["result"]["image"]
                    .as_str()
                    .ok_or_else(|| {
                        ProviderError::InvalidResponse(
                            format!("[CF Worker] Failed to parse JSON: {}", data)
                        )
                    })?;

                let decode = base64::engine::general_purpose::STANDARD
                    .decode(b64)
                    .map_err(|e| {
                        ProviderError::InvalidResponse(
                            format!("[CF Worker] Failed to decode response: {}", e)
                        )
                    });

                match decode {
                    Ok(bytes) => Ok(ProviderResponse::Bytes(bytes)),
                    _ => Err(ProviderError::InvalidResponse(
                            format!("[CF Worker] Failed to retrive")
                        ))
                }
            }

            _ => Err(ProviderError::NotSupported(format!("{}", Self.provider().clone()))),
        }
    }
}