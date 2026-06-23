use std::collections::HashMap;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::{Mutex, RwLock};
use thiserror::Error;
use serde::{Deserialize, Serialize};
use strum_macros::{Display, EnumString};


#[derive(Debug, Clone, Display, EnumString, Serialize, Deserialize, Eq, PartialEq, Hash)]
#[strum(serialize_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum Provider {
    Gemini,
    CFWorker,
    ElevenLabs,
}

#[derive(Debug, Clone, Copy)]
pub struct ProviderConfig {
    pub daily_limit: Option<u32>,
    pub retry_after: u64,
    
    // requests_per_minute: u64,
    // tokens_per_minute: u64,
    // requests_per_day: u64,

    pub reset_daily: bool,
    pub reset_monthly: bool,

    pub reset_time: Option<u32>, // hour * 3600 + minutes * 60 + seconds,
}

pub struct ScriptRequest {
    pub system: String,
    pub prompt: String,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
    pub json_mode: bool,
}

pub struct ImageRequest {
    pub prompt: String,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub num_steps: Option<u32>,
    pub guidance: Option<f32>,
}

pub struct AudioRequest {
    pub text: String,
    pub voice_id: Option<String>,
    pub language: Option<String>,
    pub speed: Option<f32>,
    pub stability: Option<f32>,
    pub similarity_boost: Option<f32>,
    pub format: Option<AudioFormat>,
}

pub enum AudioFormat {
    Mp3,
    Wav,
    Ogg,
}

pub enum ProviderRequest {
    Script(ScriptRequest),
    Image(ImageRequest),
    Audio(AudioRequest),
    // Video(VideoRequest),
    // Embedding(EmbeddingRequest),
}

pub enum ProviderResponse {
    Text(String),
    Bytes(Vec<u8>),
}

#[derive(Debug, Error)]
pub enum ProviderError {
    #[error("[{provider}] Provider http error {status} --> {body}")]
    Http { provider: String, status: u16, body: String },

    #[error("[{provider}] Provider request exceeded {retry_after} --> {body}")]
    RequestExceeded { provider: String, retry_after: u64, body: String },

    #[error("[{provider}] Provider invalid api keys --> {body}")]
    RequestUnauthorized { provider: String, body: String },

    #[error("[{0}] Provider all keys exhausted")]
    AllKeysExhausted(String),

    #[error("[{provider}] Provider response error {error}")]
    InvalidResponse { provider: String, error: String },

    #[error("Provider reponse not match")]
    UnexpectedResponse,

    #[error("[{0}] Provider does not support")]
    NotSupported(String),

    #[error("[{client}] Provider client invalid api keys")]
    ClientUnauthorized { client: String },

    #[error("[{client}] Provider client response error {error}")]
    ClientResponse { client: String, error: String },
}

#[derive(Debug, Clone)]
pub struct ApiCredential {
    pub api_key: String,
    pub account_id: Option<String>,
}

#[derive(Debug)]
pub struct ApiKeyState {
    pub credential: ApiCredential,

    pub daily_used: u32,
    pub daily_limit: Option<u32>,

    pub blocked_until: u64,
    pub last_reset_day: u64,
}

#[derive(Debug)]
pub struct CircuitBreaker {
    pub failure_count: u32,
    pub last_failure_time: u64,
    pub is_open: bool,
}

#[derive(Debug)]
pub struct ProviderState {
    pub running: u32,
    pub maximum: u32,

    pub next_index: usize,
    pub keys: Vec<ApiKeyState>,

    pub circuit: CircuitBreaker,
}

pub struct ProviderRuntime {
    pub state: ProviderState,
    pub config: ProviderConfig,
    pub client: Arc<dyn ProviderClient>,
}

#[derive(Clone)]
pub struct ProviderManager {
    inner: Arc<RwLock<HashMap<Provider, Arc<Mutex<ProviderRuntime>>>>>,
}

#[derive(Clone)]
pub struct ProviderGuard {
    provider: Provider,
    credential: ApiCredential,
    client: Arc<dyn ProviderClient>,
    manager: ProviderManager,
}

#[async_trait::async_trait]
pub trait ProviderClient: Send + Sync {
    fn provider(&self) -> Provider;
    fn config(&self) -> ProviderConfig;

    async fn call(
        &self,
        req: &ProviderRequest,
        credential: &ApiCredential,
    ) -> Result<ProviderResponse, ProviderError>;

    fn is_auth_error(&self, status: u16, body: &str) -> bool;
    fn is_quota_error(&self, status: u16, body: &str) -> bool;

    async fn read_json<T>(
        &self,
        res: reqwest::Response,
    ) -> Result<T, ProviderError>
    where
        T: serde::de::DeserializeOwned,
        Self: Sized,
    {
        let status = res.status();

        let body = res.text().await.unwrap_or_default();

        self.ensure_success(status, &body)?;

        serde_json::from_str(&body).map_err(|e| {
            ProviderError::InvalidResponse {
                provider: self.provider().to_string(),
                error: e.to_string(),
            }
        })
    }

    async fn read_bytes(
        &self,
        res: reqwest::Response,
    ) -> Result<Vec<u8>, ProviderError>
    {
        let status = res.status();

        let bytes = res.bytes().await.map_err(|e| {
            ProviderError::InvalidResponse {
                provider: self.provider().to_string(),
                error: e.to_string(),
            }
        })?;

        if !status.is_success() {
            let body = String::from_utf8_lossy(&bytes);

            self.ensure_success(status, &body)?;
        }

        Ok(bytes.to_vec())
    }

    fn ensure_success(
        &self,
        status: reqwest::StatusCode,
        body: &str,
    ) -> Result<(), ProviderError> {
        let code = status.as_u16();

        if self.is_auth_error(code, body) {
            return Err(ProviderError::RequestUnauthorized {
                provider: self.provider().to_string(),
                body: body.to_string(),
            });
        }

        if self.is_quota_error(code, body) {
            let config = self.config();
            return Err(ProviderError::RequestExceeded {
                provider: self.provider().to_string(),
                retry_after: config.retry_after,
                body: body.to_string(),
            });
        }

        if !status.is_success() {
            return Err(ProviderError::Http {
                provider: self.provider().to_string(),
                status: code,
                body: body.to_string(),
            });
        }

        Ok(())
    }
}

impl ProviderManager {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn register_with_keys(
        &self,
        client: Arc<dyn ProviderClient>,
        concurrency: u32,
        env_prefix: &str,
    ) {
        let provider = client.provider();
        let config = client.config();

        let runtime = Arc::new(Mutex::new(ProviderRuntime {
            config,
            client,
            state: ProviderState {
                running: 0,
                maximum: concurrency,
                next_index: 0,
                keys: vec![],
                circuit: CircuitBreaker {
                    failure_count: 0,
                    last_failure_time: 0,
                    is_open: false,
                },
            },
        }));

        {
            // Only lock write operations when hardcoded at app startup.
            let mut map = self.inner.write().await;
            map.insert(provider.clone(), runtime);
        }

        self.load_keys_from_env(&provider, &config, env_prefix).await;
    }

    pub async fn acquire(&self, provider: &Provider) -> Option<ProviderGuard> {
        loop {
            // Get the mutex specific to that provider using Read lock (without blocking others)
            let runtime_mutex = {
                let map = self.inner.read().await;
                map.get(provider).cloned()?
            };
            
            // Lock only the current provider's runtime
            let mut runtime: tokio::sync::MutexGuard<'_, ProviderRuntime> = runtime_mutex.lock().await;

            let client = runtime.client.clone();
            let state = &mut runtime.state;
            
            // #[cfg(debug_assertions)]
            // println!("{:#?}", state);

            if state.circuit.is_open {
                let elapsed = Self::now_day() - state.circuit.last_failure_time;

                if elapsed < 60 {
                    return None;
                }

                state.circuit.is_open = false;
                state.circuit.failure_count = 0;
            }

            if state.running >= state.maximum {
                drop(runtime);

                tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                continue;
            }

            let now = Self::now_day();
            let today = Self::current_day();

            let max_attempts = state.keys.len();
            if max_attempts == 0 {
                return None;
            }

            for _ in 0..max_attempts {
                let idx = state.next_index % max_attempts;
                
                state.next_index += 1;
                if state.next_index >= max_attempts {
                    state.next_index = 0;
                }

                let key = &mut state.keys[idx];

                if key.last_reset_day != today {
                    key.daily_used = 0;
                    key.last_reset_day = today;
                    key.blocked_until = 0;
                }

                if key.blocked_until > now {
                    continue;
                }

                if let Some(daily_limit) = key.daily_limit {
                    if key.daily_used >= daily_limit {
                        continue;
                    }
                }
                
                let credential = key.credential.clone();

                key.daily_used += 1;
                state.running += 1;

                println!(
                    "[{}] acquired key: {} | used: {} | running: {} | maximun: {}",
                    provider.clone().to_string(),
                    key.credential.clone().api_key.to_string(),
                    key.daily_used.clone().to_string(),
                    state.running.clone().to_string(),
                    state.maximum.clone().to_string(),
                );
                return Some(ProviderGuard {
                    credential,
                    provider: provider.clone(),
                    client: client.clone(),
                    manager: self.clone(),
                });
            }

            // Release the lock before repeating/waiting
            drop(runtime);

            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        };
    }

    async fn release(&self, provider: &Provider) {
        let runtime_mutex = {
            let map = self.inner.read().await;
            map.get(provider).cloned()
        };

        if let Some(mutex) = runtime_mutex {
            let mut runtime = mutex.lock().await;

            runtime.state.running = runtime.state.running.saturating_sub(1);
            runtime.state.circuit.failure_count = 0;
            runtime.state.circuit.is_open = false;
        }
    }
    
    async fn block_key(
        &self,
        provider: &Provider,
        api_key: &str,
        seconds: u64
    ) {
        let runtime_mutex = {
            let map = self.inner.read().await;
            map.get(provider).cloned()
        };

        if let Some(mutex) = runtime_mutex {
            let mut runtime = mutex.lock().await;

            let now = Self::now_day();
            
            if let Some(key) = runtime.state.keys.iter_mut().find(|k| k.credential.api_key == api_key) {
                key.blocked_until = now + seconds;
            }
        }
    }

    async fn block_until_tomorrow(
        &self,
        provider: Provider,
        api_key: &str,
    ) {
        let runtime_mutex = {
            let map = self.inner.read().await;
            map.get(&provider).cloned()
        };

        if let Some(mutex) = runtime_mutex {
            let mut runtime = mutex.lock().await;
            
            if let Some(k) = runtime.state.keys.iter_mut().find(|k| k.credential.api_key == api_key) {
                k.blocked_until = Self::next_day_timestamp();
            }
        }
    }

    pub async fn add_key(
        &self,
        provider: &Provider,
        config: &ProviderConfig,
        credential: ApiCredential,
    ) {
        let runtime_mutex = {
            let map = self.inner.read().await;
            map.get(provider).cloned()
        };

        if let Some(mutex) = runtime_mutex {
            let mut runtime = mutex.lock().await;
            runtime.state.keys.push(ApiKeyState {
                credential,
                daily_used: 0,
                daily_limit: config.daily_limit,
                blocked_until: 0,
                last_reset_day: Self::current_day(),
            });
        }
    }
    
    async fn count_key(
        &self,
        provider: &Provider,
    ) -> usize {
        let map = self.inner.read().await;
        if let Some(mutex) = map.get(provider) {
            let runtime = mutex.lock().await;
            runtime.state.keys.len()
        } else {
            0
        }
    }

    async fn next_key(
        &self,
        provider: &Provider,
    ) -> Option<ApiCredential> {
        let runtime_mutex = {
            let map = self.inner.read().await;
            map.get(provider).cloned()?
        };

        let mut runtime = runtime_mutex.lock().await;
        let now = Self::now_day();
        let today = Self::current_day();
        let total = runtime.state.keys.len();

        for _ in 0..total {
            let idx = runtime.state.next_index % total;

            runtime.state.next_index += 1;
            if runtime.state.next_index >= total {
                runtime.state.next_index = 0;
            }

            let key = &mut runtime.state.keys[idx];

            if key.last_reset_day != today {
                key.daily_used = 0;
                key.blocked_until = 0;
                key.last_reset_day = today;
            }

            if key.blocked_until > now {
                continue;
            }

            if let Some(daily_limit) = key.daily_limit {
                if key.daily_used >= daily_limit {
                    continue;
                }
            }

            key.daily_used += 1;
            
            return Some(key.credential.clone());
        }

        None
    }

    async fn record_failure(&self, provider: &Provider) {
        let runtime_mutex = {
            let map = self.inner.read().await;
            map.get(provider).cloned()
        };

        if let Some(mutex) = runtime_mutex {
            let mut runtime = mutex.lock().await;
            
            let circuit = &mut runtime.state.circuit;
            
            circuit.failure_count += 1;
            circuit.last_failure_time = Self::now_day();

            if circuit.failure_count >= 5 {
                circuit.is_open = true;
            }
        }
    }

    async fn load_keys_from_env(
        &self,
        provider: &Provider,
        config: &ProviderConfig,
        prefix: &str,
    ) {
        let mut i = 1;
        loop {
            let key_name = format!("{}_KEY_{}", prefix, i);
            let Ok(api_key) = std::env::var(&key_name) else {
                break;
            };

            let account_name = format!("{}_ACCOUNT_{}", prefix, i);
            let account_id = std::env::var(&account_name).ok();

            self.add_key(
                provider,
                config,
                ApiCredential { api_key, account_id },
            ).await;

            i += 1;
        }
    }

    fn now_day() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
    }

    fn current_day() -> u64 {
        Self::now_day() / 86400
    }
    
    fn next_day_timestamp() -> u64 {
        let now = Self::now_day();

        ((now / 86400) + 1) * 86400
    }
}

impl Drop for ProviderGuard {
    fn drop(&mut self) {
        let provider = self.provider.clone();
        let manager = self.manager.clone();

        tokio::spawn(async move {
            manager.release(&provider).await;
        });
    }
}

impl ProviderGuard {
    pub async fn call(
        self,
        req: ProviderRequest,
    ) -> Result<ProviderResponse, ProviderError> {
        let max_keys = self.manager
            .count_key(&self.provider)
            .await;

        for _ in 0..max_keys {

            let credential = match self.manager.next_key(&self.provider).await {
                Some(v) => v,
                None => break,
            };

            match self.client.call(&req, &credential).await {
                Ok(v) => {
                    return Ok(v)
                },
                Err(ProviderError::RequestExceeded { retry_after, .. }) => {
                    self.manager.block_key(&self.provider, &credential.api_key, retry_after).await;
                    continue;
                }
                Err(e) => {
                    self.manager.record_failure(&self.provider).await;
                    return Err(e)
                },
            }
        }

        Err(ProviderError::AllKeysExhausted(self.provider.clone().to_string()))
    }

    fn backoff(attempt: u32) -> std::time::Duration {
        std::time::Duration::from_millis(
            (100u64 * 2u64.pow(attempt)).min(30_000)
        )
    }
}

impl ProviderResponse {
    pub fn into_text(self) -> Result<String, ProviderError> {
        match self {
            ProviderResponse::Text(t) => Ok(t),
            ProviderResponse::Bytes(_) => Err(ProviderError::UnexpectedResponse),
        }
    }

    pub fn into_bytes(self) -> Result<Vec<u8>, ProviderError> {
        match self {
            ProviderResponse::Bytes(b) => Ok(b),
            ProviderResponse::Text(_) => Err(ProviderError::UnexpectedResponse),
        }
    }
}