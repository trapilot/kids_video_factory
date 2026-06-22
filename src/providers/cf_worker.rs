use async_trait::async_trait;
use base64::Engine;

use crate::provider::*;

pub struct CFWorkerClient;

#[async_trait]
impl ProviderClient for CFWorkerClient {
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
        status == reqwest::StatusCode::TOO_MANY_REQUESTS
            || body.contains("RESOURCE_EXHAUSTED")
            || body.contains("quota")
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
        Provider::CFWorker
    }

    fn config(&self) -> ProviderConfig {
        ProviderConfig {
            reset_daily: true,
            reset_monthly: false,
            reset_time: Some(60),
            daily_limit: Some(100000),
            retry_after: 5,
        }
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
                    return Err(ProviderError::ClientUnauthorized {
                        client: "CFWorkerClient".to_string(),
                    });
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
                        ProviderError::ClientResponse {
                            client: "CFWorker".to_string(),
                            error: e.to_string(),
                        }
                    })?;

                let data: serde_json::Value =
                    self.read_json(res).await?;

                let b64: &str = data["result"]["image"]
                    .as_str()
                    .ok_or_else(|| {
                        ProviderError::ClientResponse {
                            client: "CFWorker".to_string(),
                            error: "Invalid JSON".to_string(),
                        }
                    })?;

                let decode = base64::engine::general_purpose::STANDARD
                    .decode(b64)
                    .map_err(|e| {
                        ProviderError::ClientResponse {
                            client: "CFWorker".to_string(),
                            error: e.to_string(),
                        }
                    });

                match decode {
                    Ok(bytes) => Ok(ProviderResponse::Bytes(bytes)),
                    _ => Err(
                        ProviderError::ClientResponse {
                            client: "CFWorker".to_string(),
                            error: "Invalid reponse type".to_string(),
                        }
                    )
                }
            }

            _ => Err(ProviderError::NotSupported(format!("{}", Self.provider().clone()))),
        }
    }
}