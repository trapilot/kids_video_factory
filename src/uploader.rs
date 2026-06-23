use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use strum_macros::{Display, EnumString};

use crate::db::DbManager;


#[derive(Debug, Clone, Display, EnumString, Serialize, Deserialize, Eq, PartialEq, Hash)]
#[strum(serialize_all = "lowercase")]
#[serde(rename_all = "lowercase")]

pub enum Channel {
    Youtube,
    Tiktok,
}

#[derive(Debug, Default, Serialize, Deserialize, Clone)]
pub struct UploaderError {
    pub youtube: Option<String>,
    pub tiktok: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UploaderState {
    pub all_uploaded: bool,
    pub any_uploaded: bool,
    pub errors: UploaderError
}

#[derive(Clone)]
pub enum ChannelConfig {
    Youtube {
        category_id: String,
        description: String,
        tags: Vec<String>,
    },
    Tiktok {
        privacy_level: String,
        disable_comment: bool,
    },
    Facebook {
        is_scheduled: bool,
    },
}

#[async_trait]
pub trait Uploader: Send + Sync {
    fn channel(&self) -> Channel;

    async fn upload(
        &self,
        db: &Arc<DbManager>,
        video_path: &str,
        title: &str,
        config: ChannelConfig
    ) -> Result<(), String>;
}
pub struct UploaderManager {
    inner: Arc<Mutex<HashMap<Channel, Arc<dyn Uploader>>>>,
}

impl UploaderManager {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn register(&self, uploader: Arc<dyn Uploader>) {
        let mut map = self.inner.lock().await;
        map.insert(uploader.channel(), uploader);
    }

    pub async fn upload_all(
        &self,
        db: &Arc<DbManager>,
        path: &str,
        title: &str,
        configs: HashMap<Channel, ChannelConfig>
    ) -> Vec<(Channel, Result<(), String>)> {
        let map = self.inner.lock().await;
        let mut tasks = Vec::new();

        for (channel, config) in configs {
            if let Some(uploader) = map.get(&channel) {
                let up = uploader.clone();
                let db = db.clone();
                let p = path.to_string();
                let t = title.to_string();
                
                tasks.push(tokio::spawn(async move {
                    (channel, up.upload(&db, &p, &t, config).await)
                }));
            }
        }

        futures::future::join_all(tasks)
            .await
            .into_iter()
            .filter_map(Result::ok) // Bỏ qua lỗi spawn task
            .collect()
    }
}