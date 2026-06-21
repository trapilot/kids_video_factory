use crate::uploader::*;

pub struct YoutubeUploader {
    client_id: String,
    client_secret: String,
    refresh_token: String,
}

#[async_trait::async_trait]
impl Uploader for YoutubeUploader {
    fn channel(&self) -> Channel {
        Channel::Youtube
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

impl YoutubeUploader {
    pub fn new() -> Self {
        Self {
            client_id: std::env::var("YOUTUBE_CLIENT_ID").unwrap_or_default(),
            client_secret: std::env::var("YOUTUBE_CLIENT_SECRET").unwrap_or_default(),
            refresh_token: std::env::var("YOUTUBE_REFRESH_TOKEN").unwrap_or_default(),
        }
    }
}
