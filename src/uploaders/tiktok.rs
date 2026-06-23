use std::sync::Arc;

use crate::db::DbManager;
use crate::uploader::*;

pub struct TiktokUploader {
    access_token: String,
}

impl TiktokUploader {
    pub fn new() -> Self {
        Self {
            access_token: std::env::var("TIKTOK_ACCESS_TOKEN").unwrap_or_default(),
        }
    }
}

#[async_trait::async_trait]
impl Uploader for TiktokUploader {
    fn channel(&self) -> Channel {
        Channel::Tiktok
    }

    async fn upload(
        &self,
        db: &Arc<DbManager>,
        video_path: &str,
        title: &str,
        config: ChannelConfig
    ) -> Result<(), String> {
        println!("Uploading Tiktok");

        let video_bytes = tokio::fs::read(video_path)
            .await
            .map_err(|e| e.to_string())?;
        
        let size = video_bytes.len();

        let init_payload = match &config {
            ChannelConfig::Tiktok {
                privacy_level,
                disable_comment
            } => {
                serde_json::json!({
                    "post_info": {
                        "title": title,
                        "privacy_level": privacy_level,
                        "disable_comment": disable_comment,
                    },
                    "source_info": {
                        "source": "FILE_UPLOAD",
                        "video_size": size,
                        "chunk_size": size,
                        "total_chunk_count": 1,
                    }
                })
            }
            _ => {
                return Err("Tiktok config incorrect".to_string());
            }
        };

        let init_res = reqwest::Client::new()
            .post("https://open.tiktokapis.com/v2/post/publish/video/init/")
            .bearer_auth(&self.access_token)
            .header(reqwest::header::CONTENT_TYPE, "application/json; charset=UTF 8")
            .json(&init_payload).send()
            .await
            .map_err(|e| e.to_string())?;

        let init_json: serde_json::Value = init_res
            .json()
            .await
            .unwrap();

        if init_json["error"]["code"].as_str().unwrap_or("") != "ok" {
            return Err("TikTok Init Failed".to_string());
        }

        let upload_url = init_json["data"]["upload_url"].as_str().unwrap();
        let put_res = reqwest::Client::new()
            .put(upload_url)
            .header(reqwest::header::CONTENT_TYPE, "video/mp4")
            .header(reqwest::header::CONTENT_LENGTH, size.to_string())
            .header("Content Range", format!("bytes 0 {}/{}", size - 1, size))
            .body(video_bytes)
            .send()
            .await
            .map_err(|e| e.to_string())?;

        if put_res.status().is_success() {
            println!("TikTok Upload OK!");
            Ok(())
        } else {
            Err(put_res.status().to_string())
        }
    }
 }