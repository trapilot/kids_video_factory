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
    #[error("Provider http error {status}: {body}")]
    Http { status: u16, body: String },

    #[error("Provider quota exceeded retry after {retry_after}: {body}")]
    QuotaExceeded { retry_after: u64, body: String },

    #[error("Provider {0} invalid api keys")]
    InvalidApiKey(String),

    #[error("Provider {0} invalid account")]
    InvalidAccount(String),

    #[error("Provider {0} invalid response")]
    InvalidResponse(String),
    
    #[error("Provider {0} not defined")]
    NotFound(String),

    #[error("Provider {0} not support response type")]
    NotSupported(String),

    #[error("Provider network error: {0}")]
    Network(String),

    #[error("Provider network timeout")]
    Timeout,

    #[error("Provider unexpected response")]
    UnexpectedResponse,
}

#[derive(Clone)]
pub struct ApiCredential {
    pub api_key: String,
    pub account_id: Option<String>,
}

pub struct ApiKeyState {
    pub credential: ApiCredential,

    pub daily_used: u32,
    pub daily_limit: u32,

    pub blocked_until: u64,
    pub last_reset_day: u64,
}

pub struct CircuitBreaker {
    pub failure_count: u32,
    pub last_failure_time: u64,
    pub is_open: bool,
}

pub struct ProviderState {
    pub running: u32,
    pub maximum: u32,

    pub next_index: usize,
    pub keys: Vec<ApiKeyState>,

    pub circuit: CircuitBreaker,
}

pub struct ProviderRuntime {
    pub client: Arc<dyn ProviderClient>,
    pub state: ProviderState,
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

    async fn call(
        &self,
        req: &ProviderRequest,
        credential: &ApiCredential,
    ) -> Result<ProviderResponse, ProviderError>;

    fn is_quota_error(&self, status: u16, body: &str) -> bool;
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
        daily_limit: u32,
    ) {
        let provider = client.provider();
        let runtime = Arc::new(Mutex::new(ProviderRuntime {
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

        self.load_keys_from_env(&provider, env_prefix, daily_limit).await;
    }

    pub async fn acquire(&self, provider: &Provider) -> Option<ProviderGuard> {
        println!("[ACQUIRE] Start the acquiring for Provider: **{}**", provider);

        loop {
            // // Get the mutex specific to that provider using Read lock (without blocking others)
            let runtime_mutex = {
                let map = self.inner.read().await;
                map.get(provider).cloned()?
            };
            
            // Lock only the current provider's runtime
            // println!("[LOCK-WAIT] Currently waiting for **{}** release...", provider);
            let mut runtime: tokio::sync::MutexGuard<'_, ProviderRuntime> = runtime_mutex.lock().await;
            // println!("[LOCK-SUCCESS] Successfully acquired the key. (Mutex) of **{}**", provider);

            let client = runtime.client.clone();
            let state = &mut runtime.state;

            if state.circuit.is_open {
                let elapsed = Self::now() - state.circuit.last_failure_time;

                if elapsed < 60 {
                    // println!("[CIRCUIT-OPEN] Provider **{}** The circuit is currently disconnected. (Circuit Breaker open). Reject the request.", provider);
                    return None;
                }

                // println!("[CIRCUIT-RETRY] ​​Try closing the circuit of **{}** after 60s...", provider);
                state.circuit.is_open = false;
                state.circuit.failure_count = 0;
            }

            if state.running >= state.maximum {
                // println!(
                //     "[THROTTLE] Provider **{}** has reached its maximum limit (Number of tasks: {}/{}). Will try again after 50ms...",
                //     provider, state.running, state.maximum
                // );
                drop(runtime);

                tokio::time::sleep(std::time::Duration::from_millis(50)).await;
                continue;
            }

            let now = Self::now();
            let today = Self::current_day();

            let max_attempts = state.keys.len();
            if max_attempts == 0 {
                println!("[ERROR] Provider **{}** has no API Key registered!", provider);
                return None;
            }

            for _ in 0..max_attempts {
                let idx = state.next_index % max_attempts;
                
                state.next_index += 1;

                let key = &mut state.keys[idx];

                if key.last_reset_day != today {
                    key.daily_used = 0;
                    key.last_reset_day = today;
                    key.blocked_until = 0;
                }

                if key.blocked_until > now {
                    continue;
                }

                if key.daily_used >= key.daily_limit {
                    continue;
                }

                key.daily_used += 1;
                state.running += 1;

                println!(
                    "[SUCCESS] Suspension granted to **{}**! (Number of concurrently running tasks: {})",
                    provider, state.running
                );
                return Some(ProviderGuard {
                    provider: provider.clone(),
                    credential: key.credential.clone(),
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

            let now = Self::now();
            
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

    async fn get_client(
        &self,
        provider: &Provider,
    ) -> Option<Arc<dyn ProviderClient>> {
        let map = self.inner.read().await;
        let mutex = map.get(provider)?;
        let runtime = mutex.lock().await;
        Some(runtime.client.clone())
    }

    pub async fn add_keys(
        &self,
        provider: &Provider,
        credentials: Vec<ApiCredential>,
        daily_limit: u32,
    ) {
        for credential in credentials {
            self.add_key(provider, credential, daily_limit).await;
        }
    }

    pub async fn add_key(
        &self,
        provider: &Provider,
        credential: ApiCredential,
        daily_limit: u32,
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
                daily_limit,
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
        let now = Self::now();
        let today = Self::current_day();
        let total = runtime.state.keys.len();

        for _ in 0..total {
            let idx = runtime.state.next_index % total;

            runtime.state.next_index += 1;

            let key = &mut runtime.state.keys[idx];

            if key.last_reset_day != today {
                key.daily_used = 0;
                key.blocked_until = 0;
                key.last_reset_day = today;
            }

            if key.blocked_until > now {
                continue;
            }

            if key.daily_used >= key.daily_limit {
                continue;
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
            circuit.last_failure_time = Self::now();

            if circuit.failure_count >= 5 {
                circuit.is_open = true;
            }
        }
    }

    async fn reset_failures(&self, provider: &Provider) {
        let runtime_mutex = {
            let map = self.inner.read().await;
            map.get(provider).cloned()
        };

        if let Some(mutex) = runtime_mutex {
            let mut runtime = mutex.lock().await;

            runtime.state.circuit.failure_count = 0;
            runtime.state.circuit.is_open = false;
        }
    }

    async fn load_keys_from_env(
        &self,
        provider: &Provider,
        prefix: &str,
        daily_limit: u32,
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
                ApiCredential { api_key, account_id },
                daily_limit,
            ).await;

            i += 1;
        }
    }

    fn now() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
    }

    fn current_day() -> u64 {
        Self::now() / 86400
    }
    
    fn next_day_timestamp() -> u64 {
        let now = Self::now();

        ((now / 86400) + 1) * 86400
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
                Ok(v) => return Ok(v),
                Err(ProviderError::QuotaExceeded { retry_after, .. }) => {
                    self.manager.block_key(&self.provider, &credential.api_key, retry_after).await;
                    continue;
                }
                Err(e) => return Err(e),
            }
        }

        Err(ProviderError::InvalidApiKey("all keys exhausted".into()))
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