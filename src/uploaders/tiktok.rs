use crate::uploader::*;

pub struct TiktokUploader {
    access_token: String,
}

#[async_trait::async_trait]
impl Uploader for TiktokUploader {
    fn channel(&self) -> Channel {
        Channel::Tiktok
    }

    async fn upload(
        &self,
        video_path: &str,
        title: &str,
        config: ChannelConfig
    ) -> Result<(), String> {
        // Logic upload YouTube của bạn ở đây...
        Ok(())
    }
}

impl TiktokUploader {
    pub fn new() -> Self {
        Self {
            access_token: std::env::var("TIKTOK_ACCESS_TOKEN").unwrap_or_default(),
        }
    }
}