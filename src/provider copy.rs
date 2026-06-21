use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use std::collections::HashMap;
use tokio::sync::{Semaphore, Mutex};

use crate::enums::*;


#[derive(Clone)]
pub struct SemaphoreManager {
    inner: Arc<Mutex<HashMap<Provider, Arc<Semaphore>>>>,
}

#[derive(Clone)]
pub struct ProviderManager {
    pub inner: Arc<Mutex<HashMap<Provider, ProviderState>>>,
}

pub struct ProviderGuard {
    pub provider: Provider,
    pub credential: ApiCredential,
    pub manager: ProviderManager,
}

#[derive(Clone)]
pub struct ApiCredential {
    pub api_key: String,
    pub account_id: Option<String>,
}

#[derive(Clone)]
pub struct ApiKeyState {
    pub credential: ApiCredential,
    pub daily_used: u32,
    pub daily_limit: u32,
    pub blocked_until: u64,
    pub last_reset_day: u64,
}

#[derive(Clone)]
pub struct ProviderState {
    pub running: u32,
    pub maximum: u32,
    pub next_index: usize,
    pub keys: Vec<ApiKeyState>,
}

impl SemaphoreManager {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn acquire(&self, provider: Provider) -> Arc<Semaphore> {
        let map = self.inner.lock().await;
        
        map.get(&provider)
            .expect("Semaphore not found for provider")
            .clone()
    }

    pub async fn insert(&self, provider: Provider, concurrency: u32) {
        let mut map = self.inner.lock().await;

        map.insert(
            provider,
            Arc::new(Semaphore::new(concurrency as usize)),
        );
    }

    pub async fn update(&self, provider: Provider, concurrency: u32) {
        let mut map = self.inner.lock().await;

        map.insert(
            provider,
            Arc::new(Semaphore::new(concurrency as usize)),
        );
    }

    pub async fn get_or_create(
        &self,
        provider: Provider,
        concurrency: u32,
    ) -> Arc<Semaphore> {
        let mut map = self.inner.lock().await;

        if let Some(sem) = map.get(&provider) {
            return sem.clone();
        }

        let sem = Arc::new(Semaphore::new(concurrency as usize));

        map.insert(provider, sem.clone());

        sem
    }
}

impl ProviderManager {
    pub fn new() -> Self {
        let mut map = HashMap::new();

        map.insert(
            Provider::Gemini,
            ProviderState {
                running: 0,
                maximum: 1,
                next_index: 0,
                keys: vec![],
            }
        );

        map.insert(
            Provider::Cloudflare,
            ProviderState {
                running: 0,
                maximum: 1,
                next_index: 0,
                keys: vec![],
            }
        );

        map.insert(
            Provider::ElevenLabs,
            ProviderState {
                running: 0,
                maximum: 1,
                next_index: 0,
                keys: vec![],
            }
        );

        Self {
            inner: Arc::new(Mutex::new(map)),
        }
    }

    pub async fn acquire(&self, provider: Provider) -> Option<ProviderGuard> {
        loop {
            let mut map = self.inner.lock().await;
            
            let state = map.get_mut(&provider)?;

            if state.running >= state.maximum {
                drop(map);

                tokio::time::sleep(
                    std::time::Duration::from_millis(100),
                )
                .await;

                continue;
            }

            let key_count = state.keys.len();

            if key_count == 0 {
                return None;
            }
            
            let now = Self::now();
            let today = Self::current_day();
            
            for _ in 0..key_count {
                let idx = state.next_index % key_count;
                state.next_index += 1;

                let key_state = &mut state.keys[idx];

                // Reset daily
                if key_state.last_reset_day != today {
                    key_state.daily_used = 0;
                    key_state.last_reset_day = today;
                    key_state.blocked_until = 0;
                }

                // The key is blocked
                if key_state.blocked_until > now {
                    continue;
                }

                // Quota for the day has expired
                if key_state.daily_used >= key_state.daily_limit {
                    continue;
                }

                key_state.daily_used += 1;

                let credential = key_state.credential.clone();

                state.running += 1;

                return Some(ProviderGuard {
                    provider: provider.clone(),
                    credential,
                    manager: self.clone(),
                });

            }

            drop(map);

            tokio::time::sleep(
                std::time::Duration::from_secs(1),
            )
            .await;
        }
    }

    pub async fn release(&self, provider: Provider) {
        let mut map = self.inner.lock().await;

        if let Some(state) = map.get_mut(&provider) {
            if state.running > 0 {
                state.running -= 1;
            }
        }
    }

   pub async fn block_key(
        &self,
        provider: Provider,
        api_key: &str,
        seconds: u64,
    ) {
        let mut map = self.inner.lock().await;

        if let Some(state) = map.get_mut(&provider) {
            let now = Self::now();

            if let Some(k) = state
                .keys
                .iter_mut()
                .find(|k| k.credential.api_key == api_key)
            {
                k.blocked_until = now + seconds;
            }
        }
    }

    pub async fn block_until_tomorrow(
        &self,
        provider: Provider,
        api_key: &str,
    ) {
        let mut map = self.inner.lock().await;

        let Some(state) = map.get_mut(&provider) else {
            return;
        };

        if let Some(k) = state
            .keys
            .iter_mut()
            .find(|k| k.credential.api_key == api_key)
        {
            k.blocked_until = Self::next_day_timestamp();
        }
    }

    pub async fn add_keys(
        &self,
        provider: Provider,
        credentials: Vec<ApiCredential>,
        daily_limit: u32,
    ) {
        for credential in credentials {
            self.add_key(
                provider.clone(),
                credential,
                daily_limit,
            )
            .await;
        }
    }

    pub async fn add_key(
        &self,
        provider: Provider,
        credential: ApiCredential,
        daily_limit: u32,
    ) {
        let mut map = self.inner.lock().await;

        if let Some(state) = map.get_mut(&provider) {
            state.keys.push(ApiKeyState {
                credential,
                daily_used: 0,
                daily_limit,
                blocked_until: 0,
                last_reset_day: Self::current_day(),
            });
        }
    }

    pub async fn key_count(
        &self,
        provider: Provider,
    ) -> usize {
        let map = self.inner.lock().await;

        map.get(&provider)
            .map(|state| state.keys.len())
            .unwrap_or(0)
    }

    pub async fn quota_used(
        &self,
        provider: Provider,
        api_key: &str,
    ) -> Option<u32> {
        let map = self.inner.lock().await;

        let state = map.get(&provider)?;

        let key = state.keys
            .iter()
            .find(|k| k.credential.api_key == api_key)?;

        Some(key.daily_used)
    }

    pub async fn quota_remaining(
        &self,
        provider: Provider,
        api_key: &str,
    ) -> Option<u32> {
        let map = self.inner.lock().await;

        let state = map.get(&provider)?;

        let key = state.keys
            .iter()
            .find(|k| k.credential.api_key == api_key)?;

        Some(key.daily_limit.saturating_sub(key.daily_used))
    }

    pub async fn load_keys_from_env(
        &self,
        provider: Provider,
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
                provider.clone(),
                ApiCredential {
                    api_key,
                    account_id,
                },
                daily_limit,
            )
            .await;

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

impl Drop for ProviderGuard {
    fn drop(&mut self) {
        let provider = self.provider.clone();
        let manager = self.manager.clone();

        tokio::spawn(async move {
            manager.release(provider).await;
        });
    }
}
