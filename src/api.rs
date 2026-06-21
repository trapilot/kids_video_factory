use crate::errors;

pub struct HttpClient {
    client: reqwest::Client,
}

impl HttpClient {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }

    pub fn get(&self, url: impl Into<String>) -> RequestBuilder {
        RequestBuilder {
            inner: self.client.get(url.into()),
        }
    }

    pub fn post(&self, url: impl Into<String>) -> RequestBuilder {
        RequestBuilder {
            inner: self.client.post(url.into()),
        }
    }
}

pub struct RequestBuilder {
    inner: reqwest::RequestBuilder,
}

impl RequestBuilder {
    pub fn bearer(self, token: &str) -> Self {
        Self {
            inner: self.inner.bearer_auth(token),
        }
    }

    pub fn json<T: serde::Serialize>(self, body: &T) -> Self {
        Self {
            inner: self.inner.json(body),
        }
    }

    pub async fn send(self) -> Result<reqwest::Response, errors::ApiError> {
        self.inner
            .send()
            .await
            .map_err(|e| errors::ApiError::Internal(e.to_string()))
    }

    pub async fn send_json<T: serde::de::DeserializeOwned>(
        self,
    ) -> Result<T, errors::ApiError> {
        let resp = self.inner
            .send()
            .await
            .map_err(|e| errors::ApiError::Network(e.to_string()))?;
        
        Ok(
            resp.json::<T>()
                .await
                .map_err(|e| errors::ApiError::Internal(e.to_string()))?
        )
    }
}
